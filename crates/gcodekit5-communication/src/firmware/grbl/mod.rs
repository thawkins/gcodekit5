//! GRBL Firmware Support
//!
//! This module provides complete support for GRBL (open-source CNC control software).
//! It includes protocol constants, capabilities, version detection, and feature flags.

pub mod capabilities;
pub mod command_creator;
pub mod communicator;
pub mod constants;
pub mod controller;
pub mod error_decoder;
pub mod override_manager;
pub mod response_parser;
pub mod settings;
pub mod status_parser;
pub mod test;
pub mod utils;

pub use capabilities::{
    GrblCapabilities, GrblFeature, GrblFeatureSet, GrblVersion, VersionComparison,
};
pub use command_creator::{
    CommandCreator, JogCommand, JogMode, ProbeCommand, ProbeType, RealTimeCommand, SystemCommand,
};
pub use communicator::{GrblCommunicator, GrblCommunicatorConfig};
pub use constants::*;
pub use controller::GrblController;
pub use error_decoder::{decode_alarm, decode_error, format_alarm, format_error};
pub use override_manager::{OverrideManager, RealTimeOverrideCommand};
pub use response_parser::{BufferState, GrblResponse, GrblResponseParser, StatusReport};
pub use settings::{Setting, SettingsManager};
pub use status_parser::{
    BufferRxState, FeedSpindleState, FullStatus, MachinePosition, StatusParser,
    WorkCoordinateOffset, WorkPosition,
};
