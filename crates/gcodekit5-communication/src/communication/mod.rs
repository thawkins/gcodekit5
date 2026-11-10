//! gcodekit5-communication/src/communication/src/communication/mod.rs
//! Communication layer for serial, TCP, and WebSocket protocols
//!
//! Provides trait-based communication interface for different connection types.
//!
//! # Features
//! - Pluggable communication backends
//! - Serial (USB) communication
//! - TCP/IP network communication  
//! - WebSocket communication
//! - Event callbacks for connection state changes
//! - Configurable connection parameters

pub mod serial;
pub mod tcp;

use crate::firmware::repetier::processor::RepetierEvent;
use crate::firmware::ControllerProtocol;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

pub use serial::{list_ports, SerialPortInfo};
pub use tcp::TcpConnectionInfo;

/// Connection driver type
///
/// Specifies the underlying protocol for communication with the CNC controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionDriver {
    /// Serial port (USB/RS-232)
    Serial,
    /// TCP/IP network connection
    Tcp,
    /// WebSocket connection
    WebSocket,
}

impl fmt::Display for ConnectionDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serial => write!(f, "serial"),
            Self::Tcp => write!(f, "tcp"),
            Self::WebSocket => write!(f, "websocket"),
        }
    }
}

/// Connection parameters for establishing communication
///
/// Contains all information needed to establish a connection with a CNC controller.
/// The required fields depend on the connection driver type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionParams {
    /// Connection driver type
    pub driver: ConnectionDriver,

    /// Port name (e.g., "/dev/ttyUSB0" for serial, "192.168.1.100" for TCP)
    pub port: String,

    /// Port number (used for TCP/WebSocket connections, typically 8888 for g2core, 23 for GRBL)
    pub network_port: u16,

    /// Baud rate for serial connections (115200 typical for GRBL)
    pub baud_rate: u32,

    /// Connection timeout in milliseconds
    pub timeout_ms: u64,

    /// Whether to use RTS/CTS flow control (serial only)
    pub flow_control: bool,

    /// Number of data bits (serial only, typically 8)
    pub data_bits: u8,

    /// Number of stop bits (serial only, typically 1)
    pub stop_bits: u8,

    /// Parity setting (serial only)
    pub parity: SerialParity,

    /// Whether to enable automatic reconnection on disconnect
    pub auto_reconnect: bool,

    /// Maximum number of reconnection attempts before giving up
    pub max_retries: u32,
}

/// Serial port parity setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SerialParity {
    /// No parity bit
    None,
    /// Even parity
    Even,
    /// Odd parity
    Odd,
}

impl Default for ConnectionParams {
    fn default() -> Self {
        Self {
            driver: ConnectionDriver::Serial,
            port: "/dev/ttyUSB0".to_string(),
            network_port: 8888,
            baud_rate: 115200,
            timeout_ms: 5000,
            flow_control: false,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            auto_reconnect: true,
            max_retries: 3,
        }
    }
}

impl ConnectionParams {
    /// Create connection parameters for serial communication
    pub fn serial(port: &str, baud_rate: u32) -> Self {
        Self {
            driver: ConnectionDriver::Serial,
            port: port.to_string(),
            baud_rate,
            ..Default::default()
        }
    }

    /// Create connection parameters for TCP communication
    pub fn tcp(host: &str, port: u16) -> Self {
        Self {
            driver: ConnectionDriver::Tcp,
            port: host.to_string(),
            network_port: port,
            ..Default::default()
        }
    }

    /// Create connection parameters for WebSocket communication
    pub fn websocket(host: &str, port: u16) -> Self {
        Self {
            driver: ConnectionDriver::WebSocket,
            port: host.to_string(),
            network_port: port,
            ..Default::default()
        }
    }

    /// Validate the connection parameters
    pub fn validate(&self) -> gcodekit5_core::Result<()> {
        match self.driver {
            ConnectionDriver::Serial => {
                if self.port.is_empty() {
                    return Err(gcodekit5_core::Error::other(
                        "Serial port name cannot be empty",
                    ));
                }
                if self.baud_rate == 0 {
                    return Err(gcodekit5_core::Error::other("Baud rate must be > 0"));
                }
                if self.data_bits == 0 || self.data_bits > 8 {
                    return Err(gcodekit5_core::Error::other("Data bits must be 5-8"));
                }
            }
            ConnectionDriver::Tcp | ConnectionDriver::WebSocket => {
                if self.port.is_empty() {
                    return Err(gcodekit5_core::Error::other("Host/address cannot be empty"));
                }
                if self.network_port == 0 {
                    return Err(gcodekit5_core::Error::other("Network port must be > 0"));
                }
            }
        }

        if self.timeout_ms == 0 {
            return Err(gcodekit5_core::Error::other("Timeout must be > 0"));
        }

        Ok(())
    }
}

