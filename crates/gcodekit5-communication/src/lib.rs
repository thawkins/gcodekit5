//! # GCodeKit5 Communication
//!
//! Communication protocols and firmware implementations for GCodeKit5.
//! Supports Serial/USB, TCP/IP, and WebSocket connections.
//! Includes the GRBL and Repetier protocol implementations.
#![deny(unsafe_code)]

pub mod communication;
pub mod firmware;

pub use communication::{
    Communicator, CommunicatorEvent, CommunicatorListener, CommunicatorListenerHandle,
    ConnectionDriver, ConnectionParams, NoOpCommunicator, SerialCommunicator, SerialParity,
    serial::{SerialPortInfo, list_ports},
    tcp::TcpConnectionInfo,
};

pub use firmware::ControllerType;
