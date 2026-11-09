//! 6-Axis Position Tests
//!
//! Tests for 6-axis `CNCPoint` construction, accessors and unit conversion.

use gcodekit5_core::{CNCPoint, Units};

#[test]
fn test_cncpoint_6axis_creation() {
    let point = CNCPoint::with_axes(100.0, 50.0, 25.0, 90.0, 45.0, 0.0, Units::MM);
    assert_eq!(point.get_axes(), (100.0, 50.0, 25.0, 90.0, 45.0, 0.0));
    assert_eq!(point.unit, Units::MM);
}

#[test]
fn test_cncpoint_new_all_axes_zero() {
    let point = CNCPoint::new(Units::MM);
    assert_eq!(point.get_axes(), (0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
}

#[test]
fn test_cncpoint_default_is_mm_origin() {
    let point = CNCPoint::default();
    assert_eq!(point, CNCPoint::new(Units::MM));
}

#[test]
fn test_cncpoint_set_axes_6axis() {
    let mut point = CNCPoint::new(Units::MM);
    point.set_axes(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(point.get_axes(), (1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
}

#[test]
fn test_cncpoint_copy_6axis() {
    let original = CNCPoint::with_axes(100.0, 50.0, 25.0, 90.0, 45.0, 0.0, Units::MM);
    let copied = original;
    assert_eq!(copied, original);
}

#[test]
fn test_cncpoint_convert_to_inches_keeps_rotary_degrees() {
    let point = CNCPoint::with_axes(25.4, 50.8, 76.2, 90.0, 45.0, 180.0, Units::MM);
    let inches = point.convert_to(Units::INCH);
    assert!((inches.x - 1.0).abs() < 1e-9);
    assert!((inches.y - 2.0).abs() < 1e-9);
    assert!((inches.z - 3.0).abs() < 1e-9);
    assert_eq!((inches.a, inches.b, inches.c), (90.0, 45.0, 180.0));
    assert_eq!(inches.unit, Units::INCH);
}

#[test]
fn test_cncpoint_convert_round_trip() {
    let point = CNCPoint::with_axes(12.5, -3.0, 7.25, 30.0, -15.0, 270.0, Units::MM);
    let back = point.convert_to(Units::INCH).convert_to(Units::MM);
    let (x, y, z, a, b, c) = back.get_axes();
    assert!((x - 12.5).abs() < 1e-9 && (y + 3.0).abs() < 1e-9 && (z - 7.25).abs() < 1e-9);
    assert_eq!((a, b, c), (30.0, -15.0, 270.0));
}

#[test]
fn test_cncpoint_rotary_wraparound() {
    // Rotary angles may exceed one revolution or be negative
    let point = CNCPoint::with_axes(0.0, 0.0, 0.0, 720.0, -180.0, 1080.0, Units::MM);
    assert_eq!((point.a, point.b, point.c), (720.0, -180.0, 1080.0));
}
