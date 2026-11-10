//! # GCodeKit5 Core
//!
//! Core types, traits, and utilities for GCodeKit5.
//! Provides the fundamental abstractions for controller management,
//! state machines, events, and data models.

pub mod constants;
pub mod core;
pub mod data;
pub mod error;
pub mod gcode;
pub mod probe;
pub mod types;
pub mod units;

pub use core::{
    OverrideState,
    message::{Message, MessageLevel},
};

pub use data::{
    CNCPoint, CommunicatorState, ControllerState, ControllerStatus, MachineStatus,
    MachineStatusSnapshot, PartialPosition, Position, Units,
};

pub use probe::{
    Axis as ProbeAxis, Corner, Direction as ProbeDirection, ProbePinState, ProbeReport,
    ProbeResult, ProbeRoutine, ProbeType,
};

pub use error::{ConnectionError, ControllerError, Error, FirmwareError, GcodeError, Result};

pub use gcode::{
    CommandId, CommandListener, CommandListenerHandle, CommandNumberGenerator, CommandResponse,
    CommandState, GcodeCommand, NoOpCommandListener,
};

// Re-export type aliases for convenience
pub use types::{
    Callback, CellCallback, CellDataCallback, CellDataCallback2, DataCallback, ProgressCallback,
    ResultCallback, Shared, SharedHashMap, SharedOption, SharedVec, ThreadSafe, ThreadSafeDeque,
    ThreadSafeMap, ThreadSafeOption, ThreadSafeRw, ThreadSafeRwMap, ThreadSafeVec, UiCallback,
    UiDataCallback, shared, shared_none, shared_some, thread_safe, thread_safe_deque,
    thread_safe_map, thread_safe_none, thread_safe_rw, thread_safe_some, thread_safe_vec,
};
