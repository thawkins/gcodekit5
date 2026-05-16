//! 6-Axis Position Calculation Tests
//!
//! Tests for 6-axis position calculations and CNCPoint operations.

use gcodekit5_core::{CNCPoint, Units};

#[test]
fn test_cncpoint_6axis_creation() {
    let point = CNCPoint::with_axes(100.0, 50.0, 25.0, 90.0, 45.0, 0.0, Units::Mm);
    assert_eq!(point.x, 100.0);
    assert_eq!(point.y, 50.0);
    assert_eq!(point.z, 25.0);
    assert_eq!(point.a, 90.0);
    assert_eq!(point.b, 45.0);
    assert_eq!(point.c, 0.0);
}

#[test]
fn test_cncpoint_3axis_defaults() {
    let point = CNCPoint::new(10.0, 20.0, 30.0, Units::Mm);
    assert_eq!(point.x, 10.0);
    assert_eq!(point.y, 20.0);
    assert_eq!(point.z, 30.0);
    assert_eq!(point.a, 0.0);
    assert_eq!(point.b, 0.0);
    assert_eq!(point.c, 0.0);
}

#[test]
fn test_cncpoint_with_axes_partial() {
    let point = CNCPoint::with_axes(10.0, 20.0, 30.0, 45.0, 0.0, 0.0, Units::Mm);
    assert_eq!(point.x, 10.0);
    assert_eq!(point.y, 20.0);
    assert_eq!(point.z, 30.0);
    assert_eq!(point.a, 45.0);
    assert_eq!(point.b, 0.0);
    assert_eq!(point.c, 0.0);
}

#[test]
fn test_cncpoint_distance_6axis() {
    let p1 = CNCPoint::with_axes(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, Units::Mm);
    let p2 = CNCPoint::with_axes(10.0, 0.0, 0.0, 0.0, 0.0, 0.0, Units::Mm);
    let dist = p1.distance(&p2);
    assert!((dist - 10.0).abs() < 0.001);
}

#[test]
fn test_cncpoint_distance_with_rotary() {
    let p1 = CNCPoint::with_axes(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, Units::Mm);
    let p2 = CNCPoint::with_axes(3.0, 4.0, 0.0, 0.0, 0.0, 0.0, Units::Mm);
    let dist = p1.distance(&p2);
    assert!((dist - 5.0).abs() < 0.001);
}

#[test]
fn test_cncpoint_interpolate_6axis() {
    let p1 = CNCPoint::with_axes(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, Units::Mm);
    let p2 = CNCPoint::with_axes(100.0, 100.0, 100.0, 180.0, 90.0, 360.0, Units::Mm);
    let mid = p1.interpolate(&p2, 0.5);

    assert!((mid.x - 50.0).abs() < 0.001);
    assert!((mid.y - 50.0).abs() < 0.001);
    assert!((mid.z - 50.0).abs() < 0.001);
    assert!((mid.a - 90.0).abs() < 0.001);
    assert!((mid.b - 45.0).abs() < 0.001);
    assert!((mid.c - 180.0).abs() < 0.001);
}

#[test]
fn test_cncpoint_is_at_position_6axis() {
    let p1 = CNCPoint::with_axes(10.0, 20.0, 30.0, 45.0, 22.5, 90.0, Units::Mm);
    let p2 = CNCPoint::with_axes(10.0, 20.0, 30.0, 45.0, 22.5, 90.0, Units::Mm);
    assert!(p1.is_at_position(&p2, 0.001));
}

#[test]
fn test_cncpoint_is_at_position_with_tolerance() {
    let p1 = CNCPoint::with_axes(10.0, 20.0, 30.0, 45.0, 22.5, 90.0, Units::Mm);
    let p2 = CNCPoint::with_axes(10.01, 20.01, 30.01, 45.01, 22.51, 90.01, Units::Mm);
    assert!(p1.is_at_position(&p2, 0.02));
    assert!(!p1.is_at_position(&p2, 0.001));
}

#[test]
fn test_cncpoint_clone_6axis() {
    let original = CNCPoint::with_axes(100.0, 50.0, 25.0, 90.0, 45.0, 0.0, Units::Mm);
    let cloned = original;
    assert_eq!(cloned.x, original.x);
    assert_eq!(cloned.y, original.y);
    assert_eq!(cloned.z, original.z);
    assert_eq!(cloned.a, original.a);
    assert_eq!(cloned.b, original.b);
    assert_eq!(cloned.c, original.c);
}

#[test]
fn test_cncpoint_to_inches() {
    let point = CNCPoint::with_axes(25.4, 50.8, 76.2, 90.0, 45.0, 0.0, Units::Mm);
    let inches = point.to_inches();
    assert!((inches.x - 1.0).abs() < 0.001);
    assert!((inches.y - 2.0).abs() < 0.001);
    assert!((inches.z - 3.0).abs() < 0.001);
    // Rotary axes remain in degrees
    assert!((inches.a - 90.0).abs() < 0.001);
}

#[test]
fn test_cncpoint_rotary_wraparound() {
    // Test that rotary angles can exceed 360 degrees
    let point = CNCPoint::with_axes(0.0, 0.0, 0.0, 720.0, -180.0, 1080.0, Units::Mm);
    assert_eq!(point.a, 720.0);
    assert_eq!(point.b, -180.0);
    assert_eq!(point.c, 1080.0);
}
