//! Tests for `gcodekit5_camtools::speeds_feeds::calculator` (moved from the inline test module).

use gcodekit5_camtools::speeds_feeds::*;
use gcodekit5_core::data::tools::ToolGeometry;

#[test]
fn test_operation_type_multipliers() {
    assert!(
        OperationType::Adaptive.feed_rate_multiplier()
            > OperationType::Slotting.feed_rate_multiplier()
    );
    assert_eq!(OperationType::Slotting.radial_engagement_percent(), 100.0);
    assert_eq!(OperationType::Profiling.radial_engagement_percent(), 30.0);
}

#[test]
fn test_geometry_adjustments() {
    let high_helix = ToolGeometry {
        helix_angle: Some(50.0),
        ..Default::default()
    };
    assert!(GeometryAdjustments::helix_factor(&high_helix) > 1.0);

    let low_engagement = GeometryAdjustments::radial_engagement_factor(20.0);
    let high_engagement = GeometryAdjustments::radial_engagement_factor(100.0);
    assert!(low_engagement > high_engagement);
}

#[test]
fn test_rpm_calculation() {
    let rpm = SpeedsFeedsCalculator::rpm_from_surface_speed(200.0, 6.0);
    // RPM = (200 * 1000) / (π * 6) ≈ 10,610
    assert!(rpm > 10000.0);
    assert!(rpm < 11000.0);
}
