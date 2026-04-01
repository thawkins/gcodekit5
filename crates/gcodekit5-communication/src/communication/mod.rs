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

pub mod buffered;
pub mod serial;
pub mod tcp;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

pub use buffered::{
    BufferedCommand, BufferedCommunicatorConfig, BufferedCommunicatorWrapper, CommandStatus,
};
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

    /// Send a command and receive response
    ///
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
}

impl SerialCommunicator {
    /// Create a new serial communicator
    pub fn new() -> Self {
        Self {
            port: None,
            params: None,
            listeners: Vec::new(),
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
}

impl Default for SerialCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Communicator for SerialCommunicator {
    fn connect(&mut self, params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        if params.driver != ConnectionDriver::Serial {
            return Err(gcodekit5_core::Error::other(
                "SerialCommunicator requires Serial driver type",
            ));
        }

        // Try to open the port
        match serial::RealSerialPort::open(params) {
            Ok(port) => {
                self.port = Some(Box::new(port));
                self.params = Some(params.clone());
                self.notify_listeners(CommunicatorEvent::Connected, "Connected to serial port");
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to connect: {}", e);
                // tracing::error!("{}", msg);
                self.notify_listeners(CommunicatorEvent::Error, &msg);
                Err(e)
            }
        }
    }

    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
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
                    // tracing::error!("{}", msg);
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
            match port.write(data) {
                Ok(n) => {
                    // Notify listeners of sent data
                    let sent_data = &data[..n];
                    let data_str = String::from_utf8_lossy(sent_data);
                    self.notify_listeners(CommunicatorEvent::DataSent, &data_str);
                    Ok(n)
                }
                Err(e) => {
                    let msg = format!("Send error: {}", e);
                    // Suppress BrokenPipe errors which happen during disconnect
                    // if e.kind() != std::io::ErrorKind::BrokenPipe {
                    //     tracing::error!("{}", msg);
                    // }
                    self.notify_listeners(CommunicatorEvent::Error, &msg);
                    Err(gcodekit5_core::Error::other(msg))
                }
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
                    let data = buf[..n].to_vec();
                    // Notify listeners of received data
                    if !data.is_empty() {
                        let data_str = String::from_utf8_lossy(&data);
                        self.notify_listeners(CommunicatorEvent::DataReceived, &data_str);
                    }
                    Ok(data)
                }
                Err(e) => {
                    // Check if it's a timeout (no data) which is normal
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut
                    {
                        Ok(vec![])
                    } else if e.kind() == std::io::ErrorKind::BrokenPipe {
                        // BrokenPipe during receive often happens during cleanup or disconnect
                        Ok(vec![])
                    } else {
                        let msg = format!("Receive error: {}", e);
                        // tracing::error!("{}", msg);
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

/// TCP/Network communicator for remote controller connections
///
/// Provides network-based communication using TCP/IP for connecting to
/// CNC controllers over Ethernet or other network connections.
pub struct TcpCommunicator {
    port: Option<Box<dyn tcp::TcpPort>>,
    params: Option<ConnectionParams>,
    listeners: Vec<CommunicatorListenerHandle>,
}

impl TcpCommunicator {
    /// Create a new TCP communicator
    pub fn new() -> Self {
        Self {
            port: None,
            params: None,
            listeners: Vec::new(),
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
}

impl Default for TcpCommunicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Communicator for TcpCommunicator {
    fn connect(&mut self, params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        params.validate()?;
        if params.driver != ConnectionDriver::Tcp {
            return Err(gcodekit5_core::Error::other(
                "TcpCommunicator requires Tcp driver type",
            ));
        }

        // Try to open the connection
        match tcp::RealTcpPort::open(params) {
            Ok(port) => {
                self.port = Some(Box::new(port));
                self.params = Some(params.clone());
                self.notify_listeners(CommunicatorEvent::Connected, "Connected to TCP server");
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to connect: {}", e);
                // tracing::error!("{}", msg);
                self.notify_listeners(CommunicatorEvent::Error, &msg);
                Err(e)
            }
        }
    }

    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
        if let Some(mut port) = self.port.take() {
            match port.close() {
                Ok(()) => {
                    self.notify_listeners(
                        CommunicatorEvent::Disconnected,
                        "Disconnected from TCP server",
                    );
                    Ok(())
                }
                Err(e) => {
                    let msg = format!("Error closing port: {}", e);
                    // tracing::error!("{}", msg);
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
            match port.write(data) {
                Ok(n) => {
                    let sent_data = &data[..n];
                    let data_str = String::from_utf8_lossy(sent_data);
                    self.notify_listeners(CommunicatorEvent::DataSent, &data_str);
                    Ok(n)
                }
                Err(e) => {
                    // Handle BrokenPipe more gracefully - it often occurs during cleanup
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        // Return success for BrokenPipe if data was likely sent
                        Ok(0)
                    } else {
                        let msg = format!("Send error: {}", e);
                        // tracing::error!("{}", msg);
                        self.notify_listeners(CommunicatorEvent::Error, &msg);
                        Err(gcodekit5_core::Error::other(msg))
                    }
                }
            }
        } else {
            Err(gcodekit5_core::Error::other("Not connected to TCP server"))
        }
    }

    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> {
        if let Some(port) = &mut self.port {
            let mut buf = [0u8; 4096];
            match port.read(&mut buf) {
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    if n > 0 {
                        self.notify_listeners(
                            CommunicatorEvent::DataReceived,
                            &format!("{} bytes", n),
                        );
                    }
                    Ok(data)
                }
                Err(e) => {
                    // Check if it's a timeout (no data) which is normal
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut
                    {
                        Ok(vec![])
                    } else if e.kind() == std::io::ErrorKind::BrokenPipe {
                        // BrokenPipe during receive often happens during cleanup or disconnect
                        Ok(vec![])
                    } else {
                        let msg = format!("Receive error: {}", e);
                        // tracing::error!("{}", msg);
                        self.notify_listeners(CommunicatorEvent::Error, &msg);
                        Err(gcodekit5_core::Error::other(msg))
                    }
                }
            }
        } else {
            Err(gcodekit5_core::Error::other("Not connected to TCP server"))
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
