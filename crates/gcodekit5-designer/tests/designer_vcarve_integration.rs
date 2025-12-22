// Integration tests for V-carving toolpath generation (Phase 4.3)

use gcodekit5_designer::vcarve::{VBitTool, VCarveParams};
use gcodekit5_designer::{Point, VCarveGenerator};

#[test]
fn test_vbit_60_degree() {
    let tool = VBitTool::v60(3.175);
    assert_eq!(tool.tip_angle, 60.0);
    assert!(tool.is_valid());
}

#[test]
fn test_vbit_90_degree() {
    let tool = VBitTool::v90(3.175);
    assert_eq!(tool.tip_angle, 90.0);
    assert!(tool.is_valid());
}

#[test]
fn test_vbit_120_degree() {
    let tool = VBitTool::v120(3.175);
    assert_eq!(tool.tip_angle, 120.0);
    assert!(tool.is_valid());
}

#[test]
fn test_depth_vs_cutting_width_90() {
    let tool = VBitTool::v90(3.175);

    // For 90° V-bit: depth = width / 2
    assert!((tool.calculate_depth(1.0) - 0.5).abs() < 0.01);
    assert!((tool.calculate_depth(2.0) - 1.0).abs() < 0.01);
    assert!((tool.calculate_depth(4.0) - 2.0).abs() < 0.01);
}

#[test]
fn test_depth_vs_cutting_width_60() {
    let tool = VBitTool::v60(3.175);

    // For 60° V-bit: depth ≈ width / 1.1547
    let depth_1 = tool.calculate_depth(1.0);
    assert!(depth_1 > 0.8 && depth_1 < 0.9);
}

#[test]
fn test_radius_at_different_depths_90() {
    let tool = VBitTool::v90(3.175);

    // For 90° V-bit: radius = depth (because tan(45°) = 1)
    assert!((tool.radius_at_depth(0.5) - 0.5).abs() < 0.01);
    assert!((tool.radius_at_depth(1.0) - 1.0).abs() < 0.01);
    assert!((tool.radius_at_depth(2.0) - 2.0).abs() < 0.01);
}

#[test]
fn test_max_cutting_width_90() {
    let tool = VBitTool::v90(3.175);
    let max_width = tool.max_cutting_width();

    // V90 with 2.25mm cutting_length should give ~4.5mm max width
    assert!(max_width > 4.0 && max_width < 5.0);
}

#[test]
fn test_vcarve_params_basic() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    assert!(params.is_valid());
    assert_eq!(params.cutting_width, 1.0);
    assert_eq!(params.spindle_speed, 10000);
}

#[test]
fn test_vcarve_total_depth_calculation() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 2.0, 0.5, 10000, 100.0);

    let total_depth = params.total_depth();
    // For 90° with 2.0mm cutting width: depth = 1.0mm
    assert!((total_depth - 1.0).abs() < 0.01);
}

#[test]
fn test_vcarve_single_pass_operation() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    // Single pass operation
    assert_eq!(params.passes_needed(), 1);
    let depth_per_pass = params.depth_per_pass();
    assert!((depth_per_pass - 0.5).abs() < 0.01);
}

#[test]
fn test_vcarve_multi_pass_operation() {
    let tool = VBitTool::v90(3.175);
    // 1mm total depth with 0.3mm per pass = 4 passes
    let params = VCarveParams::new(tool, 2.0, 0.3, 10000, 100.0);

    let passes = params.passes_needed();
    assert_eq!(passes, 4);

    let depth_per_pass = params.depth_per_pass();
    // Total 1mm / 4 passes = 0.25mm per pass
    assert!((depth_per_pass - 0.25).abs() < 0.01);
}

#[test]
fn test_vcarve_path_generation_simple() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].len(), 1);
}

#[test]
fn test_vcarve_path_generation_multi_segment() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_ok());

    let passes = result.unwrap();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].len(), 3); // 3 segments for 4 points
}

