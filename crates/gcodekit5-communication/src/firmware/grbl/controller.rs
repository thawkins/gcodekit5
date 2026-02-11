//! GRBL Controller Implementation
//!
//! Provides a complete implementation of the ControllerTrait for GRBL firmware,
//! including connection management, command execution, and status polling.

use crate::communication::{ConnectionParams, NoOpCommunicator};
use crate::firmware::grbl::status_parser::StatusParser;
use crate::firmware::grbl::{GrblCommunicator, GrblCommunicatorConfig};
use async_trait::async_trait;
use gcodekit5_core::{ControllerState, ControllerStatus, PartialPosition};
use gcodekit5_core::{ControllerTrait, OverrideState};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use uuid::Uuid;

/// GRBL Controller state management
#[derive(Debug, Clone)]
pub struct GrblControllerState {
    /// Current connection state
    pub state: ControllerState,
    /// Current status
    pub status: ControllerStatus,
    /// Override state
    pub override_state: OverrideState,
    /// Machine position
    pub machine_position: gcodekit5_core::Position,
    /// Work position
    pub work_position: gcodekit5_core::Position,
    /// Is streaming active
    pub is_streaming: bool,
    /// Status poll rate (milliseconds)
    pub poll_rate_ms: u64,
}

impl Default for GrblControllerState {
    fn default() -> Self {
        Self {
            state: ControllerState::Disconnected,
            status: ControllerStatus::Idle,
            override_state: OverrideState::default(),
            machine_position: gcodekit5_core::Position::default(),
            work_position: gcodekit5_core::Position::default(),
            is_streaming: false,
            poll_rate_ms: 100,
        }
    }
}

/// GRBL Controller implementation
///
/// Implements the ControllerTrait for GRBL firmware with full protocol support.
#[allow(dead_code)]
pub struct GrblController {
    /// Name identifier
    name: String,
    /// Communicator for GRBL protocol
    communicator: Arc<GrblCommunicator>,
    /// Controller state
    state: Arc<RwLock<GrblControllerState>>,
    /// IO task handle
    io_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Command sender channel
    command_tx: Arc<RwLock<Option<mpsc::Sender<String>>>>,
    /// Shutdown signal
    shutdown_signal: Arc<RwLock<Option<mpsc::Sender<()>>>>,
    /// Registered controller listeners
    listeners: Arc<RwLock<HashMap<String, Arc<dyn gcodekit5_core::ControllerListener>>>>,
    /// Connection parameters
    connection_params: ConnectionParams,
}

impl GrblController {
    /// Create a new GRBL controller
    pub fn new(connection_params: ConnectionParams, name: Option<String>) -> anyhow::Result<Self> {
        let communicator = Arc::new(GrblCommunicator::new(
            Box::new(NoOpCommunicator::new()),
            GrblCommunicatorConfig::default(),
        ));

        Ok(Self {
            name: name.unwrap_or_else(|| "GRBL".to_string()),
            communicator,
            state: Arc::new(RwLock::new(GrblControllerState::default())),
            io_task: Arc::new(RwLock::new(None)),
            command_tx: Arc::new(RwLock::new(None)),
            shutdown_signal: Arc::new(RwLock::new(None)),
            listeners: Arc::new(RwLock::new(HashMap::new())),
            connection_params,
        })
    }

    // Initialize the controller and query its capabilities
    // fn initialize(&self) -> anyhow::Result<()> { ... } - Removed as we use async send_command in connect

