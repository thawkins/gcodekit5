//! Controller listener interface
//!
//! Defines the listener trait for controller events

use crate::core::ControllerState;
use crate::data::ControllerStatus;
use async_trait::async_trait;

/// Handle for a registered controller listener.
///
/// Uniquely identifies a listener subscription. Can be used to unsubscribe
/// from controller events.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ControllerListenerHandle(pub String);

/// Listener trait for controller events
///
/// Implement this trait to receive notifications of controller state changes
#[async_trait]
pub trait ControllerListener: Send + Sync {
    /// Called when controller state changes
    async fn on_state_changed(&self, _new_state: ControllerState) {}

    /// Called when controller status is updated
    async fn on_status_changed(&self, _status: &ControllerStatus) {}

    /// Called when an alarm occurs
    async fn on_alarm(&self, _code: u32, _description: &str) {}

    /// Called when an error occurs
    async fn on_error(&self, _message: &str) {}

    /// Called when a command is completed
    async fn on_command_complete(&self, _command: &str) {}
}