/// Communicator events for connection state changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicatorEvent {
    /// Connection established successfully
    Connected,
    /// Connection lost or disconnected
    Disconnected,
    /// Connection error occurred
    Error,
    /// Data received from device
    DataReceived,
    /// Data sent to device
    DataSent,
    /// Connection timeout
    Timeout,
}

impl fmt::Display for CommunicatorEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected => write!(f, "Connected"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Error => write!(f, "Error"),
            Self::DataReceived => write!(f, "DataReceived"),
            Self::DataSent => write!(f, "DataSent"),
            Self::Timeout => write!(f, "Timeout"),
        }
    }
}

/// Callback for communicator events
pub type CommunicatorCallback = Box<dyn Fn(CommunicatorEvent, &str) + Send + Sync>;

/// Trait for objects that listen to communicator events
pub trait CommunicatorListener: Send + Sync {
    /// Called when connection is established
    fn on_connected(&self);

    /// Called when connection is lost
    fn on_disconnected(&self);

    /// Called when a connection error occurs
    fn on_error(&self, error: &str);

    /// Called when data is received from the device
    fn on_data_received(&self, data: &[u8]);

    /// Called when data is sent to the device
    fn on_data_sent(&self, data: &[u8]);

    /// Called when a timeout occurs
    fn on_timeout(&self);
}

/// Arc-wrapped communicator listener for thread-safe sharing
pub type CommunicatorListenerHandle = Arc<dyn CommunicatorListener>;

/// Abstract communicator trait for pluggable communication backends
///
/// Provides a unified interface for different communication protocols
/// (Serial, TCP, WebSocket). Implementations handle protocol-specific
/// connection and data transmission details.
pub trait Communicator: Send + Sync {
    /// Connect to the device/server using the provided parameters
    fn connect(&mut self, params: &ConnectionParams) -> gcodekit5_core::Result<()>;

    /// Disconnect from the device/server
    fn disconnect(&mut self) -> gcodekit5_core::Result<()>;

    /// Check if currently connected
    fn is_connected(&self) -> bool;

    /// Send raw data to the device
    ///
    /// Returns the number of bytes sent
    fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize>;

    /// Receive data from the device
    ///
    /// Returns a vector of received bytes. May return empty vector if no data available.
    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>>;

    /// Send a text command with newline termination
    ///
    /// Convenience method that sends a command string followed by newline.
    fn send_command(&mut self, command: &str) -> gcodekit5_core::Result<()> {
        self.send(command.as_bytes())?;
        self.send(b"\n")?;
        Ok(())
    }

    /// Sends a command and attempts to receive a response.
    fn send_command_and_receive(&mut self, command: &str) -> gcodekit5_core::Result<Vec<u8>> {
        self.send_command(command)?;
        self.receive()
    }

    /// Add a listener for communicator events
    fn add_listener(&mut self, listener: CommunicatorListenerHandle);

    /// Remove a listener for communicator events
    fn remove_listener(&mut self, listener: &CommunicatorListenerHandle);

    /// Get connection parameters
    fn connection_params(&self) -> Option<&ConnectionParams>;

    /// Set connection parameters (without connecting)
    fn set_connection_params(&mut self, params: ConnectionParams) -> gcodekit5_core::Result<()>;

    /// Get the connection driver type
    fn driver_type(&self) -> ConnectionDriver {
        self.connection_params()
            .map(|p| p.driver)
            .unwrap_or(ConnectionDriver::Serial)
    }

