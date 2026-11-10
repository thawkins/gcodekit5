//! Regression tests for core fixes: imperial mixed-fraction sign handling,
//! G10/G92 work offsets from probe results.

use gcodekit5_core::Position;
use gcodekit5_core::probe::{ProbeReport, ProbeRoutine};
use gcodekit5_core::units::{MeasurementSystem, parse_length};

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
