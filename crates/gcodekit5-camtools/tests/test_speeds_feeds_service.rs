//! Tests for `gcodekit5_camtools::speeds_feeds::service` (moved from the inline test module).

use gcodekit5_camtools::speeds_feeds::*;
use gcodekit5_core::data::tools::ToolType;
use gcodekit5_devicedb::model::DeviceProfile;

#[test]
fn test_tool_service_filters() {
    let service = SpeedsFeedsToolService::new()
        .with_diameter_range(3.0, 12.0)
        .with_tool_types(vec![ToolType::EndMillFlat, ToolType::EndMillBall]);

    assert_eq!(service.min_diameter, Some(3.0));
    assert_eq!(service.max_diameter, Some(12.0));
    assert_eq!(service.tool_types.len(), 2);
}

#[test]
fn test_material_service_filters() {
    let service = SpeedsFeedsMaterialService::new()
        .with_machinability_range(5, 8)
        .with_cutting_params_for("endmill_flat");

    assert_eq!(service.min_machinability, Some(5));
    assert_eq!(service.max_machinability, Some(8));
    assert_eq!(service.has_cutting_params, Some("endmill_flat".to_string()));
}

#[test]
fn test_device_service_limits() {
    let device = DeviceProfile::default();
    let service = SpeedsFeedsDeviceService::new(device);
    let limits = service.get_limits();

    assert!(limits.max_rpm > 0);
    assert!(limits.max_feed_rate > 0.0);
}

#[test]
fn test_lookup_table() {
    let mut table = FeedsSpeedsLookupTable::new();
    assert!(table.is_empty());

    // Create a dummy result - need to create minimal CalculationResult
    // We can't easily create one here without importing, so just test the basic operations
    assert_eq!(table.len(), 0);
    table.clear();
    assert!(table.is_empty());
}