    /// Get the port name for this connection
    fn port_name(&self) -> String {
        self.connection_params()
            .map(|p| p.port.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// No-op communicator for testing
pub struct NoOpCommunicator {
    connected: bool,
    params: Option<ConnectionParams>,
}

impl NoOpCommunicator {
    /// Create a new no-op communicator
    pub fn new() -> Self {
        Self {
            connected: false,
            params: None,
        }
    }
}

impl Default for NoOpCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Communicator for NoOpCommunicator {
    fn connect(&mut self, params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        self.params = Some(params.clone());
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
        if !self.connected {
            return Err(gcodekit5_core::Error::other("Not connected"));
        }
        Ok(data.len())
    }

    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> {
        if !self.connected {
            return Err(gcodekit5_core::Error::other("Not connected"));
        }
        Ok(vec![])
    }

    fn add_listener(&mut self, _listener: CommunicatorListenerHandle) {}

    fn remove_listener(&mut self, _listener: &CommunicatorListenerHandle) {}

    fn connection_params(&self) -> Option<&ConnectionParams> {
        self.params.as_ref()
    }

    fn set_connection_params(&mut self, params: ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        self.params = Some(params);
        Ok(())
    }
}

/// Serial/USB communicator for direct hardware connection
///
/// Provides full serial port communication using the serialport crate.
/// Supports configurable baud rates, parity, stop bits, and flow control.
pub struct SerialCommunicator {
    port: Option<Box<dyn serial::SerialPort>>,
    params: Option<ConnectionParams>,
    listeners: Vec<CommunicatorListenerHandle>,

    /// Protocol configured by the user via Device Manager
    ///
    /// **Source of truth** for the communicator's operating mode
    /// It is set using `set_configured_protocol()`
    pub configured_protocol: ControllerProtocol,

    /// Protocol detected via autodetect while reading incoming bytes
    pub detected_protocol: ControllerProtocol,

    pub repetier_processor: crate::firmware::repetier::processor::RepetierProcessor,
    repetier_events: Vec<RepetierEvent>,
    repetier_flow_paused: bool,
    repetier_flow_stopped: bool,
}

impl SerialCommunicator {
    /// Create a new serial communicator
    pub fn new() -> Self {
        Self {
            port: None,
            params: None,
            listeners: Vec::new(),
            configured_protocol: ControllerProtocol::Unknown,
            detected_protocol: ControllerProtocol::Unknown,
            repetier_processor: crate::firmware::repetier::processor::RepetierProcessor::new(),
            repetier_events: Vec::new(),
            repetier_flow_paused: false,
            repetier_flow_stopped: false,
        }
    }

    /// Creates a communicator already connected through `port`.
    ///
    /// For tests: lets a fake [`serial::SerialPort`] stand in for hardware.
    #[doc(hidden)]
    pub fn with_port(port: Box<dyn serial::SerialPort>) -> Self {
        Self {
            port: Some(port),
            ..Self::new()
        }
    }

    /// Drops a port whose connection is gone, so `is_connected()` is false,
    /// and tells listeners. Returns the error to report.
    fn drop_dead_port(&mut self, e: &std::io::Error) -> gcodekit5_core::Error {
        if let Some(mut port) = self.port.take() {
            if let Err(close_err) = port.close() {
                tracing::debug!("Error closing dead port: {}", close_err);
            }
        }
        let msg = format!("Connection lost: {}", e);
        tracing::warn!("{}", msg);
        self.notify_listeners(CommunicatorEvent::Disconnected, &msg);
        gcodekit5_core::Error::other(msg)
    }

    /// Turns a port write error into the error `send` returns. A write that
    /// fails because the connection is gone (cable pulled, peer closed)
    /// drops the port, like the same error on a read.
    fn write_failed(&mut self, e: std::io::Error) -> gcodekit5_core::Error {
        if is_terminal_io_error(e.kind()) {
            self.drop_dead_port(&e)
        } else {
            let msg = format!("Send error: {}", e);
            self.notify_listeners(CommunicatorEvent::Error, &msg);
            gcodekit5_core::Error::other(msg)
        }
    }

    /// Notify listeners of an event
    fn notify_listeners(&self, event: CommunicatorEvent, message: &str) {
        for listener in &self.listeners {
            match event {
                CommunicatorEvent::Connected => listener.on_connected(),
                CommunicatorEvent::Disconnected => listener.on_disconnected(),
                CommunicatorEvent::Error => listener.on_error(message),
                CommunicatorEvent::DataReceived => listener.on_data_received(message.as_bytes()),
                CommunicatorEvent::DataSent => listener.on_data_sent(message.as_bytes()),
                CommunicatorEvent::Timeout => listener.on_timeout(),
            }
        }
    }

    /// Returns and clears the Repetier events accumulated since the last call
    pub fn take_repetier_events(&mut self) -> Vec<RepetierEvent> {
        std::mem::take(&mut self.repetier_events)
    }

    /// Checks if the Repetier stream is paused by the user
    /// The RepetierSender checks this before sending each command
    pub fn repetier_is_paused(&self) -> bool {
        self.repetier_flow_paused
    }

    /// Checks if the Repetier stream has been paused by the user
    /// The RepetierSender checks this to terminate its loop
    pub fn repetier_is_stopped(&self) -> bool {
        self.repetier_flow_stopped
    }

    /// Clears the flow control flags
    pub fn repetier_reset_flow_control(&mut self) {
        self.repetier_flow_paused = false;
        self.repetier_flow_stopped = false;
    }

    /// Returns the current value of Repetier's line counter N
    pub fn repetier_line_counter(&self) -> u32 {
        self.repetier_processor.line_counter
    }

    /// Sets the Repetier line counter N (used by the sender during rewind)
    pub fn repetier_set_line_counter(&mut self, n: u32) {
        self.repetier_processor.line_counter = n;
    }

    /// Answers a firmware `Resend:N` by writing the numbered lines from
    /// `from` onwards again, exactly as they were first sent.
    ///
    /// # Returns
    /// The number of lines written (each needs one ack), or an error when
    /// a requested line is no longer known or the port write fails.
    pub fn repetier_resend(&mut self, from: u32) -> gcodekit5_core::Result<usize> {
        let lines = self.repetier_processor.resend_lines(from).ok_or_else(|| {
            gcodekit5_core::Error::other(format!("line N{from} is not available to re-send"))
        })?;
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| gcodekit5_core::Error::other("Not connected to serial port"))?;
        let payload = lines.concat();
        let mut bytes_to_send = payload.as_bytes();
        while !bytes_to_send.is_empty() {
            match port.write(bytes_to_send) {
                Ok(0) => return Err(gcodekit5_core::Error::other("Full port".to_string())),
                Ok(n) => bytes_to_send = &bytes_to_send[n..],
                Err(e) => return Err(gcodekit5_core::Error::other(e.to_string())),
            }
        }
        if !lines.is_empty() {
            self.repetier_processor.mark_buffer_full();
            self.notify_listeners(CommunicatorEvent::DataSent, &payload);
        }
        Ok(lines.len())
    }

    /// Sets the protocol configured by the user (via Device Manager)
    ///
    /// The CONFIGURATION is the source of truth for the operating mode
    /// - `Repetier`: activates the `RepetierProcessor`
    /// - Anything else: deactivates it
    ///
    /// Must be called **after `connect()`** and **before** the first
    /// command (reset, $I, etc.)
    pub fn set_configured_protocol(&mut self, protocol: ControllerProtocol) {
        self.configured_protocol = protocol;
        if protocol == ControllerProtocol::Repetier {
            self.repetier_processor.activate();
        }
        tracing::debug!("SerialCommunicator: protocolo configurado a {:?}", protocol);
    }

    /// User-configured protocol. Source of truth
    pub fn configured_protocol(&self) -> ControllerProtocol {
        self.configured_protocol
    }

    /// Protocol detected via auto-detection. For information only
    pub fn detected_protocol(&self) -> ControllerProtocol {
        self.detected_protocol
    }
}

impl Default for SerialCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Presents a TCP connection through the [`serial::SerialPort`] interface, so the
/// byte-stream code built on [`SerialCommunicator`] works unchanged over the network
/// (e.g. FluidNC or grblHAL controllers reached by Wi-Fi/Ethernet).
struct TcpAsSerialPort {
    inner: Box<dyn tcp::TcpPort>,
    name: String,
}

impl serial::SerialPort for TcpAsSerialPort {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.inner.write(data)
    }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.inner.read(buf) {
            // A zero-byte read on a TCP stream means the peer closed the connection;
            // report it instead of returning "no data" forever.
            Ok(0) if !buf.is_empty() => Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "connection closed by the controller",
            )),
            other => other,
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn close(&mut self) -> std::io::Result<()> {
        self.inner.close()
    }
}

