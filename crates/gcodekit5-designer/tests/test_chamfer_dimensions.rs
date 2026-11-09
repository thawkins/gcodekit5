use gcodekit5_designer::model::DesignerShape;
use gcodekit5_designer::ops::perform_chamfer;
use gcodekit5_designer::{Rectangle, Shape};

#[test]
fn rectangle_chamfer_uses_requested_edge_length() {
    let rect = Rectangle::new(0.0, 0.0, 100.0, 50.0);
    let shape = Shape::Rectangle(rect);

    let chamfered = perform_chamfer(&shape, 10.0);

    let points = collect_path_points(&chamfered);
    assert!(
        !points.is_empty(),
        "Chamfered shape should produce path points"
    );

    let min_x = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_y = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    let edge_eps = 1e-2;
    let top_points: Vec<_> = points
        .iter()
        .copied()
        .filter(|(_, y)| (y - max_y).abs() <= edge_eps)
        .collect();
    let left_points: Vec<_> = points
        .iter()
        .copied()
        .filter(|(x, _)| (x - min_x).abs() <= edge_eps)
        .collect();

    assert!(
        !top_points.is_empty() && !left_points.is_empty(),
        "Expected points on top and left edges after chamfer"
    );

    let min_x_on_top = top_points
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::INFINITY, f64::min);
    let max_y_on_left = left_points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    let top_offset = min_x_on_top - min_x;
    let left_offset = max_y - max_y_on_left;

    // For a 90-degree corner, the chamfer leg on each edge should match the input distance.
    let tol = 0.5;
    assert!(
        (top_offset - 10.0).abs() <= tol,
        "Top edge chamfer offset was {top_offset}, expected ~10.0"
    );
    assert!(
        (left_offset - 10.0).abs() <= tol,
        "Left edge chamfer offset was {left_offset}, expected ~10.0"
    );
}

fn collect_path_points(shape: &Shape) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let path = shape.render();

    for event in path.iter() {
        match event {
            lyon::path::Event::Begin { at } => points.push((at.x as f64, at.y as f64)),
            lyon::path::Event::Line { to, .. } => points.push((to.x as f64, to.y as f64)),
            lyon::path::Event::Quadratic { to, .. } => points.push((to.x as f64, to.y as f64)),
            lyon::path::Event::Cubic { to, .. } => points.push((to.x as f64, to.y as f64)),
            lyon::path::Event::End { .. } => {}
        }
    }

    points
}
