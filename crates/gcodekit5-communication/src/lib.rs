#![allow(dead_code)]
//! # GCodeKit4 Communication
//!
//! Communication protocols and firmware implementations for GCodeKit4.
//! Supports Serial/USB, TCP/IP, and WebSocket connections.
//! Includes firmware-specific implementations for GRBL, TinyG, g2core, etc.

pub mod communication;
pub mod firmware;

pub use communication::{
    serial::{list_ports, SerialPortInfo},
    tcp::TcpConnectionInfo,
    Communicator, CommunicatorEvent, CommunicatorListener, CommunicatorListenerHandle,
    ConnectionDriver, ConnectionParams, NoOpCommunicator, SerialCommunicator, SerialParity,
    TcpCommunicator,
    BufferedCommunicatorWrapper, BufferedCommunicatorConfig, BufferedCommand, CommandStatus,
};

pub use firmware::{CapabilityManager, CapabilityState, ControllerType, FirmwareDetector};
