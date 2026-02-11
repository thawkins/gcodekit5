//! Event system for controller communication
//!
//! Provides:
//! - Event types for controller and machine state changes
//! - Event dispatcher for publishing events to subscribers
//! - Listener registration and management

use crate::data::{ControllerState, ControllerStatus};
use tokio::sync::broadcast;

/// Controller event types
#[derive(Debug, Clone)]
pub enum ControllerEvent {
    /// Connection state changed
    Connected(String),
    /// Disconnection occurred
    Disconnected,
    /// Controller state changed
    StateChanged(ControllerState),
    /// Controller status changed
    StatusChanged(ControllerStatus),
    /// Alarm occurred
    Alarm(u32, String),
    /// Error occurred
    Error(String),
    /// Command completed
    CommandComplete(String),
    /// Position changed
    PositionChanged {
        /// Machine position as (X, Y, Z) coordinates in mm.
        machine_pos: (f64, f64, f64),
        /// Work position as (X, Y, Z) coordinates in mm.
        work_pos: (f64, f64, f64),
    },
    /// Spindle speed changed
    SpindleSpeedChanged(f64),
    /// Feed rate changed
    FeedRateChanged(f64),
}

impl std::fmt::Display for ControllerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllerEvent::Connected(name) => write!(f, "Connected to {}", name),
            ControllerEvent::Disconnected => write!(f, "Disconnected"),
            ControllerEvent::StateChanged(state) => write!(f, "State: {}", state),
            ControllerEvent::StatusChanged(status) => write!(f, "Status: {}", status),
            ControllerEvent::Alarm(code, desc) => write!(f, "Alarm {} ({})", code, desc),
            ControllerEvent::Error(msg) => write!(f, "Error: {}", msg),
            ControllerEvent::CommandComplete(cmd) => write!(f, "Command complete: {}", cmd),
            ControllerEvent::PositionChanged {
                machine_pos,
                work_pos,
            } => {
                write!(
                    f,
                    "Position - Machine: {:?}, Work: {:?}",
                    machine_pos, work_pos
                )
            }
            ControllerEvent::SpindleSpeedChanged(speed) => write!(f, "Spindle: {} RPM", speed),
            ControllerEvent::FeedRateChanged(rate) => write!(f, "Feed rate: {} mm/min", rate),
        }
    }
}

/// Event dispatcher for publishing events to subscribers
#[derive(Clone)]
pub struct EventDispatcher {
    /// Broadcast sender channel for controller events.
    tx: broadcast::Sender<ControllerEvent>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    ///
    /// # Arguments
    /// * `buffer_size` - Size of the broadcast buffer (default 100)
    pub fn new(buffer_size: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer_size);
        Self { tx }
    }

    /// Create a new event dispatcher with default buffer size
    pub fn default_with_buffer() -> Self {
        Self::new(100)
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ControllerEvent> {
        self.tx.subscribe()
    }

    /// Publish an event to all subscribers
    pub fn publish(
        &self,
        event: ControllerEvent,
    ) -> Result<usize, broadcast::error::SendError<ControllerEvent>> {
        self.tx.send(event)
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::default_with_buffer()
    }
}
