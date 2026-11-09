//! Serial port communication implementation
//!
//! Provides low-level serial port operations for direct hardware connection
//! to CNC controllers via USB or RS-232.
//!
//! Supports:
//! - Port enumeration and discovery
//! - Baud rate configuration
//! - Flow control settings
//! - Parity and stop bit configuration
//! - Blocking read/write operations

use crate::{ConnectionDriver, ConnectionParams, SerialParity};
use gcodekit5_core::{Error, Result};
use std::io::{self, Read, Write};
use std::sync::Mutex;
use std::time::Duration;

/// Result type for serial operations
pub type SerialResult<T> = std::result::Result<T, SerialPortError>;

/// Serial port specific errors
#[derive(Debug, Clone)]
pub struct SerialPortError {
    message: String,
}

impl std::fmt::Display for SerialPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Serial port error: {}", self.message)
    }
}

impl std::error::Error for SerialPortError {}

impl SerialPortError {
    /// Create a new serial port error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Information about an available serial port
#[derive(Debug, Clone)]
pub struct SerialPortInfo {
    /// Port name (e.g., "/dev/ttyUSB0", "COM3")
    pub port_name: String,

    /// Port description (e.g., "USB Serial Port")
    pub description: String,

    /// Manufacturer name if available
    pub manufacturer: Option<String>,

    /// Serial number if available
    pub serial_number: Option<String>,

    /// USB vendor ID if applicable
    pub vid: Option<u16>,

    /// USB product ID if applicable
    pub pid: Option<u16>,
}

impl SerialPortInfo {
    /// Create a new port info
    pub fn new(port_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            description: description.into(),
            manufacturer: None,
            serial_number: None,
            vid: None,
            pid: None,
        }
    }

    /// Set manufacturer
    pub fn with_manufacturer(mut self, manufacturer: impl Into<String>) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self
    }

    /// Set serial number
    pub fn with_serial_number(mut self, serial_number: impl Into<String>) -> Self {
        self.serial_number = Some(serial_number.into());
        self
    }

    /// Set USB IDs
    pub fn with_usb_ids(mut self, vid: u16, pid: u16) -> Self {
        self.vid = Some(vid);
        self.pid = Some(pid);
        self
    }
}

/// List available serial ports on the system
///
/// Returns a list of available COM ports with information about each port.
/// Filters ports to controller-capable serial devices (see [`is_valid_cnc_port_name`]),
/// including on-board UARTs (`/dev/ttyS*`, `/dev/ttyAMA*`) and Bluetooth serial
/// (`/dev/rfcomm*`), while hiding unpopulated `/dev/ttyS*` placeholders.
pub fn list_ports() -> Result<Vec<SerialPortInfo>> {
    match serialport::available_ports() {
        Ok(ports) => {
            let mut port_infos: Vec<SerialPortInfo> = ports
                .iter()
                .filter(|port| is_valid_cnc_port(&port.port_name))
                .map(|port| {
                    let info = SerialPortInfo::new(&port.port_name, get_port_description(port));

                    match &port.port_type {
                        serialport::SerialPortType::UsbPort(usb_info) => {
                            let mut info = info.with_usb_ids(usb_info.vid, usb_info.pid);
                            if let Some(ref mfg) = usb_info.manufacturer {
                                info = info.with_manufacturer(mfg);
                            }
                            if let Some(ref serial) = usb_info.serial_number {
                                info = info.with_serial_number(serial);
                            }
                            info
                        }
                        _ => info,
                    }
                })
                .collect();

            // Check for virtual/simulator serial ports (PTY devices)
            // These are not detected by serialport::available_ports() since they're not hardware
            check_virtual_ports(&mut port_infos);

            Ok(port_infos)
        }
        Err(e) => {
            tracing::error!("Failed to enumerate serial ports: {}", e);
            Err(Error::other(format!("Failed to enumerate ports: {}", e)))
        }
    }
}

