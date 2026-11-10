//! # GCodeKit5 Communication
//!
//! Communication protocols and firmware implementations for GCodeKit5.
//! Supports Serial/USB, TCP/IP, and WebSocket connections.
//! Includes the GRBL and Repetier protocol implementations.

pub mod communication;
pub mod firmware;

pub use communication::{
    serial::{list_ports, SerialPortInfo},
    tcp::TcpConnectionInfo,
    Communicator, CommunicatorEvent, CommunicatorListener, CommunicatorListenerHandle,
    ConnectionDriver, ConnectionParams, NoOpCommunicator, SerialCommunicator, SerialParity,
};

pub use firmware::ControllerType;