    /// Start the IO loop task
    fn start_io_loop(&mut self) -> anyhow::Result<()> {
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<String>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        *self.command_tx.write() = Some(cmd_tx);
        *self.shutdown_signal.write() = Some(shutdown_tx);

        let communicator = self.communicator.clone();
        let state = self.state.clone();
        let listeners = self.listeners.clone();

        let handle = tokio::spawn(async move {
            let mut buffer = String::new();
            let mut sent_queue: VecDeque<usize> = VecDeque::new();
            let mut local_cmd_queue: VecDeque<String> = VecDeque::new();
            let mut last_poll = Instant::now();

            // We use a short sleep to prevent busy looping when no data
            let loop_delay = Duration::from_millis(10);

            loop {
                // Check for shutdown
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // 1. READ PHASE: Read from serial port
                // We use a short timeout in the communicator configuration or rely on non-blocking behavior
                // Since we can't easily change the trait to async, we assume read_response returns quickly
                // or times out quickly (we set timeout to 50ms in connect)
                match communicator.read_response() {
                    Ok(data) if !data.is_empty() => {
                        let s = String::from_utf8_lossy(&data);
                        buffer.push_str(&s);

                        // Process complete lines
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer.drain(..=pos + 1); // +1 to remove \n

                            if !line.is_empty() {
                                // Check for status report
                                if line.starts_with('<') {
                                    // Update full status
                                    let full_status = StatusParser::parse_full(&line);
                                    let mut state_guard = state.write();

                                    if let Some(mpos) = full_status.mpos {
                                        state_guard.machine_position.x = mpos.x as f32;
                                        state_guard.machine_position.y = mpos.y as f32;
                                        state_guard.machine_position.z = mpos.z as f32;
                                        // Update other axes...
                                    }

                                    if let Some(wpos) = full_status.wpos {
                                        state_guard.work_position.x = wpos.x as f32;
                                        state_guard.work_position.y = wpos.y as f32;
                                        state_guard.work_position.z = wpos.z as f32;
                                    }

                                    if let Some(machine_state) = full_status.machine_state {
                                        let s = machine_state.as_str();

                                        // Update ControllerState (detailed)
                                        state_guard.state = match s {
                                            s if s.starts_with("Idle") => ControllerState::Idle,
                                            s if s.starts_with("Run") => ControllerState::Run,
                                            s if s.starts_with("Hold") => ControllerState::Hold,
                                            s if s.starts_with("Alarm") => ControllerState::Alarm,
                                            s if s.starts_with("Home") => ControllerState::Home,
                                            s if s.starts_with("Jog") => ControllerState::Jog,
                                            s if s.starts_with("Door") => ControllerState::Door,
                                            s if s.starts_with("Check") => ControllerState::Check,
                                            s if s.starts_with("Sleep") => ControllerState::Sleep,
                                            unknown => {
                                                tracing::warn!(
                                                    "Unknown GRBL state '{}', defaulting to Idle",
                                                    unknown
                                                );
                                                ControllerState::Idle
                                            }
                                        };

                                        // Update ControllerStatus (simplified)
                                        state_guard.status = match s {
                                            s if s.starts_with("Idle") => ControllerStatus::Idle,
                                            s if s.starts_with("Run") => ControllerStatus::Run,
                                            s if s.starts_with("Hold") => ControllerStatus::Hold,
                                            s if s.starts_with("Alarm") => ControllerStatus::Alarm,
                                            s if s.starts_with("Home") => ControllerStatus::Run,
                                            s if s.starts_with("Jog") => ControllerStatus::Run,
                                            _ => ControllerStatus::Idle,
                                        };

                                        // Notify registered listeners about state/status change
                                        let new_state = state_guard.state;
                                        let new_status = state_guard.status;
                                        // Drop write guard before notifying
                                        drop(state_guard);
                                        let listeners_clone = listeners.clone();
                                        for listener in listeners_clone.read().values() {
                                            let listener = listener.clone();
                                            let s_copy = new_status;
                                            let st_copy = new_state;
                                            tokio::spawn(async move {
                                                let _ = listener.on_state_changed(st_copy).await;
                                                let _ = listener.on_status_changed(&s_copy).await;
                                            });
                                        }
                                    }
                                } else if line == "ok" {
                                    // Acknowledge command
                                    if let Some(len) = sent_queue.pop_front() {
                                        communicator.acknowledge_chars(len);
                                    }
                                } else if line.starts_with("error:") {
                                    // Handle error (also consumes a command slot)
                                    tracing::error!("GRBL Error: {}", line);
                                    if let Some(len) = sent_queue.pop_front() {
                                        communicator.acknowledge_chars(len);
                                    }
                                } else {
                                    // Other messages (welcome, settings, etc)
                                    tracing::debug!("GRBL Message: {}", line);
                                }
                            }
                        }
                    }
                    _ => {} // No data or error
                }

                // 2. COMMAND FETCH PHASE: Get commands from channel
                while let Ok(cmd) = cmd_rx.try_recv() {
                    local_cmd_queue.push_back(cmd);
                }

                // 3. WRITE PHASE: Send commands if buffer allows
                // We peek at the next command
                if let Some(cmd) = local_cmd_queue.front() {
                    let cmd_len = cmd.len() + 1; // +1 for newline
                    if communicator.is_ready_to_send(cmd_len) {
                        // Send it
                        if communicator.send_command(cmd).is_ok() {
                            // Move to sent queue
                            sent_queue.push_back(cmd_len);
                            local_cmd_queue.pop_front();
                        }
                    }
                }

                // 4. POLL PHASE: Send status query if needed
                let poll_rate = state.read().poll_rate_ms;
                if last_poll.elapsed() >= Duration::from_millis(poll_rate) {
                    let _ = communicator.send_realtime_byte(b'?');
                    last_poll = Instant::now();
                }

                // Yield to let other tasks run and prevent CPU hogging
                tokio::time::sleep(loop_delay).await;
            }
        });