/// Returns true for I/O errors that mean the connection itself is gone (as opposed to
/// a timeout or a transient fault), so the port must be dropped.
fn is_terminal_io_error(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::NotConnected
    )
}

impl Communicator for SerialCommunicator {
    /// Opens the connection described by `params`.
    ///
    /// Serial ports are opened directly; TCP connections are opened with
    /// [`tcp::RealTcpPort`] and carried through the same byte-stream interface.
    /// WebSocket connections are not supported and return an error.
    fn connect(&mut self, params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        let opened: gcodekit5_core::Result<Box<dyn serial::SerialPort>> = match params.driver {
            ConnectionDriver::Serial => serial::RealSerialPort::open(params)
                .map(|p| Box::new(p) as Box<dyn serial::SerialPort>),
            ConnectionDriver::Tcp => tcp::RealTcpPort::open(params).and_then(|p| {
                // `open` applies `timeout_ms` to connect, read and write. Reads here are
                // polled with the communicator lock held, so use the same short timeout
                // as the serial port; `timeout_ms` still governs connecting and writes.
                p.set_read_timeout(Some(serial::POLL_READ_TIMEOUT))
                    .map_err(|e| {
                        gcodekit5_core::Error::other(format!("Failed to set TCP read timeout: {e}"))
                    })?;
                Ok(Box::new(TcpAsSerialPort {
                    inner: Box::new(p),
                    name: format!("{}:{}", params.port, params.network_port),
                }) as Box<dyn serial::SerialPort>)
            }),
            ConnectionDriver::WebSocket => Err(gcodekit5_core::Error::other(
                "WebSocket connections are not supported yet",
            )),
        };

        // Reset emulation flags upon connection
        self.configured_protocol = ControllerProtocol::Unknown;
        self.detected_protocol = ControllerProtocol::Unknown;
        self.repetier_flow_paused = false;
        self.repetier_flow_stopped = false;

        match opened {
            Ok(port) => {
                let name = port.name();
                self.port = Some(port);
                self.params = Some(params.clone());
                self.notify_listeners(
                    CommunicatorEvent::Connected,
                    &format!("Connected to {name}"),
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to connect: {}", e);
                self.notify_listeners(CommunicatorEvent::Error, &msg);
                Err(e)
            }
        }
    }

    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
        self.configured_protocol = ControllerProtocol::Unknown;
        self.detected_protocol = ControllerProtocol::Unknown;

        if let Some(mut port) = self.port.take() {
            match port.close() {
                Ok(()) => {
                    self.notify_listeners(
                        CommunicatorEvent::Disconnected,
                        "Disconnected from serial port",
                    );
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("Error closing port: {}", e);
                    self.notify_listeners(CommunicatorEvent::Error, &msg);
                    Err(gcodekit5_core::Error::other(msg))
                }
            }
        } else {
            Ok(())
        }
    }

    fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
        if let Some(port) = &mut self.port {
            let raw_str = String::from_utf8_lossy(data);
            let cleaned = raw_str.trim().to_ascii_uppercase();

            if self.configured_protocol == ControllerProtocol::Repetier {
                // Drop only truly empty payloads. Line terminators (`\n`,
                // `\r`) must reach the port: `send_command` issues them
                // separately, and if they are dropped the command is left
                // unterminated and the firmware silently ignores it.
                if data.is_empty() {
                    return Ok(data.len());
                }

                if data == b"!" {
                    tracing::debug!("[REPETIER] Pause requested; streaming on hold");
                    self.repetier_flow_paused = true;
                    return Ok(0);
                }
                if data == b"~" {
                    tracing::debug!("[REPETIER] Resume requested; streaming continues");
                    self.repetier_flow_paused = false;
                    return Ok(0);
                }
                // 0x18 = Ctrl-X (soft reset)
                // 0x18 = Ctrl-X (soft reset). Es un comando UNIVERSAL:
                // todos los firmwares lo entienden. Debe salir al puerto
                // tal cual, sin pasar por el processor Repetier (que lo
                // descartaría porque no encaja en su formato N<num>*ck).
                if data == [0x18u8] {
                    tracing::debug!("[REPETIER] Soft reset (sent without the processor)");
                    self.repetier_flow_stopped = true;

                    // Enviar directo al puerto.
                    let mut bytes_to_send = data;
                    while !bytes_to_send.is_empty() {
                        match port.write(bytes_to_send) {
                            Ok(0) => {
                                return Err(gcodekit5_core::Error::other("Full port".to_string()))
                            }
                            Ok(n) => bytes_to_send = &bytes_to_send[n..],
                            Err(e) => return Err(self.write_failed(e)),
                        }
                    }
                    return Ok(data.len());
                }

                if let Some(translated_bytes) = self.repetier_processor.process_outgoing(data) {
                    // Trace: the exact command written to the port.
                    // Format: "[TX] N<num> <gcode> *<checksum>"
                    for tx_line in translated_bytes.split(|&b| b == b'\n') {
                        if !tx_line.is_empty() {
                            tracing::debug!(
                                "[TX] {:?}",
                                String::from_utf8_lossy(&translated_bytes)
                            );
                        }
                    }

                    let mut bytes_to_send = translated_bytes.as_slice();
                    while !bytes_to_send.is_empty() {
                        match port.write(bytes_to_send) {
                            Ok(0) => {
                                return Err(gcodekit5_core::Error::other("Full port".to_string()))
                            }
                            Ok(n) => bytes_to_send = &bytes_to_send[n..],
                            Err(e) => return Err(self.write_failed(e)),
                        }
                    }

                    if cleaned.starts_with("$J=")
                        || cleaned.starts_with('G')
                        || cleaned.starts_with('X')
                    {
                        self.repetier_processor.mark_buffer_full();
                    }

                    self.notify_listeners(
                        CommunicatorEvent::DataSent,
                        &String::from_utf8_lossy(&translated_bytes),
                    );
                    return Ok(data.len());
                } else {
                    tracing::debug!("[TX-DROPPED] {:?}", cleaned);
                    return Ok(0);
                }
            }

            match port.write(data) {
                Ok(n) => {
                    tracing::debug!("[TX] (raw) {:?}", String::from_utf8_lossy(data));
                    let sent_data = &data[..n];
                    let data_str = String::from_utf8_lossy(sent_data);
                    self.notify_listeners(CommunicatorEvent::DataSent, &data_str);
                    Ok(n)
                }
                Err(e) => Err(self.write_failed(e)),
            }
        } else {
            Err(gcodekit5_core::Error::other("Not connected to serial port"))
        }
    }

    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> {
        if let Some(port) = &mut self.port {
            let mut buf = [0u8; 4096];
            match port.read(&mut buf) {
                Ok(n) => {
                    let mut data = buf[..n].to_vec();

                    if !data.is_empty() {
                        let incoming_text = String::from_utf8_lossy(&data);
                        tracing::debug!("[RX] {}", incoming_text.trim_end().replace('\n', " | "));

                        // ─── Auto-detection (informational only) ───
                        // Uses the PROTOCOLS table from protocol_registry to
                        // determine which protocol is transmitting. It does NOT
                        // activate the processor. The protocol is determined by
                        // `configured_protocol`.
                        //
                        // If something has already been detected, detection is
                        // not performed again (prevents overwriting due to
                        // false positives from subsequent messages).
                        if self.detected_protocol == ControllerProtocol::Unknown {
                            if let Some(detected) =
                                crate::firmware::protocol_registry::detect_protocol(&incoming_text)
                            {
                                tracing::debug!("Auto-detected protocol: {:?}.", detected);
                                self.detected_protocol = detected;
                            }
                        }

                        // ─── Operating mode based on configuration ───
                        if self.configured_protocol == ControllerProtocol::Repetier {
                            let result = self.repetier_processor.process_incoming(data.clone());

                            for event in &result.events {
                                match event {
                                    RepetierEvent::Ack => {
                                        self.repetier_processor.mark_ack();
                                    }
                                    RepetierEvent::Resend(_n) => {
                                        self.repetier_processor.mark_buffer_full();
                                    }
                                }
                            }

                            self.repetier_events.extend(result.events.clone());

                            data = result.display;
                        } else {
                            let data_str = String::from_utf8_lossy(&data);
                            self.notify_listeners(CommunicatorEvent::DataReceived, &data_str);
                        }
                    }
                    Ok(data)
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut
                    {
                        Ok(vec![])
                    } else if is_terminal_io_error(e.kind()) {
                        // The connection is gone (serialport reports a pulled USB
                        // cable as BrokenPipe): drop the port so is_connected() is
                        // false and listeners (the UI) see the disconnect.
                        Err(self.drop_dead_port(&e))
                    } else {
                        let msg = format!("Receive error: {}", e);
                        self.notify_listeners(CommunicatorEvent::Error, &msg);
                        Err(gcodekit5_core::Error::other(msg))
                    }
                }
            }
        } else {
            Err(gcodekit5_core::Error::other("Not connected to serial port"))
        }
    }

    fn add_listener(&mut self, listener: CommunicatorListenerHandle) {
        self.listeners.push(listener);
    }

    fn remove_listener(&mut self, listener: &CommunicatorListenerHandle) {
        self.listeners.retain(|l| !Arc::ptr_eq(l, listener));
    }

    fn connection_params(&self) -> Option<&ConnectionParams> {
        self.params.as_ref()
    }

    fn set_connection_params(&mut self, params: ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        self.params = Some(params);
        Ok(())
    }
}
