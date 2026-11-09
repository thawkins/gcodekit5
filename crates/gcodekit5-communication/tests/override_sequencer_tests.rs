//! Tests for step-by-step GRBL feed/spindle override sequencing.

use gcodekit5_communication::firmware::grbl::{
    apply_override_step, is_valid_override_target, next_override_step, FEED_OVERRIDE_CODES,
    MAX_OVERRIDE_PERCENT, MIN_OVERRIDE_PERCENT, SPINDLE_OVERRIDE_CODES,
};

/// Drives `current` to `target` the way the controller/UI does: one step, then the
/// next step is chosen from the value GRBL would report.
fn simulate(mut current: u16, target: u16) -> (u16, Vec<u8>) {
    let mut sent = Vec::new();
    while let Some(byte) = next_override_step(current, target, FEED_OVERRIDE_CODES) {
        sent.push(byte);
        current = apply_override_step(current, byte, FEED_OVERRIDE_CODES);
        assert!(sent.len() <= 32, "sequence did not converge: {sent:?}");
    }
    (current, sent)
}

#[test]
fn test_override_sequencer_reaches_every_target_from_every_start() {
    for start in MIN_OVERRIDE_PERCENT..=MAX_OVERRIDE_PERCENT {
        for target in MIN_OVERRIDE_PERCENT..=MAX_OVERRIDE_PERCENT {
            let (reached, sent) = simulate(start, target);
            assert_eq!(reached, target, "start {start} target {target}");
            // Never more than a reset plus 9 x 10% and 9 x 1% steps.
            assert!(sent.len() <= 19, "start {start} target {target}: {sent:?}");
        }
    }
}

#[test]
fn test_override_sequencer_every_step_changes_reported_value() {
    // The controller waits for each step to show up in the Ov: report, so a step
    // that leaves the value unchanged would stall the sequence.
    for start in MIN_OVERRIDE_PERCENT..=MAX_OVERRIDE_PERCENT {
        for target in MIN_OVERRIDE_PERCENT..=MAX_OVERRIDE_PERCENT {
            let mut current = start;
            while let Some(byte) = next_override_step(current, target, FEED_OVERRIDE_CODES) {
                let next = apply_override_step(current, byte, FEED_OVERRIDE_CODES);
                assert_ne!(
                    next, current,
                    "start {start} target {target} byte {byte:#x}"
                );
                current = next;
            }
        }
    }
}

#[test]
fn test_override_sequencer_uses_coarse_then_fine_steps() {
    let (_, sent) = simulate(100, 123);
    assert_eq!(sent, vec![0x91, 0x91, 0x93, 0x93, 0x93]);
    let (_, sent) = simulate(100, 78);
    assert_eq!(sent, vec![0x92, 0x92, 0x94, 0x94]);
}

#[test]
fn test_override_sequencer_resets_when_shorter() {
    // From 190% to 105%: reset then +1 x5 (6 steps) beats -10 x8, -1 x5 (13 steps).
    assert_eq!(
        next_override_step(190, 105, FEED_OVERRIDE_CODES),
        Some(0x90)
    );
    assert_eq!(
        next_override_step(173, 100, SPINDLE_OVERRIDE_CODES),
        Some(0x99)
    );
}

#[test]
fn test_override_sequencer_done_when_at_target() {
    assert_eq!(next_override_step(150, 150, FEED_OVERRIDE_CODES), None);
}

#[test]
fn test_override_sequencer_spindle_codes() {
    assert_eq!(
        next_override_step(100, 120, SPINDLE_OVERRIDE_CODES),
        Some(0x9A)
    );
    assert_eq!(
        next_override_step(100, 95, SPINDLE_OVERRIDE_CODES),
        Some(0x9D)
    );
}

#[test]
fn test_override_sequencer_target_range() {
    assert!(!is_valid_override_target(0));
    assert!(!is_valid_override_target(9));
    assert!(is_valid_override_target(10));
    assert!(is_valid_override_target(200));
    assert!(!is_valid_override_target(201));
}

mod controller {
    use gcodekit5_communication::communication::ConnectionParams;
    use gcodekit5_communication::firmware::grbl::GrblController;
    use gcodekit5_core::ControllerTrait;

    fn controller() -> GrblController {
        GrblController::new(ConnectionParams::default(), Some("test".to_string()))
            .expect("controller should construct")
    }

    #[tokio::test]
    async fn test_override_controller_rejects_out_of_range_target() {
        let mut c = controller();
        assert!(c.set_feed_override(5).await.is_err());
        assert!(c.set_spindle_override(250).await.is_err());
    }

    #[tokio::test]
    async fn test_override_controller_no_step_when_already_at_target() {
        // The reported override starts at 100%, so nothing needs to be sent.
        let mut c = controller();
        assert!(c.set_feed_override(100).await.is_ok());
        assert_eq!(c.get_override_state().feed_override, 100);
    }

    #[tokio::test]
    async fn test_override_controller_does_not_claim_unconfirmed_value() {
        // With no controller connected the step can't be sent or confirmed by an Ov:
        // report: the call must fail rather than record the requested value as applied.
        let mut c = controller();
        assert!(c.set_feed_override(120).await.is_err());
        assert_eq!(c.get_override_state().feed_override, 100);
    }
}
