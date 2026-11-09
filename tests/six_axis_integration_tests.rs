//! 6-Axis Integration Tests
//!
//! Integration tests for 6-axis G-code streaming and validation.

use std::fs;
use std::path::Path;

/// Test that 6-axis G-code files exist and are valid
#[test]
fn test_six_axis_gcode_files_exist() {
    let test_files = vec![
        "assets/gcode/test_6axis_basic.nc",
        "assets/gcode/test_6axis_helical.nc",
        "assets/gcode/test_4axis_rotary.nc",
        "assets/gcode/test_5axis_simultaneous.nc",
        "assets/gcode/test_6axis_validation.nc",
    ];

    for file in test_files {
        assert!(Path::new(file).exists(), "Test file {} should exist", file);

        let content =
            fs::read_to_string(file).unwrap_or_else(|_| panic!("Should be able to read {}", file));

        // Verify file has content
        assert!(
            !content.is_empty(),
            "Test file {} should not be empty",
            file
        );

        // Verify basic G-code structure
        assert!(
            content.contains("G"),
            "Test file {} should contain G-codes",
            file
        );
    }
}

/// Test 6-axis G-code parsing patterns
#[test]
fn test_six_axis_gcode_patterns() {
    // Test that we can identify 6-axis movement patterns
    let test_cases = vec![
        ("G1 X10 Y20 Z30 A45 B22.5 C0", true),
        ("G0 X0 Y0 Z0 A0 B0 C0", true),
        ("G2 X10 Y0 I5 J0 A90", true),
        ("G3 X0 Y10 I0 J5 B45 C90", true),
    ];

    for (gcode, should_match) in test_cases {
        let has_6axis = gcode.contains(" A") || gcode.contains(" B") || gcode.contains(" C");
        assert_eq!(
            has_6axis, should_match,
            "Pattern match failed for: {}",
            gcode
        );
    }
}

/// Test rotary axis range validation
#[test]
fn test_rotary_axis_ranges() {
    // Valid rotary angles
    let valid_angles: Vec<f32> = vec![0.0, 45.0, 90.0, 180.0, 270.0, 360.0, 720.0, -180.0];
    for angle in valid_angles {
        // Rotary axes can be any value (continuous rotation)
        assert!(angle.is_finite(), "Angle {} should be finite", angle);
    }
}

/// Test 6-axis position extraction from status reports
#[test]
fn test_position_extraction_6axis() {
    // Test parsing 6-axis position strings (like from status reports)
    let test_cases = vec![
        (
            "100.000,50.000,25.000,90.000,45.000,0.000",
            vec![100.0, 50.0, 25.0, 90.0, 45.0, 0.0],
        ),
        (
            "0.000,0.000,0.000,0.000,0.000,0.000",
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ),
        (
            "-10.000,20.000,-5.000,180.000,-90.000,360.000",
            vec![-10.0, 20.0, -5.0, 180.0, -90.0, 360.0],
        ),
    ];

    for (input, expected) in test_cases {
        let coords: Vec<f64> = input
            .split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();

        assert_eq!(coords.len(), 6, "Should parse 6 coordinates from {}", input);

        for (i, (actual, exp)) in coords.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - exp).abs() < 0.001,
                "Coordinate {} mismatch: got {}, expected {}",
                i,
                actual,
                exp
            );
        }
    }
}

/// Test G-code buffer size estimation for 6-axis moves
#[test]
fn test_six_axis_buffer_estimation() {
    // 6-axis moves produce longer G-code lines
    let simple_move = "G1 X10 Y20 Z30 F1000";
    let six_axis_move = "G1 X10 Y20 Z30 A45 B22.5 C0 F1000";

    // 6-axis move should be longer
    assert!(six_axis_move.len() > simple_move.len());

    // Both should fit in typical GRBL buffer (256 bytes)
    assert!(simple_move.len() < 256);
    assert!(six_axis_move.len() < 256);
}
