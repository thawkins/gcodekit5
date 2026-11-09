//! TinyG Controller Implementation
//!
//! Placeholder implementation of the ControllerTrait for TinyG firmware. The TinyG
//! transport and JSON protocol are not implemented yet, so every machine operation
//! returns a clear "not supported" error instead of silently succeeding.

use crate::communication::ConnectionParams;
use async_trait::async_trait;
use gcodekit5_core::{thread_safe_rw, ThreadSafeRw, ThreadSafeRwMap};
use gcodekit5_core::{
    ControllerState, ControllerStatus, ControllerTrait, PartialPosition, Position,
};

/// TinyG Controller state management
#[derive(Debug, Clone)]
struct TinyGControllerState {
    /// Current connection state
    pub state: ControllerState,
    /// Current status
    pub status: ControllerStatus,
    /// Machine position
    pub machine_position: Position,
    /// Work position
    pub work_position: Position,
    /// Is streaming active
    pub is_streaming: bool,
    /// Status poll rate (milliseconds)
    pub poll_rate_ms: u64,
    /// TinyG version
    pub version: Option<String>,
    /// TinyG buffer level
    pub buffer_level: u8,
}

impl Default for TinyGControllerState {
    fn default() -> Self {
        Self {
            state: ControllerState::Disconnected,
            status: ControllerStatus::Idle,
            machine_position: Position::default(),
            work_position: Position::default(),
            is_streaming: false,
            poll_rate_ms: 200,
            version: None,
            buffer_level: 0,
        }
    }
}

/// TinyG Controller implementation
///
/// **Not yet functional.** There is no serial/TCP transport or JSON protocol
/// handling for TinyG in this crate, so instead of silently pretending to
/// succeed, `connect`, `send_command`, jogging, probing and every other machine
/// operation return a [`gcodekit5_core::FirmwareError::UnsupportedFeature`] error.
///
/// Implements the ControllerTrait for TinyG firmware with full protocol support.
/// TinyG uses JSON-based communication for increased flexibility and feature support.
pub struct TinyGController {
    /// Name identifier
    name: String,
    /// Controller state
    state: ThreadSafeRw<TinyGControllerState>,
    /// Registered controller listeners
    listeners: ThreadSafeRwMap<String, std::sync::Arc<dyn gcodekit5_core::ControllerListener>>,
    /// Connection parameters
    connection_params: ConnectionParams,
}

impl TinyGController {
    /// Create a new TinyG controller.
    ///
    /// The controller is created disconnected. See the type-level docs: the
    /// TinyG protocol is not implemented yet, so every machine operation
    /// returns a "not supported" error.
    pub fn new(connection_params: ConnectionParams, name: Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.unwrap_or_else(|| "TinyG".to_string()),
            state: thread_safe_rw(TinyGControllerState::default()),
            listeners: thread_safe_rw(std::collections::HashMap::new()),
            connection_params,
        })
    }

    /// Connection parameters this controller was created with.
    pub fn connection_params(&self) -> &ConnectionParams {
        &self.connection_params
    }

    /// Build the error returned by every operation that needs the (unimplemented)
    /// TinyG transport.
    fn unsupported(&self, operation: &str) -> anyhow::Error {
        tracing::warn!(controller = %self.name, operation, "TinyG operation not supported");
        anyhow::Error::new(gcodekit5_core::FirmwareError::UnsupportedFeature {
            firmware: "TinyG".to_string(),
            feature: format!(
                "{operation} (TinyG communication is not implemented; use a GRBL-compatible controller)"
            ),
        })
    }
}

#[async_trait]
impl ControllerTrait for TinyGController {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_state(&self) -> ControllerState {
        self.state.read().state
    }

    fn get_status(&self) -> ControllerStatus {
        self.state.read().status
    }

