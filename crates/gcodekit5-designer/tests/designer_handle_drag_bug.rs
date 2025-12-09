//! Test for handle drag position tracking bug fix
//! Verifies that dragging different handles uses current handle position,
//! not the position of the first handle used.

use gcodekit5_designer::{Canvas, Point};

#[test]
fn test_single_handle_resize() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // Resize from top-left corner (handle 0)
    canvas.resize_selected(0, 10.0, 10.0);

    // Rectangle should now be at (110, 110) to (200, 200)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!((x1 - 110.0).abs() < 0.1, "x1 should be 110, got {}", x1);
                assert!((y1 - 110.0).abs() < 0.1, "y1 should be 110, got {}", y1);
                assert!((x2 - 200.0).abs() < 0.1, "x2 should be 200, got {}", x2);
                assert!((y2 - 200.0).abs() < 0.1, "y2 should be 200, got {}", y2);
            }
        }
    }
}

#[test]
fn test_sequential_handle_drags() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // First: Resize from top-left corner (handle 0)
    canvas.resize_selected(0, 10.0, 10.0);

    // Verify rectangle is now at (110, 110) to (200, 200)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, _x2, _y2) = obj.shape.bounding_box();
                assert!((x1 - 110.0).abs() < 0.1);
                assert!((y1 - 110.0).abs() < 0.1);
            }
        }
    }

    // Second: Resize from bottom-right corner (handle 3)
    // This should use CURRENT position of bottom-right, not old one
    canvas.resize_selected(3, 10.0, 10.0);

    // Rectangle should now be at (110, 110) to (210, 210)
    // Bottom-right corner moved by (10, 10) from its current position (200, 200)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!(
                    (x1 - 110.0).abs() < 0.1,
                    "After 2nd drag: x1 should be 110, got {}",
                    x1
                );
                assert!(
                    (y1 - 110.0).abs() < 0.1,
                    "After 2nd drag: y1 should be 110, got {}",
                    y1
                );
                assert!(
                    (x2 - 210.0).abs() < 0.1,
                    "After 2nd drag: x2 should be 210, got {}",
                    x2
                );
                assert!(
                    (y2 - 210.0).abs() < 0.1,
                    "After 2nd drag: y2 should be 210, got {}",
                    y2
                );
            }
        }
    }
}

#[test]
fn test_top_right_handle_drag() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // Resize from top-right corner (handle 1)
    // Top-right is at (200, 100), moving it by (10, -10) should make it (210, 90)
    canvas.resize_selected(1, 10.0, -10.0);

    // Rectangle should now be at (100, 90) to (210, 200)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!((x1 - 100.0).abs() < 0.1, "x1 should be 100, got {}", x1);
                assert!((y1 - 90.0).abs() < 0.1, "y1 should be 90, got {}", y1);
                assert!((x2 - 210.0).abs() < 0.1, "x2 should be 210, got {}", x2);
                assert!((y2 - 200.0).abs() < 0.1, "y2 should be 200, got {}", y2);
            }
        }
    }
}

#[test]
fn test_bottom_left_handle_drag() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // Resize from bottom-left corner (handle 2)
    // Bottom-left is at (100, 200), moving it by (-10, 10) should make it (90, 210)
    canvas.resize_selected(2, -10.0, 10.0);

    // Rectangle should now be at (90, 100) to (200, 210)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!((x1 - 90.0).abs() < 0.1, "x1 should be 90, got {}", x1);
                assert!((y1 - 100.0).abs() < 0.1, "y1 should be 100, got {}", y1);
                assert!((x2 - 200.0).abs() < 0.1, "x2 should be 200, got {}", x2);
                assert!((y2 - 210.0).abs() < 0.1, "y2 should be 210, got {}", y2);
            }
        }
    }
}

#[test]
fn test_alternating_corner_drags_no_jump() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle at (100, 100) to (200, 200)
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // Drag top-left (0) by 5, 5
    canvas.resize_selected(0, 5.0, 5.0);
    // Should now be (105, 105) to (200, 200)

    // Drag bottom-right (3) by 5, 5
    canvas.resize_selected(3, 5.0, 5.0);
    // Should now be (105, 105) to (205, 205) - NOT jumping back to old position

    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!(
                    (x1 - 105.0).abs() < 0.1,
                    "No jump: x1 should be 105, got {}",
                    x1
                );
                assert!(
                    (y1 - 105.0).abs() < 0.1,
                    "No jump: y1 should be 105, got {}",
                    y1
                );
                assert!(
                    (x2 - 205.0).abs() < 0.1,
                    "No jump: x2 should be 205, got {}",
                    x2
                );
                assert!(
                    (y2 - 205.0).abs() < 0.1,
                    "No jump: y2 should be 205, got {}",
                    y2
                );
            }
        }
    }
}

#[test]
fn test_move_handle_drag() {
    let mut canvas = Canvas::new();

    // Add and select a rectangle
    canvas.add_rectangle(100.0, 100.0, 100.0, 100.0);
    canvas.select_at(&Point::new(150.0, 150.0), 0.0, false);

    // Drag center handle (4) to move the entire shape
    canvas.resize_selected(4, 20.0, 30.0);

    // Rectangle should now be at (120, 130) to (220, 230)
    if let Some(selected_id) = canvas.selected_id() {
        for obj in canvas.shapes() {
            if obj.id == selected_id {
                let (x1, y1, x2, y2) = obj.shape.bounding_box();
                assert!((x1 - 120.0).abs() < 0.1, "x1 should be 120, got {}", x1);
                assert!((y1 - 130.0).abs() < 0.1, "y1 should be 130, got {}", y1);
                assert!((x2 - 220.0).abs() < 0.1, "x2 should be 220, got {}", x2);
                assert!((y2 - 230.0).abs() < 0.1, "y2 should be 230, got {}", y2);
            }
        }
    }
}
