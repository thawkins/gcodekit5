//! # GCodeKit4 Core
//!
//! Core types, traits, and utilities for GCodeKit4.
//! Provides the fundamental abstractions for controller management,
//! state machines, events, and data models.

pub mod constants;
pub mod core;
pub mod data;
pub mod error;
pub mod event_bus;
pub mod types;
pub mod units;

pub use core::{
    event::{ControllerEvent, EventDispatcher},
    message::{Message, MessageDispatcher, MessageLevel},
    ControllerListener, ControllerListenerHandle, ControllerTrait, OverrideState, SimpleController,
};

pub use data::{
    CNCPoint, CommunicatorState, ControllerState, ControllerStatus, MachineStatus,
    MachineStatusSnapshot, PartialPosition, Position, Units,
};

pub use error::{ConnectionError, ControllerError, Error, FirmwareError, GcodeError, Result};

// Re-export event bus for convenience
pub use event_bus::{
    event_bus, AppEvent, EventBus, EventBusConfig, EventCategory, EventFilter, SubscriptionId,
};

// Re-export type aliases for convenience
pub use types::{
    shared, shared_none, shared_some, thread_safe, thread_safe_deque, thread_safe_map,
    thread_safe_none, thread_safe_rw, thread_safe_some, thread_safe_vec, Callback, DataCallback,
    ProgressCallback, ResultCallback, Shared, SharedHashMap, SharedOption, SharedVec, ThreadSafe,
    ThreadSafeDeque, ThreadSafeMap, ThreadSafeOption, ThreadSafeRw, ThreadSafeRwMap, ThreadSafeVec,
    UiCallback, UiDataCallback,
};