    fn get_override_state(&self) -> gcodekit5_core::OverrideState {
        gcodekit5_core::OverrideState::default()
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        // Refuse rather than report a connection that does not exist.
        self.state.write().state = ControllerState::Disconnected;
        Err(self.unsupported("connecting"))
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        let mut state = self.state.write();
        state.state = ControllerState::Disconnected;
        state.is_streaming = false;
        Ok(())
    }

    async fn send_command(&mut self, _command: &str) -> anyhow::Result<()> {
        Err(self.unsupported("sending commands"))
    }

    async fn home(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("homing"))
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("soft reset"))
    }

    async fn clear_alarm(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("clearing alarms"))
    }

    async fn unlock(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("unlocking"))
    }

    async fn jog_start(
        &mut self,
        _axis: char,
        _direction: i32,
        _feed_rate: f64,
    ) -> anyhow::Result<()> {
        Err(self.unsupported("continuous jogging"))
    }

    async fn jog_stop(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("stopping a jog"))
    }

    async fn jog_incremental(
        &mut self,
        _axis: char,
        _distance: f64,
        _feed_rate: f64,
    ) -> anyhow::Result<()> {
        Err(self.unsupported("incremental jogging"))
    }

    async fn start_streaming(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("streaming"))
    }

    async fn pause_streaming(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("pausing (feed hold)"))
    }

    async fn resume_streaming(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("resuming (cycle start)"))
    }

    async fn cancel_streaming(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("cancelling a stream"))
    }

    async fn probe_z(&mut self, _feed_rate: f64) -> anyhow::Result<PartialPosition> {
        Err(self.unsupported("Z probing"))
    }

    async fn probe_x(&mut self, _feed_rate: f64) -> anyhow::Result<PartialPosition> {
        Err(self.unsupported("X probing"))
    }

    async fn probe_y(&mut self, _feed_rate: f64) -> anyhow::Result<PartialPosition> {
        Err(self.unsupported("Y probing"))
    }

    async fn set_feed_override(&mut self, _percentage: u16) -> anyhow::Result<()> {
        Err(self.unsupported("feed override"))
    }

    async fn set_rapid_override(&mut self, _percentage: u8) -> anyhow::Result<()> {
        Err(self.unsupported("rapid override"))
    }

    async fn set_spindle_override(&mut self, _percentage: u16) -> anyhow::Result<()> {
        Err(self.unsupported("spindle override"))
    }

    async fn set_work_zero(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("setting work zero"))
    }

    async fn set_work_zero_axes(&mut self, _axes: &str) -> anyhow::Result<()> {
        Err(self.unsupported("setting work zero"))
    }

    async fn go_to_work_zero(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("moving to work zero"))
    }

    async fn set_work_coordinate_system(&mut self, _wcs: u8) -> anyhow::Result<()> {
        Err(self.unsupported("selecting a work coordinate system"))
    }

    async fn get_wcs_offset(&self, _wcs: u8) -> anyhow::Result<PartialPosition> {
        Err(self.unsupported("reading work coordinate offsets"))
    }

    async fn query_status(&mut self) -> anyhow::Result<ControllerStatus> {
        Err(self.unsupported("querying status"))
    }

    async fn query_settings(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("querying settings"))
    }

    async fn query_parser_state(&mut self) -> anyhow::Result<()> {
        Err(self.unsupported("querying parser state"))
    }

    fn register_listener(
        &mut self,
        listener: std::sync::Arc<dyn gcodekit5_core::ControllerListener>,
    ) -> gcodekit5_core::ControllerListenerHandle {
        let id = uuid::Uuid::new_v4().to_string();
        self.listeners.write().insert(id.clone(), listener);
        gcodekit5_core::ControllerListenerHandle(id)
    }

    fn unregister_listener(&mut self, handle: gcodekit5_core::ControllerListenerHandle) {
        self.listeners.write().remove(&handle.0);
    }

    fn listener_count(&self) -> usize {
        self.listeners.read().len()
    }
}