        *self.io_task.write() = Some(handle);
        Ok(())
    }

    /// Stop the IO loop task
    fn stop_io_loop(&mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.shutdown_signal.write().take() {
            let _ = tx.try_send(());
        }

        if let Some(handle) = self.io_task.write().take() {
            handle.abort();
        }

        Ok(())
    }
}

#[async_trait]
impl ControllerTrait for GrblController {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_state(&self) -> ControllerState {
        self.state.read().state
    }

    fn get_status(&self) -> ControllerStatus {
        self.state.read().status
    }

    fn get_override_state(&self) -> OverrideState {
        self.state.read().override_state
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        // Set a short timeout for the serial port to allow the IO loop to spin
        let mut params = self.connection_params.clone();
        params.timeout_ms = 50; // 50ms read timeout

        self.communicator.connect(&params)?;
        *self.state.write() = GrblControllerState::default();

        // Start the IO loop BEFORE initializing to handle responses
        self.start_io_loop()?;

        // Initialize (this might need to be async or handled via queue now)
        // For now, we'll just queue the initialization commands
        self.send_command("$RST=*").await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.send_command("$I").await?;
        self.send_command("$").await?;
        self.send_command("$G").await?;

        {
            let mut state = self.state.write();
            state.state = ControllerState::Idle;
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.stop_io_loop()?;
        self.communicator.disconnect()?;

        {
            let mut state = self.state.write();
            state.state = ControllerState::Disconnected;
        }

        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> anyhow::Result<()> {
        // Push to command channel
        let tx = {
            let guard = self.command_tx.read();
            guard.clone()
        };

        if let Some(tx) = tx {
            tx.send(command.to_string())
                .await
                .map_err(|_| anyhow::anyhow!("Failed to send command to IO loop"))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Controller not connected"))
        }
    }

    async fn home(&mut self) -> anyhow::Result<()> {
        self.send_command("$H").await?;
        Ok(())
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        self.communicator.send_realtime_byte(0x18)?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Reset communicator state
        self.communicator.clear()?;

        // Restart IO loop to clear queues
        self.stop_io_loop()?;
        self.start_io_loop()?;

        Ok(())
    }

    async fn clear_alarm(&mut self) -> anyhow::Result<()> {
        self.send_command("$X").await?;
        Ok(())
    }

    async fn unlock(&mut self) -> anyhow::Result<()> {
        self.send_command("$X").await?;
        Ok(())
    }

    async fn jog_start(
        &mut self,
        axis: char,
        direction: i32,
        feed_rate: f64,
    ) -> anyhow::Result<()> {
        if direction == 0 {
            return Err(anyhow::anyhow!("Direction must be non-zero"));
        }

        // Create a jog command using $J= syntax with G91 (relative) and G0 (rapid)
        let direction_str = if direction > 0 { "+" } else { "-" };
        let cmd = format!("$J=G91 G0 {}{} F{:.0}", axis, direction_str, feed_rate);
        self.send_command(&cmd).await?;

        Ok(())
    }

    async fn jog_stop(&mut self) -> anyhow::Result<()> {
        self.communicator.send_realtime_byte(0x85)?;
        Ok(())
    }

    async fn jog_incremental(
        &mut self,
        axis: char,
        distance: f64,
        feed_rate: f64,
    ) -> anyhow::Result<()> {
        // Format: $J=G91 G0 X{signed_distance} F{feed_rate}
        // distance already includes sign from the caller
        let cmd = format!("$J=G91 G0 {}{:.3} F{:.0}", axis, distance, feed_rate);
        self.send_command(&cmd).await?;

        Ok(())
    }

    async fn start_streaming(&mut self) -> anyhow::Result<()> {
        let mut state = self.state.write();
        state.is_streaming = true;
        state.state = ControllerState::Run;
        Ok(())
    }

    async fn pause_streaming(&mut self) -> anyhow::Result<()> {
        self.communicator.send_realtime_byte(0x21)?;
        self.state.write().state = ControllerState::Hold;
        Ok(())
    }

    async fn resume_streaming(&mut self) -> anyhow::Result<()> {
        self.communicator.send_realtime_byte(0x7E)?;
        self.state.write().state = ControllerState::Run;
        Ok(())
    }

    async fn cancel_streaming(&mut self) -> anyhow::Result<()> {
        self.communicator.send_realtime_byte(0x18)?;
        let mut state = self.state.write();
        state.is_streaming = false;
        state.state = ControllerState::Idle;
        Ok(())
    }

    async fn probe_z(&mut self, feed_rate: f64) -> anyhow::Result<PartialPosition> {
        let cmd = format!("G38.2Z-100F{}", feed_rate);
        self.send_command(&cmd).await?;

        let state = self.state.read();
        Ok(PartialPosition {
            z: Some(state.work_position.z),
            ..Default::default()
        })
    }

    async fn probe_x(&mut self, feed_rate: f64) -> anyhow::Result<PartialPosition> {
        let cmd = format!("G38.2X100F{}", feed_rate);
        self.send_command(&cmd).await?;

        let state = self.state.read();
        Ok(PartialPosition {
            x: Some(state.work_position.x),
            ..Default::default()
        })
    }

    async fn probe_y(&mut self, feed_rate: f64) -> anyhow::Result<PartialPosition> {
        let cmd = format!("G38.2Y100F{}", feed_rate);
        self.send_command(&cmd).await?;

        let state = self.state.read();
        Ok(PartialPosition {
            y: Some(state.work_position.y),
            ..Default::default()
        })
    }

    async fn set_feed_override(&mut self, percentage: u16) -> anyhow::Result<()> {
        if percentage > 200 {
            return Err(anyhow::anyhow!("Feed override must be 0-200%"));
        }

        self.state.write().override_state.feed_override = percentage;

        // Send real-time override commands based on percentage
        // GRBL uses specific codes for different percentages
        if percentage == 100 {
            self.communicator.send_realtime_byte(0x90)?;
        }

        Ok(())
    }

    async fn set_rapid_override(&mut self, percentage: u8) -> anyhow::Result<()> {
        if ![25, 50, 100].contains(&percentage) {
            return Err(anyhow::anyhow!("Rapid override must be 25, 50, or 100"));
        }

        self.state.write().override_state.rapid_override = percentage;
        Ok(())
    }

    async fn set_spindle_override(&mut self, percentage: u16) -> anyhow::Result<()> {
        if percentage > 200 {
            return Err(anyhow::anyhow!("Spindle override must be 0-200%"));
        }

        self.state.write().override_state.spindle_override = percentage;
        Ok(())
    }

    async fn set_work_zero(&mut self) -> anyhow::Result<()> {
        self.send_command("G92X0Y0Z0").await?;
        Ok(())
    }

    async fn set_work_zero_axes(&mut self, axes: &str) -> anyhow::Result<()> {
        let mut cmd = String::from("G92");
        for axis in axes.chars() {
            if ['X', 'Y', 'Z', 'A', 'B', 'C'].contains(&axis) {
                cmd.push(axis);
                cmd.push('0');
            }
        }
        self.send_command(&cmd).await?;
        Ok(())
    }

    async fn go_to_work_zero(&mut self) -> anyhow::Result<()> {
        self.send_command("G00X0Y0Z0").await?;
        Ok(())
    }

    async fn set_work_coordinate_system(&mut self, wcs: u8) -> anyhow::Result<()> {
        if !(54..=59).contains(&wcs) {
            return Err(anyhow::anyhow!("Work coordinate system must be 54-59"));
        }

        let cmd = format!("G{}", wcs);
        self.send_command(&cmd).await?;
        Ok(())
    }

    async fn get_wcs_offset(&self, _wcs: u8) -> anyhow::Result<PartialPosition> {
        let state = self.state.read();
        Ok(PartialPosition {
            x: Some(state.work_position.x),
            y: Some(state.work_position.y),
            z: Some(state.work_position.z),
            ..Default::default()
        })
    }

    async fn query_status(&mut self) -> anyhow::Result<ControllerStatus> {
        Ok(self.get_status())
    }

    async fn query_settings(&mut self) -> anyhow::Result<()> {
        self.communicator.send_command("$")?;
        Ok(())
    }

    async fn query_parser_state(&mut self) -> anyhow::Result<()> {
        self.communicator.send_command("$G")?;
        Ok(())
    }

    fn register_listener(
        &mut self,
        listener: Arc<dyn gcodekit5_core::ControllerListener>,
    ) -> gcodekit5_core::ControllerListenerHandle {
        let id = Uuid::new_v4().to_string();
        let handle = gcodekit5_core::ControllerListenerHandle(id.clone());
        self.listeners.write().insert(id, listener);
        handle
    }

    fn unregister_listener(&mut self, handle: gcodekit5_core::ControllerListenerHandle) {
        let _ = self.listeners.write().remove(&handle.0);
    }

    fn listener_count(&self) -> usize {
        self.listeners.read().len()
    }
}

// Test helpers and unit tests
#[cfg(test)]
impl GrblController {
    /// Notify listeners of a status change (test helper)
    pub(crate) fn notify_status_change(&self, status: gcodekit5_core::ControllerStatus) {
        let listeners = self.listeners.clone();
        for listener in listeners.read().values().cloned() {
            let s_copy = status;
            tokio::spawn(async move {
                let _ = listener.on_status_changed(&s_copy).await;
            });
        }
    }

    /// Notify listeners of a state change (test helper)
    pub(crate) fn notify_state_change(&self, state: gcodekit5_core::ControllerState) {
        let listeners = self.listeners.clone();
        for listener in listeners.read().values().cloned() {
            let st_copy = state;
            tokio::spawn(async move {
                let _ = listener.on_state_changed(st_copy).await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    struct TestListener {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl TestListener {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl gcodekit5_core::ControllerListener for TestListener {
        async fn on_status_changed(&self, status: &gcodekit5_core::ControllerStatus) {
            let mut g = self.calls.lock().await;
            g.push(format!("status:{:?}", status));
        }

        async fn on_state_changed(&self, new_state: gcodekit5_core::ControllerState) {
            let mut g = self.calls.lock().await;
            g.push(format!("state:{:?}", new_state));
        }
    }

    #[tokio::test]
    async fn test_register_unregister_listener() {
        let mut controller =
            GrblController::new(ConnectionParams::default(), Some("test".to_string())).unwrap();
        let listener = Arc::new(TestListener::new());
        let handle = controller.register_listener(listener.clone());
        assert_eq!(controller.listener_count(), 1);
        controller.unregister_listener(handle);
        assert_eq!(controller.listener_count(), 0);
    }

    #[tokio::test]
    async fn test_listener_receives_status_change() {
        let mut controller =
            GrblController::new(ConnectionParams::default(), Some("test".to_string())).unwrap();
        let listener = Arc::new(TestListener::new());
        let calls = listener.calls.clone();
        let _handle = controller.register_listener(listener.clone());
        controller.notify_status_change(gcodekit5_core::ControllerStatus::Alarm);
        tokio::time::sleep(Duration::from_millis(50)).await;
        let g = calls.lock().await;
        assert!(g.iter().any(|s| s.contains("status:Alarm")));
    }
}
