use gcodekit5_designer::pocket_operations::{
    Island, PocketGenerator, PocketOperation, PocketStrategy,
};
use gcodekit5_designer::{Circle, Point, Rectangle};

fn min_start_distance_to_center(
    toolpaths: &[gcodekit5_designer::toolpath::Toolpath],
    center: Point,
) -> f64 {
    toolpaths
        .iter()
        .flat_map(|tp| tp.segments.iter())
        .flat_map(|seg| [seg.start, seg.end])
        .map(|p| p.distance_to(&center))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn test_pocket_operation_creation() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    assert_eq!(op.depth, -10.0);
    assert_eq!(op.tool_diameter, 3.175);
}

#[test]
fn test_pocket_operation_calculate_offset() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let offset1 = op.calculate_offset(1);
    let offset2 = op.calculate_offset(2);
    assert!(offset2 > offset1);
}

#[test]
fn test_island_contains_point() {
    let island = Island::new(Point::new(50.0, 50.0), 10.0);
    assert!(island.contains_point(&Point::new(50.0, 50.0)));
    assert!(island.contains_point(&Point::new(55.0, 50.0)));
    assert!(!island.contains_point(&Point::new(65.0, 50.0)));
}

#[test]
fn test_pocket_generator_rectangular() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let gen = PocketGenerator::new(op);
    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);

    let toolpaths = gen.generate_rectangular_pocket(&rect, 1.0);
    assert!(toolpaths.len() > 0);
    assert!(toolpaths[0].segments.len() > 0);
}

#[test]
fn test_pocket_generator_circular() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let gen = PocketGenerator::new(op);
    let circle = Circle {
        center: Point::new(50.0, 50.0),
        radius: 25.0,
        rotation: 0.0,
    };

    let toolpaths = gen.generate_circular_pocket(&circle, 1.0);
    assert!(toolpaths.len() > 0);
    assert!(toolpaths[0].segments.len() > 0);
}

#[test]
fn test_pocket_generator_with_islands() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let mut gen = PocketGenerator::new(op);
    gen.add_circular_island(Point::new(50.0, 50.0), 10.0);

    assert_eq!(gen.islands.len(), 1);
    assert!(gen.islands[0].contains_point(&Point::new(50.0, 50.0)));
    assert!(!gen.islands[0].contains_point(&Point::new(100.0, 100.0)));
}

#[test]
fn test_pocket_generator_offset_paths() {
    let op = PocketOperation::new("pocket1".to_string(), -10.0, 3.175);
    let gen = PocketGenerator::new(op);
    let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);

    let paths = gen.generate_offset_paths(&rect, 3);
    assert!(paths.len() > 0);
}

#[test]
fn test_contour_parallel_cleans_center() {
    let mut op = PocketOperation::new("pocket1".to_string(), -5.0, 4.0);
    op.set_strategy(PocketStrategy::ContourParallel);
    op.stepover = 1.5;
    let gen = PocketGenerator::new(op);
    let square = vec![
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        Point::new(20.0, 20.0),
        Point::new(0.0, 20.0),
    ];

    let toolpaths = gen.generate_polygon_pocket(&square, 5.0);
    assert!(!toolpaths.is_empty());

    let center = Point::new(10.0, 10.0);
    let min_dist = min_start_distance_to_center(&toolpaths, center);
    assert!(
        min_dist <= 0.25,
        "center cleanup missing, min dist {:.3}",
        min_dist
    );
}

#[test]
fn test_adaptive_cleans_center() {
    let mut op = PocketOperation::new("pocket1".to_string(), -5.0, 4.0);
    op.set_strategy(PocketStrategy::Adaptive);
    op.stepover = 1.5;
    let gen = PocketGenerator::new(op);
    let square = vec![
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        Point::new(20.0, 20.0),
        Point::new(0.0, 20.0),
    ];

    let toolpaths = gen.generate_polygon_pocket(&square, 5.0);
    assert!(!toolpaths.is_empty());

    let center = Point::new(10.0, 10.0);
    let min_dist = min_start_distance_to_center(&toolpaths, center);
    assert!(
        min_dist <= 0.25,
        "adaptive center cleanup missing, min dist {:.3}",
        min_dist
    );
}

#[test]
fn test_concave_union_cleanup() {
    // Hourglass/union shape that can leave a void between lobes without cleanup.
    let mut op = PocketOperation::new("pocket1".to_string(), -5.0, 4.0);
    op.set_strategy(PocketStrategy::ContourParallel);
    op.stepover = 1.8;
    let gen = PocketGenerator::new(op);
    let poly = vec![
        Point::new(-25.0, -35.0),
        Point::new(25.0, -35.0),
        Point::new(5.0, -5.0),
        Point::new(5.0, 5.0),
        Point::new(25.0, 35.0),
        Point::new(-25.0, 35.0),
        Point::new(-5.0, 5.0),
        Point::new(-5.0, -5.0),
    ];

    let toolpaths = gen.generate_polygon_pocket(&poly, 5.0);
    assert!(!toolpaths.is_empty());

    let waist = Point::new(0.0, 0.0);
    let min_dist = min_start_distance_to_center(&toolpaths, waist);
    assert!(
        min_dist <= 0.5,
        "void between lobes not cleaned, min dist {:.3}",
        min_dist
    );
}
