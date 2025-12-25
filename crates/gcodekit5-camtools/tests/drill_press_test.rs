use gcodekit5_camtools::drill_press::{DrillPressGenerator, DrillPressParameters};

#[test]
fn test_simple_drilling() {
    let params = DrillPressParameters {
        hole_diameter: 5.0,
        tool_diameter: 5.0,
        top_z: 0.0,
        bottom_z: -10.0,
        peck_depth: 0.0,
        plunge_rate: 100.0,
        feed_rate: 500.0,
        spindle_speed: 1000.0,
        safe_z: 5.0,
        x: 10.0,
        y: 20.0,
    };

    let generator = DrillPressGenerator::new(params);
    let gcode = generator.generate().unwrap();

    assert!(gcode.contains("G1 Z-10.000 F100.0"));
    assert!(gcode.contains("M3 S1000"));
    assert!(gcode.contains("G0 X10.000 Y20.000"));
}

#[test]
fn test_peck_drilling() {
    let params = DrillPressParameters {
        hole_diameter: 5.0,
        tool_diameter: 5.0,
        top_z: 0.0,
        bottom_z: -5.0,
        peck_depth: 2.0,
        plunge_rate: 100.0,
        feed_rate: 500.0,
        spindle_speed: 1000.0,
        safe_z: 5.0,
        x: 0.0,
        y: 0.0,
    };

    let generator = DrillPressGenerator::new(params);
    let gcode = generator.generate().unwrap();

    // Should have multiple plunges
    assert!(gcode.contains("G1 Z-2.000 F100.0"));
    assert!(gcode.contains("G1 Z-4.000 F100.0"));
    assert!(gcode.contains("G1 Z-5.000 F100.0"));
    // Should have retractions
    assert!(gcode.contains("G0 Z0.000 ; Retract to clear chips"));
}

#[test]
fn test_helical_drilling() {
    let params = DrillPressParameters {
        hole_diameter: 10.0,
        tool_diameter: 6.0,
        top_z: 0.0,
        bottom_z: -5.0,
        peck_depth: 2.0,
        plunge_rate: 100.0,
        feed_rate: 500.0,
        spindle_speed: 1000.0,
        safe_z: 5.0,
        x: 0.0,
        y: 0.0,
    };

    let generator = DrillPressGenerator::new(params);
    let gcode = generator.generate().unwrap();

    // Radius = (10 - 6) / 2 = 2.0
    assert!(gcode.contains("G0 X2.000 Y0.000"));
    // Helical moves
    assert!(gcode.contains("G2 X2.000 Y0.000 I-2.000 J0.0 Z-2.000 F500.0"));
    assert!(gcode.contains("G2 X2.000 Y0.000 I-2.000 J0.0 Z-4.000 F500.0"));
    assert!(gcode.contains("G2 X2.000 Y0.000 I-2.000 J0.0 Z-5.000 F500.0"));
    // Final circle
    assert!(gcode.contains("G2 X2.000 Y0.000 I-2.000 J0.0 F500.0"));
    // Return to center
    assert!(gcode.contains("G1 X0.000 Y0.000 F500.0"));
}
