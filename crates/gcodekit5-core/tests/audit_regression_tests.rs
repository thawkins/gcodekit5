//! Regression tests for core fixes: imperial mixed-fraction sign handling,
//! G10 WCS index generation, validator robustness and event bus re-entrancy.

use std::sync::Arc;

use gcodekit5_core::event_bus::{
    AppEvent, ConnectionEvent, DisconnectReason, EventBus, EventFilter,
};
use gcodekit5_core::gcode::GcodeValidator;
use gcodekit5_core::probe::{ProbeReport, ProbeRoutine};
use gcodekit5_core::units::{parse_length, MeasurementSystem};
use gcodekit5_core::Position;

#[test]
fn test_units_negative_mixed_fraction() {
    let mm = parse_length("-1 1/2", MeasurementSystem::Imperial).expect("parse");
    assert!((mm - (-38.1)).abs() < 1e-4, "got {mm}");
}

#[test]
fn test_probe_g10_uses_wcs_index() {
    let mut report = ProbeReport::new(ProbeRoutine::BoreCenter {
        diameter: 10.0,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 20.0,
    });
    report.set_computed(Position::new(1.0, 2.0, 3.0));
    report.build_commands(55);
    assert_eq!(
        report.suggested_g10.as_deref(),
        Some("G10 L2 P2 X1.000 Y2.000 Z3.000")
    );
}

#[test]
fn test_validator_non_ascii_word_does_not_panic() {
    let mut validator = GcodeValidator::new();
    let report = validator.validate("G1 X10 ÉTAPE\nG1 Y5 (X9999 is a comment)");
    assert!(report.is_valid, "{report}");
}

#[test]
fn test_event_bus_handler_can_unsubscribe_during_publish() {
    let bus = Arc::new(EventBus::new());
    let bus_inner = Arc::clone(&bus);
    let id_slot: Arc<parking_lot::Mutex<Option<gcodekit5_core::event_bus::SubscriptionId>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let slot_inner = Arc::clone(&id_slot);
    let id = bus.subscribe(EventFilter::All, move |_| {
        if let Some(id) = *slot_inner.lock() {
            // Would deadlock if publish() held the handlers lock while calling us
            bus_inner.unsubscribe(id);
        }
    });
    *id_slot.lock() = Some(id);
    let _ = bus.publish(AppEvent::Connection(ConnectionEvent::Disconnected {
        port: "test".to_string(),
        reason: DisconnectReason::UserRequested,
    }));
    assert_eq!(bus.subscriber_count(), 0);
}

#[test]
fn test_validator_relative_moves_not_checked_as_absolute() {
    let mut validator = GcodeValidator::new(); // X limits 0..200
                                               // In G91, X-5 is an increment from an unknown position, not an absolute -5.
    let report = validator.validate("G21 G90\nG0 X10\nG91\nG1 X-5 F100\nG1 X-5");
    assert!(report.is_valid, "{report}");

    // Back in G90, absolute targets are checked again.
    let report = validator.validate("G91\nG1 X-5\nG90\nG1 X-5");
    assert_eq!(report.error_count(), 1, "{report}");
    assert_eq!(report.errors[0].0, 4);

    // An increment longer than the whole axis travel must leave the envelope.
    let report = validator.validate("G91 G1 X250 F100");
    assert!(!report.is_valid, "{report}");
}

#[test]
fn test_validator_g91_1_is_not_g91() {
    let mut validator = GcodeValidator::new();
    // G91.1 is incremental arc centres (IJK) and leaves G90 in effect.
    let report = validator.validate("G91.1\nG0 X-5");
    assert!(!report.is_valid, "{report}");
}

#[test]
fn test_validator_words_without_spaces() {
    let mut validator = GcodeValidator::new();
    assert!(validator.validate("G0X10Y20Z5").is_valid);

    let report = validator.validate("G1X10Y250F100");
    assert!(
        !report.is_valid,
        "Y250 must be caught without spaces: {report}"
    );

    let report = validator.validate("G1X10A45");
    assert!(!report.is_valid, "A axis on 3-axis machine: {report}");

    let report = validator.validate("G1X1F99999");
    assert!(!report.is_valid, "feed over max: {report}");

    let report = validator.validate("M3S99999");
    assert!(!report.is_valid, "spindle over max: {report}");
}

#[test]
fn test_validator_jog_and_offsets() {
    let mut validator = GcodeValidator::new();
    // Jog distance mode applies to the jog only, and system commands are skipped.
    let report = validator.validate("$J=G91 X-10 F500\n$H\n$110=5000\nG0 X-1");
    assert_eq!(report.error_count(), 1, "{report}");
    assert_eq!(
        report.errors[0].0, 4,
        "G90 remains in effect after a G91 jog"
    );

    // G10/G92 axis words are offsets, not targets.
    assert!(
        validator
            .validate("G10 L2 P1 X-300 Y-300\nG92 X-50")
            .is_valid
    );
}

#[test]
fn test_validator_inch_mode_converts_to_mm() {
    let mut validator = GcodeValidator::new();
    // 10 in = 254 mm, outside the 200 mm X travel.
    assert!(!validator.validate("G20\nG0 X10").is_valid);
    assert!(validator.validate("G20\nG0 X5\nG21\nG0 X150").is_valid);
}

#[test]
fn test_probe_g92_makes_probed_point_the_work_zero() {
    let mut report = ProbeReport::new(ProbeRoutine::BoreCenter {
        diameter: 10.0,
        safe_height: 5.0,
        fast_feed: 100.0,
        slow_feed: 20.0,
    });
    report.set_computed(Position::new(-100.0, -50.0, -20.0));

    // Without the tool's current position no G92 can be computed.
    report.build_commands(54);
    assert_eq!(report.suggested_g92, None);

    // Tool sits 5 mm above the probed point in Z and at the probed X/Y: the probed
    // point becomes work zero, so the current position is declared as (0, 0, 5).
    report.build_commands_with_current(54, Some(Position::new(-100.0, -50.0, -15.0)));
    assert_eq!(
        report.suggested_g92.as_deref(),
        Some("G92 X0.000 Y0.000 Z5.000")
    );
}
