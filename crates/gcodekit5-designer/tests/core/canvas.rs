use gcodekit5_designer::canvas::Canvas;
use gcodekit5_designer::shapes::Point;

#[test]
fn test_canvas_add_shapes() {
    let mut canvas = Canvas::new();
    let rect_id = canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);
    let circle_id = canvas.add_circle(Point::new(20.0, 20.0), 5.0);

    assert_eq!(canvas.shape_count(), 2);
    assert_ne!(rect_id, circle_id);
}

#[test]
fn test_canvas_select() {
    let mut canvas = Canvas::new();
    canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);

    let p = Point::new(5.0, 5.0);
    let selected = canvas.select_at(&p, 0.0, false);

    assert!(selected.is_some());
    assert_eq!(canvas.selected_id(), selected);
}

#[test]
fn test_canvas_zoom() {
    let mut canvas = Canvas::new();
    canvas.set_zoom(2.0);
    assert_eq!(canvas.zoom(), 2.0);

    canvas.set_zoom(0.05); // Out of range, should stay at 2.0
    assert_eq!(canvas.zoom(), 2.0);

    canvas.set_zoom(0.5); // Valid zoom
    assert_eq!(canvas.zoom(), 0.5);
}

#[test]
fn test_canvas_clear() {
    let mut canvas = Canvas::new();
    canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);
    canvas.clear();

    assert_eq!(canvas.shape_count(), 0);
    assert_eq!(canvas.selected_id(), None);
}

#[test]
fn test_resize_handle_sequence() {
    let mut canvas = Canvas::with_size(1200.0, 800.0);
    canvas.add_rectangle(0.0, 0.0, 100.0, 100.0);
    canvas.select_at(&Point::new(50.0, 50.0), 0.0, false);

    // Verify initial state
    let shapes: Vec<_> = canvas.shapes().collect();
    let shape = &shapes[0];
    let (x1, y1, x2, y2) = shape.shape.bounding_box();
    assert_eq!((x1, y1, x2, y2), (0.0, 0.0, 100.0, 100.0));

    // Drag bottom-left handle down by 20
    canvas.resize_selected(2, 0.0, 20.0);
    let shapes: Vec<_> = canvas.shapes().collect();
    let shape = &shapes[0];
    let (x1, y1, x2, y2) = shape.shape.bounding_box();
    assert_eq!((x1, y1, x2, y2), (0.0, 0.0, 100.0, 120.0));

    // Drag center handle by (10, 10)
    canvas.resize_selected(4, 10.0, 10.0);
    let shapes: Vec<_> = canvas.shapes().collect();
    let shape = &shapes[0];
    let (x1, y1, x2, y2) = shape.shape.bounding_box();
    // Expected: center was at (50, 60), moving by (10, 10) should give (60, 70)
    // Which means rect should be at (10, 10, 110, 130)
    assert_eq!((x1, y1, x2, y2), (10.0, 10.0, 110.0, 130.0));
}

#[test]
fn test_deselect_by_clicking_empty_space() {
    let mut canvas = Canvas::new();
    let rect_id = canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);

    // Select the rectangle
    let p = Point::new(5.0, 5.0);
    let selected = canvas.select_at(&p, 0.0, false);
    assert_eq!(selected, Some(rect_id));
    assert_eq!(canvas.selected_id(), Some(rect_id));

    // Click on empty space (far away from rectangle)
    let empty_point = Point::new(100.0, 100.0);
    let result = canvas.select_at(&empty_point, 0.0, false);

    // Should return None (no shape at that point)
    assert_eq!(result, None);
    // And selected_id should be None (deselected)
    assert_eq!(canvas.selected_id(), None);
}
