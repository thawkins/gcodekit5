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

pub mod command;
pub mod parser;
pub mod pipeline;
pub mod processors;
pub mod stream;

pub use command::*;
pub use parser::*;
pub use pipeline::*;
pub use processors::*;