/// Check whether a port name looks like a serial device a CNC controller could be on.
///
/// Accepted patterns:
/// - Windows: `COM<n>`
/// - Linux: `/dev/ttyUSB*`, `/dev/ttyACM*`, `/dev/ttyGRBL` (simulator),
///   `/dev/ttyS*` (on-board UARTs), `/dev/ttyAMA*` (Raspberry Pi PL011 UART),
///   `/dev/ttyTHS*` (Jetson), `/dev/ttymxc*` (i.MX), `/dev/ttyO*` (BeagleBone),
///   `/dev/rfcomm*` (Bluetooth serial)
/// - macOS: `/dev/cu.*` except the built-in Bluetooth-Incoming-Port and debug consoles
///
/// Obvious non-devices are rejected: virtual consoles (`/dev/tty<n>`), pseudo
/// terminals, `/dev/console`, macOS `tty.*` dial-in duplicates of `cu.*`, and
/// macOS debug/incoming-only ports. Note that on Linux `/dev/ttyS*` placeholders
/// with no UART behind them are filtered separately by [`list_ports`] via sysfs.
///
/// # Arguments
/// * `port_name` - The OS device name, e.g. `/dev/ttyAMA0` or `COM3`.
///
/// # Returns
/// `true` if the name matches a controller-capable serial device pattern.
pub fn is_valid_cnc_port_name(port_name: &str) -> bool {
    // Windows COM ports
    if let Some(num) = port_name.strip_prefix("COM") {
        return !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
    }

    // Linux: USB CDC/serial adapters, on-board and SoC UARTs, Bluetooth RFCOMM.
    const LINUX_PREFIXES: [&str; 8] = [
        "/dev/ttyUSB",
        "/dev/ttyACM",
        "/dev/ttyS",
        "/dev/ttyAMA",
        "/dev/ttyTHS",
        "/dev/ttymxc",
        "/dev/ttyO",
        "/dev/rfcomm",
    ];
    if port_name == "/dev/ttyGRBL" {
        return true;
    }
    for prefix in LINUX_PREFIXES {
        if let Some(rest) = port_name.strip_prefix(prefix) {
            return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
        }
    }

    // macOS: callout devices. Skip the always-present Bluetooth incoming port and
    // debug consoles, which are never a controller.
    if let Some(rest) = port_name.strip_prefix("/dev/cu.") {
        let lower = rest.to_ascii_lowercase();
        let is_non_device = lower.is_empty()
            || lower.starts_with("bluetooth-incoming-port")
            || lower.contains("debug-console")
            || lower.starts_with("wlan-debug");
        return !is_non_device;
    }

    false
}

/// On Linux, `/dev/ttyS0`..`ttyS31` exist even when no UART is fitted. The kernel
/// reports `type` 0 (`PORT_UNKNOWN`) in sysfs for those placeholders, so hide them.
/// Returns `true` (keep the port) whenever the information is unavailable.
fn uart_placeholder_present(port_name: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Some(dev) = port_name.strip_prefix("/dev/") {
            if dev.starts_with("ttyS") {
                let type_path = format!("/sys/class/tty/{dev}/type");
                if let Ok(t) = std::fs::read_to_string(type_path) {
                    return t.trim() != "0";
                }
            }
        }
    }
    let _ = port_name;
    true
}

/// Full filter used by [`list_ports`]: name pattern plus hardware presence.
fn is_valid_cnc_port(port_name: &str) -> bool {
    is_valid_cnc_port_name(port_name) && uart_placeholder_present(port_name)
}

/// Get a user-friendly description for a port
fn get_port_description(port: &serialport::SerialPortInfo) -> String {
    match &port.port_type {
        serialport::SerialPortType::UsbPort(usb_info) => {
            format!(
                "USB {} {}",
                usb_info.manufacturer.as_deref().unwrap_or("Device"),
                usb_info.product.as_deref().unwrap_or("Serial Port")
            )
        }
        serialport::SerialPortType::BluetoothPort => "Bluetooth Serial".to_string(),
        serialport::SerialPortType::PciPort => "PCI Serial".to_string(),
        _ => "Serial Port".to_string(),
    }
}

