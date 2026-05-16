//! 6-Axis Jog Controller Tests
//!
//! Tests for 6-axis jog command generation and controller operations.

use gcodekit5_ui::ui::jog_controller::{JogController, JogDirection, JogStepSize};

#[test]
fn test_jog_controller_linear_axes() {
    let controller = JogController::new();
    controller.set_step_size(JogStepSize::One);
    controller.set_feed_rate(1000.0);

    // Test X positive jog
    let cmd = controller.generate_jog_command(JogDirection::XPos);
    assert!(cmd.contains("G91"));
    assert!(cmd.contains("X1"));
    assert!(cmd.contains("F1000"));

    // Test Y negative jog
    let cmd = controller.generate_jog_command(JogDirection::YNeg);
    assert!(cmd.contains("Y-1"));
}

#[test]
fn test_jog_controller_z_axis() {
    let controller = JogController::new();
    controller.set_step_size(JogStepSize::Ten);

    let cmd = controller.generate_jog_command(JogDirection::ZPos);
    assert!(cmd.contains("Z10"));

    let cmd = controller.generate_jog_command(JogDirection::ZNeg);
    assert!(cmd.contains("Z-10"));
}

#[test]
fn test_jog_controller_step_sizes() {
    let controller = JogController::new();
    controller.set_feed_rate(500.0);

    // Test different step sizes
    controller.set_step_size(JogStepSize::PointOne);
    let cmd = controller.generate_jog_command(JogDirection::XPos);
    assert!(cmd.contains("X0.1"));

    controller.set_step_size(JogStepSize::Ten);
    let cmd = controller.generate_jog_command(JogDirection::XPos);
    assert!(cmd.contains("X10"));

    controller.set_step_size(JogStepSize::Hundred);
    let cmd = controller.generate_jog_command(JogDirection::XPos);
    assert!(cmd.contains("X100"));
}

#[test]
fn test_jog_controller_axis_letter() {
    assert_eq!(JogDirection::XPos.axis(), 'X');
    assert_eq!(JogDirection::YNeg.axis(), 'Y');
    assert_eq!(JogDirection::ZPos.axis(), 'Z');
}

#[test]
fn test_jog_controller_is_positive_direction() {
    assert!(JogDirection::XPos.is_positive());
    assert!(!JogDirection::XNeg.is_positive());
    assert!(JogDirection::YPos.is_positive());
    assert!(!JogDirection::YNeg.is_positive());
    assert!(JogDirection::ZPos.is_positive());
    assert!(!JogDirection::ZNeg.is_positive());
}

#[test]
fn test_jog_controller_continuous_mode() {
    let controller = JogController::new();
    controller.set_continuous_mode(true);
    controller.set_feed_rate(2000.0);

    let cmd = controller.generate_jog_command(JogDirection::XPos);
    // Continuous jog should use feed rate without distance
    assert!(cmd.contains("F2000"));
}

#[test]
fn test_jog_controller_feed_rate_validation() {
    let controller = JogController::new();

    // Test minimum feed rate
    controller.set_feed_rate(0.0);
    assert_eq!(controller.get_feed_rate(), 1.0); // Should clamp to minimum

    // Test maximum feed rate
    controller.set_feed_rate(100000.0);
    assert_eq!(controller.get_feed_rate(), 50000.0); // Should clamp to maximum
}

#[test]
fn test_jog_direction_all_variants() {
    // Verify all directions are distinct
    let directions = vec![
        JogDirection::XPos,
        JogDirection::XNeg,
        JogDirection::YPos,
        JogDirection::YNeg,
        JogDirection::ZPos,
        JogDirection::ZNeg,
    ];

    // Check no duplicates
    let mut unique = std::collections::HashSet::new();
    for dir in &directions {
        assert!(unique.insert(dir.clone()), "Duplicate direction found");
    }
}
