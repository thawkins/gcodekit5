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
/// Filters ports to include only CNC controller patterns:
/// - Windows: COM* (e.g., COM1, COM3)
/// - Linux: /dev/ttyUSB*, /dev/ttyACM*, /dev/ttyGRBL
/// - macOS: /dev/cu.usbserial-*, /dev/cu.usbmodem*
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

/// Check if a port name matches CNC controller patterns
///
/// Valid patterns:
/// - Windows: COM* (COM1, COM2, etc.)
/// - Linux: /dev/ttyUSB*, /dev/ttyACM*, /dev/ttyGRBL
/// - macOS: /dev/cu.usbserial-*, /dev/cu.usbmodem*
fn is_valid_cnc_port(port_name: &str) -> bool {
    // Windows COM ports
    if port_name.starts_with("COM") && port_name[3..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // Linux USB and ACM devices
    if port_name.starts_with("/dev/ttyUSB")
        || port_name.starts_with("/dev/ttyACM")
        || port_name.starts_with("/dev/ttyGRBL")
    {
        return true;
    }

    // macOS serial and modem devices
    if port_name.starts_with("/dev/cu.usbserial-") || port_name.starts_with("/dev/cu.usbmodem") {
        return true;
    }

    false
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
            .timeout(Duration::from_millis(1)) // Short timeout for non-blocking reads (Modify at 1 for Ray5)
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
            Ok(mut port) => port.write(data),
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
