//! Tests for GRBL continuous jogging (streamed short increments).

use std::time::Duration;

use gcodekit5_communication::communication::ConnectionParams;
use gcodekit5_communication::firmware::grbl::{continuous_jog_increment, GrblController};
use gcodekit5_core::ControllerTrait;

#[test]
fn test_continuous_jog_increment_matches_one_tick_of_motion() {
    // 1200 mm/min = 20 mm/s; one 50 ms tick covers 1 mm.
    let step = continuous_jog_increment(1200.0, Duration::from_millis(50));
    assert!((step - 1.0).abs() < 1e-9, "step {step}");
}

#[test]
fn test_continuous_jog_increment_is_small_at_any_reasonable_feed() {
    // Even at 10 m/min an increment is well under a centimetre, so at a soft limit
    // the machine stops within one increment of the boundary.
    let step = continuous_jog_increment(10_000.0, Duration::from_millis(50));
    assert!(step < 10.0, "step {step}");
}

fn controller() -> GrblController {
    GrblController::new(ConnectionParams::default(), Some("test".to_string()))
        .expect("controller should construct")
}

#[tokio::test]
async fn test_continuous_jog_rejects_zero_direction() {
    assert!(controller().jog_start('X', 0, 500.0).await.is_err());
}

#[tokio::test]
async fn test_continuous_jog_rejects_non_positive_feed() {
    assert!(controller().jog_start('X', 1, 0.0).await.is_err());
    assert!(controller().jog_start('X', 1, f64::NAN).await.is_err());
}

#[tokio::test]
async fn test_continuous_jog_requires_connection() {
    assert!(controller().jog_start('Y', -1, 500.0).await.is_err());
}
