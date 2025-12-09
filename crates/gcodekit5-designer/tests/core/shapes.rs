use gcodekit5_designer::shapes::{Circle, Ellipse, Line, Point, Rectangle, Shape};

#[test]
fn test_point_distance() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    assert_eq!(p1.distance_to(&p2), 5.0);
}

#[test]
fn test_rectangle_contains_point() {
    let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
    assert!(rect.contains_point(&Point::new(5.0, 5.0), 0.0));
    assert!(!rect.contains_point(&Point::new(15.0, 5.0), 0.0));
}

#[test]
fn test_circle_contains_point() {
    let circle = Circle::new(Point::new(0.0, 0.0), 5.0);
    assert!(circle.contains_point(&Point::new(3.0, 4.0), 0.0));
    assert!(!circle.contains_point(&Point::new(10.0, 0.0), 0.0));
}

#[test]
fn test_line_length() {
    let line = Line::new(Point::new(0.0, 0.0), Point::new(3.0, 4.0));
    assert_eq!(line.length(), 5.0);
}

#[test]
fn test_ellipse_contains_point() {
    let ellipse = Ellipse::new(Point::new(0.0, 0.0), 5.0, 3.0);
    assert!(ellipse.contains_point(&Point::new(0.0, 0.0), 0.0));
    assert!(ellipse.contains_point(&Point::new(4.0, 0.0), 0.0));
    assert!(!ellipse.contains_point(&Point::new(6.0, 0.0), 0.0));
}

#[test]
fn test_ellipse_bounding_box() {
    let ellipse = Ellipse::new(Point::new(10.0, 10.0), 5.0, 3.0);
    let (min_x, min_y, max_x, max_y) = ellipse.bounding_box();
    assert_eq!(min_x, 5.0);
    assert_eq!(min_y, 7.0);
    assert_eq!(max_x, 15.0);
    assert_eq!(max_y, 13.0);
}

/*
#[test]
fn test_polyline_regular() {
    let polyline = Polyline::regular(Point::new(0.0, 0.0), 10.0, 4);
    assert_eq!(polyline.vertices.len(), 4);
}

#[test]
fn test_polyline_bounding_box() {
    let polyline = Polyline::new(vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ]);
    let (min_x, min_y, max_x, max_y) = polyline.bounding_box();
    assert_eq!(min_x, 0.0);
    assert_eq!(min_y, 0.0);
    assert_eq!(max_x, 10.0);
    assert_eq!(max_y, 10.0);
}

#[test]
fn test_polyline_contains_point() {
    let polyline = Polyline::new(vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ]);
    assert!(polyline.contains_point(&Point::new(5.0, 5.0)));
    assert!(!polyline.contains_point(&Point::new(15.0, 5.0)));
}
*/
