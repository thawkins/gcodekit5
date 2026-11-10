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

/// Number of holes and the largest hole's half-width in `shape`.
fn hole_stats(shape: &gcodekit5_designer::model::Shape) -> (usize, f64) {
    use gcodekit5_designer::model::DesignerShape;
    let mp = shape.as_csg().to_multipolygon();
    let holes: Vec<_> = mp.0.iter().flat_map(|p| p.interiors().iter()).collect();
    let half_width = holes
        .iter()
        .map(|ring| {
            let xs = ring.0.iter().map(|c| c.x);
            let (lo, hi) = xs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            });
            (hi - lo) / 2.0
        })
        .fold(0.0, f64::max);
    (holes.len(), half_width)
}

#[test]
fn test_offset_keeps_holes_and_moves_them() {
    // Audit C-04: offsetting a shape with a hole filled the hole in.
    use gcodekit5_designer::model::{DesignCircle, Point, Shape};
    use gcodekit5_designer::ops::{BooleanOp, perform_boolean, perform_offset};
    let outer = Shape::Circle(DesignCircle::new(Point::new(0.0, 0.0), 20.0));
    let inner = Shape::Circle(DesignCircle::new(Point::new(0.0, 0.0), 8.0));
    let donut = perform_boolean(&outer, &inner, BooleanOp::Difference);
    assert_eq!(hole_stats(&donut).0, 1);

    let (holes, r) = hole_stats(&perform_offset(&donut, 1.0));
    assert_eq!(holes, 1, "growing the shape must keep the hole");
    assert!((r - 7.0).abs() < 0.1, "hole radius {r}, expected 7");

    let (holes, r) = hole_stats(&perform_offset(&donut, -1.0));
    assert_eq!(holes, 1, "shrinking the shape must keep the hole");
    assert!((r - 9.0).abs() < 0.1, "hole radius {r}, expected 9");
}

/// A circle as polygon points.
fn circle_pts(r: f64) -> Vec<[f64; 2]> {
    (0..64)
        .map(|i| {
            let a = i as f64 / 64.0 * std::f64::consts::TAU;
            [r * a.cos(), r * a.sin()]
        })
        .collect()
}

#[test]
fn test_offset_keeps_an_island_inside_a_hole() {
    // Review of #85: cutting one polygon's hole removed an island processed
    // before it. The polygons are given in both orders.
    use csgrs::sketch::Sketch;
    use csgrs::traits::CSG;
    use gcodekit5_designer::model::{DesignPath, DesignerShape, Shape};
    use gcodekit5_designer::ops::perform_offset;
    let donut: Sketch<()> = Sketch::polygon(&circle_pts(20.0), None)
        .difference(&Sketch::polygon(&circle_pts(8.0), None));
    let island: Sketch<()> = Sketch::polygon(&circle_pts(3.0), None);
    for (first, second) in [(&island, &donut), (&donut, &island)] {
        let mut geometry = first.geometry.clone();
        geometry.0.extend(second.geometry.0.iter().cloned());
        let shape = Shape::Path(DesignPath::from_csg(Sketch::from_geo(geometry, None)));
        assert_eq!(
            shape.as_csg().to_multipolygon().0.len(),
            2,
            "donut + island"
        );
        let grown = perform_offset(&shape, 1.0);
        let polys = grown.as_csg().to_multipolygon().0.len();
        assert_eq!(polys, 2, "the island must survive the offset");
        assert_eq!(hole_stats(&grown).0, 1, "the donut keeps its hole");
    }
}
