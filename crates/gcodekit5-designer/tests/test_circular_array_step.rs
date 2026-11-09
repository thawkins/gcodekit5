//! Tests for circular array angular spacing.

use gcodekit5_designer::arrays::circular_array_step;

#[test]
fn test_circular_array_step_full_circle_has_count_gaps() {
    // 6 items around a full circle: the last gap closes the circle.
    assert!((circular_array_step(360.0, 6) - 60.0).abs() < 1e-9);
    assert!((circular_array_step(-360.0, 4) + 90.0).abs() < 1e-9);
}

#[test]
fn test_circular_array_step_partial_arc_ends_on_the_arc_end() {
    // 3 items over 90 degrees sit at 0, 45 and 90: two gaps.
    let step = circular_array_step(90.0, 3);
    assert!((step - 45.0).abs() < 1e-9);
    assert!(
        (step * 2.0 - 90.0).abs() < 1e-9,
        "last item must land on the arc end"
    );
}

#[test]
fn test_circular_array_step_degenerate_counts() {
    assert_eq!(circular_array_step(180.0, 0), 0.0);
    assert_eq!(circular_array_step(180.0, 1), 0.0);
}
