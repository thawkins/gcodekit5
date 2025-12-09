//! Tests for firmware::connection_watch

use gcodekit5_communication::firmware::connection_watch::*;
use std::time::Duration;

#[test]
fn test_connection_watcher_creation() {
    let watcher = ConnectionWatcher::new(ConnectionWatchConfig::default());
    assert_eq!(watcher.time_since_heartbeat() < 100, true);
}

#[tokio::test]
async fn test_heartbeat_update() {
    let watcher = ConnectionWatcher::new(ConnectionWatchConfig::default());
    let initial = watcher.time_since_heartbeat();

    tokio::time::sleep(Duration::from_millis(100)).await;
    let after_wait = watcher.time_since_heartbeat();
    assert!(after_wait >= initial);

    watcher.heartbeat();
    let after_heartbeat = watcher.time_since_heartbeat();
    assert!(after_heartbeat <= 10); // Should be very recent
}

#[tokio::test]
async fn test_connection_state_transitions() {
    let config = ConnectionWatchConfig {
        timeout_ms: 200,
        check_interval_ms: 50,
        enable_heartbeat: false,
        heartbeat_interval_ms: 1000,
    };
    let watcher = ConnectionWatcher::new(config);
    watcher.start().await.unwrap();

    // Initially healthy
    assert_eq!(watcher.get_state().await, ConnectionWatchState::Healthy);

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(watcher.get_state().await, ConnectionWatchState::Lost);

    // Heartbeat should reset state. Poll for state change to avoid timing races.
    watcher.heartbeat();
    let mut attempts = 0;
    let mut state = watcher.get_state().await;
    while state != ConnectionWatchState::Healthy && attempts < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        state = watcher.get_state().await;
        attempts += 1;
    }
    assert_eq!(state, ConnectionWatchState::Healthy);

    watcher.stop().await;
}
