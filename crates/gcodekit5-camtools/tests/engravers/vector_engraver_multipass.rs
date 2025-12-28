use gcodekit5_camtools::{VectorEngraver, VectorEngravingParameters};
use std::fs;

#[test]
fn test_multipass_generation() {
    let test_dir = std::env::temp_dir().join("gcodekit_tests");
    fs::create_dir_all(&test_dir).ok();
    let svg_path = test_dir.join("test_multipass.svg");

    let svg_content = r#"<?xml version="1.0"?>
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <path d="M 10 10 L 90 90"/>
</svg>"#;

    fs::write(&svg_path, svg_content).unwrap();

    let mut params = VectorEngravingParameters::default();
    params.multi_pass = true;
    params.num_passes = 3;
    params.z_step_down = 0.5;

    let engraver = VectorEngraver::from_file(&svg_path, params).unwrap();

    let gcode = engraver.generate_gcode().unwrap();

    // Verify multi-pass is implemented
    assert!(gcode.contains("Pass 2 of 3"), "Should have pass 2 comment");
    assert!(gcode.contains("Pass 3 of 3"), "Should have pass 3 comment");
    assert!(
        gcode.contains("Lower Z for next pass"),
        "Should have Z decrement between passes"
    );

    // Verify Z is lowered correctly between passes
    assert!(
        gcode.contains("G0 Z-0.50"),
        "Should lower Z by 0.50mm for pass 2"
    );
    assert!(
        gcode.contains("G0 Z-1.00"),
        "Should lower Z by 1.00mm for pass 3"
    );

    // Each pass should have laser commands
    let m3_count = gcode.matches("M3").count();
    assert!(
        m3_count >= 3,
        "Should have at least 3 M3 commands (one per pass)"
    );

    fs::remove_file(&svg_path).ok();
}

#[test]
fn test_single_pass_no_multipass() {
    let test_dir = std::env::temp_dir().join("gcodekit_tests");
    fs::create_dir_all(&test_dir).ok();
    let svg_path = test_dir.join("test_single_pass.svg");

    let svg_content = r#"<?xml version="1.0"?>
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <path d="M 10 10 L 90 90"/>
</svg>"#;

    fs::write(&svg_path, svg_content).unwrap();

    let params = VectorEngravingParameters::default();

    let engraver = VectorEngraver::from_file(&svg_path, params).unwrap();

    let gcode = engraver.generate_gcode().unwrap();

    // No pass comments when multi_pass is false
    assert!(
        !gcode.contains("Pass 1 of"),
        "Should have no pass comments when multi_pass is false"
    );
    assert!(
        !gcode.contains("Lower Z for next pass"),
        "Should not have Z decrement when single pass"
    );

    // Should still have laser commands
    let m3_count = gcode.matches("M3").count();
    assert!(m3_count >= 1, "Should have at least 1 M3 command");

    fs::remove_file(&svg_path).ok();
}

#[test]
fn test_laser_disabled_at_path_end() {
    let test_dir = std::env::temp_dir().join("gcodekit_tests");
    fs::create_dir_all(&test_dir).ok();
    let svg_path = test_dir.join("test_laser_end.svg");

    let svg_content = r#"<?xml version="1.0"?>
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <path d="M 10 10 L 90 90"/>
  <path d="M 20 20 L 80 80"/>
</svg>"#;

    fs::write(&svg_path, svg_content).unwrap();

    let params = VectorEngravingParameters::default();

    let engraver = VectorEngraver::from_file(&svg_path, params).unwrap();

    let gcode = engraver.generate_gcode().unwrap();

    // Count M3 (laser on) and M5 (laser off) commands
    let m3_count = gcode.matches("M3").count();
    let m5_count = gcode.matches("M5").count();

    // After each path, laser should be disabled with M5
    // M5 should be >= M3 to ensure laser is off after each path
    assert!(
        m5_count >= m3_count,
        "Laser should be disabled after each path"
    );

    fs::remove_file(&svg_path).ok();
}