#[test]
fn test_vcarve_depth_values_in_segments() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];

    let passes = VCarveGenerator::generate_passes(&params, &path).unwrap();

    // Check that segments have correct depth
    for segment in &passes[0] {
        assert!((segment.depth - 0.5).abs() < 0.01);
        assert!((segment.radius - 0.5).abs() < 0.01);
    }
}

#[test]
fn test_vcarve_time_estimate_simple() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];

    let time = VCarveGenerator::estimate_time(&params, &path).unwrap();
    // 100mm / 100 mm/min = 1 minute (single pass)
    assert!((time - 1.0).abs() < 0.01);
}

#[test]
fn test_vcarve_time_estimate_multi_pass() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 2.0, 0.3, 10000, 100.0); // 4 passes
    let path = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)];

    let time = VCarveGenerator::estimate_time(&params, &path).unwrap();
    // 100mm * 4 passes / 100 mm/min = 4 minutes
    assert!((time - 4.0).abs() < 0.01);
}

#[test]
fn test_vcarve_validate_params_success() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);

    let result = VCarveGenerator::validate_params(&params);
    assert!(result.is_ok());
}

#[test]
fn test_vcarve_validate_params_width_too_large() {
    let tool = VBitTool::v90(3.175);
    // Max cutting width for V90 with 2.25mm cutting_length is ~4.5mm
    let params = VCarveParams::new(tool, 10.0, 0.5, 10000, 100.0);

    let result = VCarveGenerator::validate_params(&params);
    assert!(result.is_err());
}

#[test]
fn test_vcarve_cutting_angle_effects() {
    // Test how different V-bit angles affect depth for same cutting width
    let tool_60 = VBitTool::v60(3.175);
    let tool_90 = VBitTool::v90(3.175);
    let tool_120 = VBitTool::v120(3.175);

    let cutting_width = 1.0;

    let depth_60 = tool_60.calculate_depth(cutting_width);
    let depth_90 = tool_90.calculate_depth(cutting_width);
    let depth_120 = tool_120.calculate_depth(cutting_width);

    // Sharper angle (60°) gives deeper cuts than obtuse angle (120°)
    assert!(depth_60 > depth_90);
    assert!(depth_90 > depth_120);
}

#[test]
fn test_vcarve_large_cutting_width() {
    let tool = VBitTool::v90(3.175);
    let max_width = tool.max_cutting_width();

    // Should be able to use width close to max
    let params = VCarveParams::new(tool, max_width * 0.9, 0.5, 10000, 100.0);
    assert!(params.is_valid());
}

#[test]
fn test_vcarve_very_shallow_passes() {
    let tool = VBitTool::v90(3.175);
    // Very shallow per-pass depth requires many passes
    let params = VCarveParams::new(tool, 2.0, 0.1, 10000, 100.0); // 10 passes

    let passes = params.passes_needed();
    assert_eq!(passes, 10);
}

#[test]
fn test_vcarve_segment_length_calculation() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![
        Point::new(0.0, 0.0),
        Point::new(3.0, 4.0), // 5mm segment
    ];

    let passes = VCarveGenerator::generate_passes(&params, &path).unwrap();
    assert_eq!(passes[0][0].length(), 5.0);
}

#[test]
fn test_vcarve_empty_path_error() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_err());
}

#[test]
fn test_vcarve_single_point_path_error() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.5, 10000, 100.0);
    let path = vec![Point::new(0.0, 0.0)];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_err());
}

#[test]
fn test_vcarve_complex_path() {
    let tool = VBitTool::v90(3.175);
    let params = VCarveParams::new(tool, 1.0, 0.3, 10000, 100.0);

    // Create a more complex path (hexagon-like shape)
    let path = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(15.0, 5.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
        Point::new(-5.0, 5.0),
    ];

    let result = VCarveGenerator::generate_passes(&params, &path);
    assert!(result.is_ok());

    let passes = result.unwrap();
    // Should have 2 passes (0.5mm depth / 0.3mm per pass = 2)
    assert_eq!(passes.len(), 2);
    // Each pass should have 5 segments (6 points - 1)
    for pass in &passes {
        assert_eq!(pass.len(), 5);
    }
}
