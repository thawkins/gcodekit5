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
