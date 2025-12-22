use gcodekit5_designer::model::Point;
use gcodekit5_designer::vcarve::{VBitTool, VCarveGenerator, VCarveParams, VCarveSegment};

#[test]
fn test_vbit_creation() {
    let tool = VBitTool::new(90.0, 3.175, 2.25);
    assert_eq!(tool.tip_angle, 90.0);
    assert_eq!(tool.diameter, 3.175);
    assert_eq!(tool.cutting_length, 2.25);
}

#[test]
fn test_vbit_presets() {
    let v60 = VBitTool::v60(3.175);
    assert_eq!(v60.tip_angle, 60.0);

    let v90 = VBitTool::v90(3.175);
    assert_eq!(v90.tip_angle, 90.0);

    let v120 = VBitTool::v120(3.175);
    assert_eq!(v120.tip_angle, 120.0);
}

#[test]
fn test_vbit_validation() {
    let valid = VBitTool::new(90.0, 3.175, 2.25);
    assert!(valid.is_valid());

    let invalid_angle = VBitTool::new(0.0, 3.175, 2.25);
    assert!(!invalid_angle.is_valid());

    let invalid_diameter = VBitTool::new(90.0, 0.0, 2.25);
    assert!(!invalid_diameter.is_valid());
}

#[test]
fn test_depth_calculation_90_degree() {
    let tool = VBitTool::v90(3.175);
    let depth = tool.calculate_depth(1.0);

    // For 90° V-bit: depth = width / (2 * tan(45°)) = width / 2
    assert!((depth - 0.5).abs() < 0.01);
}

#[test]
fn test_depth_calculation_60_degree() {
    let tool = VBitTool::v60(3.175);
    let depth = tool.calculate_depth(1.0);

    // For 60° V-bit: depth = width / (2 * tan(30°)) ≈ width / 1.1547
    assert!((depth - 0.866).abs() < 0.01);
}

#[test]
fn test_max_cutting_width() {
    let tool = VBitTool::v90(3.175);
    let max_width = tool.max_cutting_width();

    // For 90° V-bit with cutting_length = 2.25: max_width ≈ 4.5
    assert!((max_width - 4.5).abs() < 0.1);
}

#[test]
fn test_radius_at_depth() {
    let tool = VBitTool::v90(3.175);
    let radius = tool.radius_at_depth(1.0);

    // For 90° V-bit at 1mm depth: radius = 1mm
    assert!((radius - 1.0).abs() < 0.01);
}

#[test]
fn test_vcarve_params_creation() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    assert_eq!(params.cutting_width, 1.0);
    assert_eq!(params.max_depth_per_pass, 0.5);
}

#[test]
fn test_vcarve_params_validation() {
    let tool = VBitTool::v90(3.175);
    let valid_params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    assert!(valid_params.is_valid());

    let invalid_width = VCarveParams::new(tool, 0.0, 0.5, 10000, 100.0);
    assert!(!invalid_width.is_valid());
}

#[test]
fn test_total_depth() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    let depth = params.total_depth();
    assert!((depth - 0.5).abs() < 0.01);
}

#[test]
fn test_passes_needed_single_pass() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    let passes = params.passes_needed();
    assert_eq!(passes, 1);
}

#[test]
fn test_passes_needed_multiple_passes() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.2, 10000, 100.0);

    let passes = params.passes_needed();
    assert_eq!(passes, 3);
}

#[test]
fn test_depth_per_pass() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.2, 10000, 100.0);

    let depth_per_pass = params.depth_per_pass();
    assert!((depth_per_pass - 0.166667).abs() < 0.01);
}

#[test]
fn test_vcarve_segment_creation() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(10.0, 0.0);
    let segment = VCarveSegment::new(p1, p2, 1.0, 0.5);

    assert_eq!(segment.start, p1);
    assert_eq!(segment.end, p2);
    assert_eq!(segment.depth, 1.0);
    assert_eq!(segment.radius, 0.5);
}

#[test]
fn test_vcarve_segment_length() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    let segment = VCarveSegment::new(p1, p2, 1.0, 0.5);

    assert_eq!(segment.length(), 5.0);
}

#[test]
fn test_calculate_depth() {
    let tool = VBitTool::v90(3.175);
    let result = VCarveGenerator::calculate_depth(&tool, 1.0);

    assert!(result.is_ok());
    let depth = result.unwrap();
    assert!((depth - 0.5).abs() < 0.01);
}

#[test]
fn test_calculate_depth_invalid() {
    let tool = VBitTool::v90(3.175);
    let result = VCarveGenerator::calculate_depth(&tool, 0.0);

    assert!(result.is_err());
}

#[test]
fn test_generate_passes_single() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
    ];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].len(), 2);
}

#[test]
fn test_generate_passes_multiple() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.2, 10000, 100.0);
    let path = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
    ];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 3);
}

#[test]
fn test_estimate_time() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];

    let result = VCarveGenerator::estimate_time(&params, &path);
    assert!(result.is_ok());

    let time = result.unwrap();
    // 100mm path / 100 mm/min = 1 minute
    assert!((time - 1.0).abs() < 0.01);
}

#[test]
fn test_validate_params_valid() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    let result = VCarveGenerator::validate_params(&params);
    assert!(result.is_ok());
}

#[test]
fn test_validate_params_invalid_width() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 10.0, 0.5, 10000, 100.0);

    let result = VCarveGenerator::validate_params(&params);
    assert!(result.is_err());
}
