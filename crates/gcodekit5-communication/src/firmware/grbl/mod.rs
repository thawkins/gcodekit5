//! GRBL Firmware Support
//!
//! This module provides complete support for GRBL (open-source CNC control software).
//! It includes protocol constants, capabilities, version detection, and feature flags.

pub mod error_decoder;
pub mod settings;
pub mod status_parser;
pub mod grbl_sender;
pub use error_decoder::{decode_alarm, decode_error, format_alarm, format_error};
pub use settings::{Setting, SettingsManager};
pub use status_parser::{
    BufferRxState, FeedSpindleState, FullStatus, MachinePosition, StatusParser,
    WorkCoordinateOffset, WorkPosition,
};
pub use grbl_sender::GrblSender;
