//! GRBL Firmware Support
//!
//! This module provides complete support for GRBL (open-source CNC control software).
//! It includes protocol constants, capabilities, version detection, and feature flags.

pub mod error_decoder;
pub mod override_sequencer;
pub mod settings;
pub mod status_parser;
pub use error_decoder::{decode_alarm, decode_error, format_alarm, format_error};
pub use override_sequencer::{
    FEED_OVERRIDE_CODES, MAX_OVERRIDE_PERCENT, MIN_OVERRIDE_PERCENT, OverrideStepCodes,
    SPINDLE_OVERRIDE_CODES, apply_override_step, is_valid_override_target, next_override_step,
};
pub use settings::{Setting, SettingsManager};
pub use status_parser::{
    BufferRxState, FeedSpindleState, FullStatus, MachinePosition, StatusParser,
    WorkCoordinateOffset, WorkPosition,
};
