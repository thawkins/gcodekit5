use gcodekit5_designer::drilling_patterns::{
    DrillOperation, DrillingPattern, DrillingPatternGenerator, PatternType,
};
use gcodekit5_designer::model::Point;

#[test]
fn test_drill_operation_creation() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    assert_eq!(op.hole_diameter, 6.35);
    assert_eq!(op.depth, -15.0);
}

#[test]
fn test_drill_operation_peck_drilling() {
    let mut op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    op.set_peck_drilling(5.0);

    let pecks = op.calculate_pecks();
    assert_eq!(pecks, 3);
}

#[test]
fn test_drilling_pattern_linear() {
    let pattern = DrillingPattern::linear(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 5);
    assert_eq!(pattern.hole_count(), 5);
    assert_eq!(pattern.holes[0], Point::new(0.0, 0.0));
    assert_eq!(pattern.holes[4], Point::new(100.0, 0.0));
}

#[test]
fn test_drilling_pattern_circular() {
    let pattern = DrillingPattern::circular(Point::new(50.0, 50.0), 25.0, 8);
    assert_eq!(pattern.hole_count(), 8);
}

#[test]
fn test_drilling_pattern_grid() {
    let pattern = DrillingPattern::grid(Point::new(0.0, 0.0), 10.0, 10.0, 5, 3);
    assert_eq!(pattern.hole_count(), 15);
}

#[test]
fn test_drilling_pattern_custom() {
    let points = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(20.0, 0.0),
    ];
    let pattern = DrillingPattern::custom(points);
    assert_eq!(pattern.hole_count(), 3);
}

#[test]
fn test_drilling_generator_linear() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_linear_pattern(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 5);
    assert_eq!(toolpath.segments.len(), 5);
}

#[test]
fn test_drilling_generator_circular() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_circular_pattern(Point::new(50.0, 50.0), 25.0, 8);
    assert_eq!(toolpath.segments.len(), 8);
}

#[test]
fn test_drilling_generator_grid() {
    let op = DrillOperation::new("drill1".to_string(), 6.35, 6.35, -15.0);
    let gen = DrillingPatternGenerator::new(op);

    let toolpath = gen.generate_grid_pattern(Point::new(0.0, 0.0), 10.0, 10.0, 5, 3);
    assert_eq!(toolpath.segments.len(), 15);
}

#[test]
fn test_drilling_pattern_type_names() {
    assert_eq!(PatternType::Linear.name(), "Linear");
    assert_eq!(PatternType::Circular.name(), "Circular");
    assert_eq!(PatternType::Grid.name(), "Grid");
}
