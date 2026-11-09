//! g2core Controller Implementation
//!
//! Placeholder implementation of the ControllerTrait for g2core firmware. The g2core
//! transport and JSON protocol are not implemented yet, so every machine operation
//! returns a clear "not supported" error instead of silently succeeding.
//! g2core is the next generation of TinyG with support for 6 axes and advanced kinematics.

use crate::communication::ConnectionParams;
use async_trait::async_trait;
use gcodekit5_core::{thread_safe_rw, ThreadSafeRw, ThreadSafeRwMap};
use gcodekit5_core::{
    ControllerState, ControllerStatus, ControllerTrait, PartialPosition, Position,
};

/// g2core Controller state management
#[derive(Debug, Clone)]
struct G2CoreControllerState {
    /// Current connection state
    pub state: ControllerState,
    /// Current status
    pub status: ControllerStatus,
    /// Machine position (supports 6 axes)
    pub machine_position: Position,
    /// Work position (supports 6 axes)
    pub work_position: Position,
    /// Is streaming active
    pub is_streaming: bool,
    /// Status poll rate (milliseconds)
    pub poll_rate_ms: u64,
    /// g2core version
    pub version: Option<String>,
    /// g2core buffer level
    pub buffer_level: u8,
    /// Number of active axes
    pub active_axes: u8,
    /// Kinematics mode (if supported)
    pub kinematics_mode: Option<String>,
}

impl Default for G2CoreControllerState {
    fn default() -> Self {
        Self {
            state: ControllerState::Disconnected,
            status: ControllerStatus::Idle,
            machine_position: Position::default(),
            work_position: Position::default(),
            is_streaming: false,
            poll_rate_ms: 150,
            version: None,
            buffer_level: 0,
            active_axes: 6,
            kinematics_mode: None,
        }
    }
}

/// g2core Controller implementation
///
/// **Not yet functional.** There is no serial/TCP transport or JSON protocol
/// handling for g2core in this crate, so instead of silently pretending to
/// succeed, `connect`, `send_command`, jogging, probing and every other machine
/// operation return a [`gcodekit5_core::FirmwareError::UnsupportedFeature`] error.
///
/// Implements the ControllerTrait for g2core firmware with full protocol support.
/// g2core uses JSON-based communication with advanced features including:
/// - 6-axis support
/// - Advanced kinematics
/// - Spindle speed control
/// - Enhanced streaming protocol
pub struct G2CoreController {
    /// Name identifier
    name: String,
    /// Controller state
    state: ThreadSafeRw<G2CoreControllerState>,
    /// Registered controller listeners
    listeners: ThreadSafeRwMap<String, std::sync::Arc<dyn gcodekit5_core::ControllerListener>>,
    /// Connection parameters
    connection_params: ConnectionParams,
}

impl G2CoreController {
    /// Create a new g2core controller.
    ///
    /// The controller is created disconnected. See the type-level docs: the
    /// g2core protocol is not implemented yet, so every machine operation
    /// returns a "not supported" error.
    pub fn new(connection_params: ConnectionParams, name: Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            name: name.unwrap_or_else(|| "g2core".to_string()),
            state: thread_safe_rw(G2CoreControllerState::default()),
            listeners: thread_safe_rw(std::collections::HashMap::new()),
            connection_params,
        })
    }

    /// Connection parameters this controller was created with.
    pub fn connection_params(&self) -> &ConnectionParams {
        &self.connection_params
    }

    /// Build the error returned by every operation that needs the (unimplemented)
    /// g2core transport.
    fn unsupported(&self, operation: &str) -> anyhow::Error {
        tracing::warn!(controller = %self.name, operation, "g2core operation not supported");
        anyhow::Error::new(gcodekit5_core::FirmwareError::UnsupportedFeature {
            firmware: "g2core".to_string(),
            feature: format!(
                "{operation} (g2core communication is not implemented; use a GRBL-compatible controller)"
            ),
        })
    }

    /// Query the number of active axes
    pub fn get_active_axes(&self) -> u8 {
        self.state.read().active_axes
    }

    /// Set the number of active axes
    pub fn set_active_axes(&self, axes: u8) {
        self.state.write().active_axes = axes;
    }

    /// Query the kinematics mode
    pub fn get_kinematics_mode(&self) -> Option<String> {
        self.state.read().kinematics_mode.clone()
    }

    /// Set the kinematics mode
    pub fn set_kinematics_mode(&self, mode: Option<String>) {
        self.state.write().kinematics_mode = mode;
    }
}

#[async_trait]
impl ControllerTrait for G2CoreController {
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
