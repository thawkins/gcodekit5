//! Connection watch timer
//!
//! Monitors the connection to the controller and detects timeouts.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::warn;

/// Connection watch configuration
#[derive(Debug, Clone)]
pub struct ConnectionWatchConfig {
    /// Timeout duration in milliseconds
    pub timeout_ms: u64,
    /// Check interval in milliseconds
    pub check_interval_ms: u64,
    /// Enable heartbeat (periodic status queries)
    pub enable_heartbeat: bool,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

impl Default for ConnectionWatchConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            check_interval_ms: 500,
            enable_heartbeat: true,
            heartbeat_interval_ms: 1000,
        }
    }
}

/// Connection watch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionWatchState {
    /// Connection is healthy
    Healthy,
    /// Connection may be timing out
    Degraded,
    /// Connection is lost
    Lost,
}

/// Connection watcher that monitors communication.
///
/// Timing uses a monotonic [`Instant`] so wall-clock changes (NTP, DST, manual
/// adjustment) cannot trigger or mask a timeout. The timeout window starts when
/// [`ConnectionWatcher::start`] is called, not when the watcher is constructed.
pub struct ConnectionWatcher {
    /// Configuration
    config: ConnectionWatchConfig,
    /// Monotonic reference point; heartbeats are stored as ms elapsed since it
    origin: Instant,
    /// Last heartbeat, in milliseconds since `origin`
    last_heartbeat: Arc<AtomicU64>,
    /// Watcher task handle
    watch_task: Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
    /// Current state
    state: Arc<tokio::sync::Mutex<ConnectionWatchState>>,
}

/// Milliseconds elapsed since `origin`, saturating at `u64::MAX`.
fn elapsed_ms(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_millis()).unwrap_or(u64::MAX)
}

impl ConnectionWatcher {
    /// Create a new connection watcher. No timing happens until [`Self::start`].
    pub fn new(config: ConnectionWatchConfig) -> Self {
        Self {
            config,
            origin: Instant::now(),
            last_heartbeat: Arc::new(AtomicU64::new(0)),
            watch_task: Arc::new(tokio::sync::Mutex::new(None)),
            state: Arc::new(tokio::sync::Mutex::new(ConnectionWatchState::Healthy)),
        }
    }

    /// Start watching the connection.
    ///
    /// Resets the heartbeat clock and state to healthy, so the timeout is measured
    /// from this call. Calling `start` again stops the previous watch task first,
    /// so at most one task ever runs.
    ///
    /// # Errors
    /// Currently infallible; returns `Result` for API stability.
    pub async fn start(&self) -> anyhow::Result<()> {
        // Hold the task slot for the whole restart so concurrent start() calls
        // cannot both spawn.
        let mut watch_task = self.watch_task.lock().await;
        if let Some(previous) = watch_task.take() {
            previous.abort();
        }

        self.heartbeat();
        *self.state.lock().await = ConnectionWatchState::Healthy;

        let config = self.config.clone();
        let origin = self.origin;
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let state = Arc::clone(&self.state);

        let task = tokio::spawn(async move {
            // tokio::time::interval panics on a zero period.
            let period = Duration::from_millis(config.check_interval_ms.max(1));
            let mut check_interval = interval(period);

            loop {
                check_interval.tick().await;

                let now = elapsed_ms(origin);
                let last = last_heartbeat.load(Ordering::Relaxed);
                let time_since_heartbeat = now.saturating_sub(last);

                let mut current_state = state.lock().await;
                let new_state = if time_since_heartbeat > config.timeout_ms {
                    ConnectionWatchState::Lost
                } else if time_since_heartbeat > config.timeout_ms / 2 {
                    ConnectionWatchState::Degraded
                } else {
                    ConnectionWatchState::Healthy
                };

                if *current_state != new_state {
                    if new_state == ConnectionWatchState::Lost {
                        warn!(
                            elapsed_ms = time_since_heartbeat,
                            timeout_ms = config.timeout_ms,
                            "Connection timeout detected"
                        );
                    }
                    *current_state = new_state;
                }
            }
        });

        *watch_task = Some(task);

        Ok(())
    }

    /// Stop watching the connection
    pub async fn stop(&self) {
        let mut watch_task = self.watch_task.lock().await;
        if let Some(task) = watch_task.take() {
            task.abort();
        }
    }

    /// Returns `true` while a watch task started by [`Self::start`] is running.
    pub async fn is_running(&self) -> bool {
        self.watch_task
            .lock()
            .await
            .as_ref()
            .is_some_and(|t| !t.is_finished())
    }

    /// Update the last heartbeat time
    pub fn heartbeat(&self) {
        self.last_heartbeat
            .store(elapsed_ms(self.origin), Ordering::Relaxed);
    }

    /// Get current connection state
    pub async fn get_state(&self) -> ConnectionWatchState {
        *self.state.lock().await
    }

    /// Check if connection is healthy
    pub async fn is_healthy(&self) -> bool {
        self.get_state().await == ConnectionWatchState::Healthy
    }

    /// Check if connection is lost
    pub async fn is_lost(&self) -> bool {
        self.get_state().await == ConnectionWatchState::Lost
    }

    /// Get time since last heartbeat (or since construction if never started) in
    /// milliseconds, measured on the monotonic clock.
    pub fn time_since_heartbeat(&self) -> u64 {
        let last = self.last_heartbeat.load(Ordering::Relaxed);
        elapsed_ms(self.origin).saturating_sub(last)
    }
}

impl Drop for ConnectionWatcher {
    fn drop(&mut self) {
        // Do not leave a detached task polling forever once the watcher is gone.
        if let Ok(mut slot) = self.watch_task.try_lock() {
            if let Some(task) = slot.take() {
                task.abort();
            }
        }
    }
}

impl std::fmt::Debug for ConnectionWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionWatcher")
            .field("config", &self.config)
            .field("time_since_heartbeat", &self.time_since_heartbeat())
            .finish()
    }
}