/// Check for virtual/simulator serial ports and add them to the list if they exist
///
/// Virtual ports (PTY devices) are not detected by serialport::available_ports()
/// because they're not hardware devices. This function manually checks for known
/// simulator ports:
/// - Linux: /dev/ttyGRBL (grblHAL simulator)
fn check_virtual_ports(port_infos: &mut Vec<SerialPortInfo>) {
    #[cfg(target_os = "linux")]
    {
        let virtual_ports = [("/dev/ttyGRBL", "grblHAL Simulator (Virtual Port)")];

        for (port_name, description) in &virtual_ports {
            // Check if this port already exists in the list
            if port_infos.iter().any(|p| p.port_name == *port_name) {
                continue;
            }

            // Check if the port exists and is accessible
            if std::path::Path::new(port_name).exists() {
                tracing::info!("Found virtual serial port: {}", port_name);
                port_infos.push(SerialPortInfo::new(*port_name, *description));
            }
        }
    }
}

/// Convert a parity setting to serialport format
fn to_serialport_parity(parity: SerialParity) -> serialport::Parity {
    match parity {
        SerialParity::None => serialport::Parity::None,
        SerialParity::Even => serialport::Parity::Even,
        SerialParity::Odd => serialport::Parity::Odd,
    }
}

/// Low-level serial port interface
pub trait SerialPort: Send + Sync {
    /// Write data to the port
    fn write(&mut self, data: &[u8]) -> io::Result<usize>;

    /// Read data from the port
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Get the port name
    fn name(&self) -> String;

    /// Close the port
    fn close(&mut self) -> io::Result<()>;
}

/// Trait for serial port I/O operations
pub trait ReadWrite: std::io::Read + std::io::Write + Send {}
impl<T: std::io::Read + std::io::Write + Send> ReadWrite for T {}

/// Real serial port implementation using serialport crate
pub struct RealSerialPort {
    port: Mutex<Box<dyn ReadWrite>>,
}

impl RealSerialPort {
    /// Open a serial port with the given parameters
    pub fn open(params: &ConnectionParams) -> Result<Self> {
        if params.driver != ConnectionDriver::Serial {
            return Err(Error::other("RealSerialPort requires Serial driver type"));
        }

        let builder = serialport::new(&params.port, params.baud_rate)
            .timeout(Duration::from_millis(10)) // Short timeout for non-blocking reads (Modify at 1 for Ray5)
            .data_bits(match params.data_bits {
                5 => serialport::DataBits::Five,
                6 => serialport::DataBits::Six,
                7 => serialport::DataBits::Seven,
                8 => serialport::DataBits::Eight,
                _ => {
                    return Err(Error::other(format!(
                        "Invalid data bits: {}",
                        params.data_bits
                    )))
                }
            })
            .stop_bits(match params.stop_bits {
                1 => serialport::StopBits::One,
                2 => serialport::StopBits::Two,
                _ => {
                    return Err(Error::other(format!(
                        "Invalid stop bits: {}",
                        params.stop_bits
                    )))
                }
            })
            .parity(to_serialport_parity(params.parity))
            .flow_control(if params.flow_control {
                serialport::FlowControl::Hardware
            } else {
                serialport::FlowControl::None
            });

        match builder.open_native() {
            Ok(port) => Ok(RealSerialPort {
                port: Mutex::new(Box::new(port)),
            }),
            Err(e) => {
                tracing::warn!("Failed to open serial port {}: {}", params.port, e);
                Err(Error::other(format!(
                    "Failed to open port {}: {}",
                    params.port, e
                )))
            }
        }
    }
}

impl SerialPort for RealSerialPort {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self.port.lock() {
            // write_all: a short write would silently drop the tail of a G-code line
            Ok(mut port) => port.write_all(data).map(|()| data.len()),
            Err(e) => Err(io::Error::other(e.to_string())),
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.port.lock() {
            Ok(mut port) => port.read(buf),
            Err(e) => Err(io::Error::other(e.to_string())),
        }
    }

    fn name(&self) -> String {
        "serial_port".to_string()
    }

    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}
