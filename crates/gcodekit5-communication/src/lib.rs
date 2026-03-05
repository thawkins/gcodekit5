// Crate-wide: many protocol implementations are partial/in-progress (TinyG, g2core, FluidNC).
// Fields and methods are defined for API completeness but not yet wired to callers.
#![allow(dead_code)]
//! # GCodeKit4 Communication
//!
//! Communication protocols and firmware implementations for GCodeKit4.
//! Supports Serial/USB, TCP/IP, and WebSocket connections.
//! Includes firmware-specific implementations for GRBL, TinyG, g2core, etc.

pub mod communication;
pub mod error;
pub mod firmware;

pub use communication::{
    serial::{list_ports, SerialPortInfo},
    tcp::TcpConnectionInfo,
    BufferedCommand, BufferedCommunicatorConfig, BufferedCommunicatorWrapper, CommandStatus,
    Communicator, CommunicatorEvent, CommunicatorListener, CommunicatorListenerHandle,
    ConnectionDriver, ConnectionParams, NoOpCommunicator, SerialCommunicator, SerialParity,
    TcpCommunicator,
};

pub use firmware::{CapabilityManager, CapabilityState, ControllerType, FirmwareDetector};
