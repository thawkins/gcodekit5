//! G-Code parser and state machine
//!
//! This module provides:
//! - G-Code command parsing
//! - Modal state tracking
//! - Command validation
//! - Preprocessor framework
//! - Command lifecycle management
//! - Command listener framework
//! - Stream management (reading from files or strings)

// Shared with the CAM tools, so they live in gcodekit5-core.
pub use gcodekit5_core::gcode::arc;
pub mod command;
pub mod parser;
pub mod pipeline;
pub mod processors;
pub mod stream;
pub use gcodekit5_core::gcode::words;

pub use command::*;
pub use parser::*;
pub use pipeline::*;
pub use processors::*;
