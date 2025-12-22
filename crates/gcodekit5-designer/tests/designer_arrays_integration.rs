// Integration tests for Designer array operations (Phase 4.2)

use gcodekit5_designer::{
    ArrayGenerator, ArrayOperation, ArrayType, CircularArrayParams, GridArrayParams,
    LinearArrayParams, Point,
};

#[test]
fn test_linear_array_operation_creation() {
    let params = LinearArrayParams::new(3, 2, 10.0, 20.0);
    let operation = ArrayOperation::Linear(params);

    assert_eq!(operation.array_type(), ArrayType::Linear);
    assert!(operation.is_valid());
    assert_eq!(operation.total_copies(), 6);
}

#[test]
fn test_linear_array_single_row() {
    let params = LinearArrayParams::new(5, 1, 10.0, 0.0);
    let operation = ArrayOperation::Linear(params);

    assert_eq!(operation.total_copies(), 5);
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 5);
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (10.0, 0.0));
    assert_eq!(offsets[2], (20.0, 0.0));
    assert_eq!(offsets[3], (30.0, 0.0));
    assert_eq!(offsets[4], (40.0, 0.0));
}

#[test]
fn test_linear_array_grid_pattern() {
    let params = LinearArrayParams::new(3, 3, 10.0, 10.0);
    let operation = ArrayOperation::Linear(params);

    assert_eq!(operation.total_copies(), 9);
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 9);
    // Check first row
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (10.0, 0.0));
    assert_eq!(offsets[2], (20.0, 0.0));
    // Check second row
    assert_eq!(offsets[3], (0.0, 10.0));
    assert_eq!(offsets[4], (10.0, 10.0));
    assert_eq!(offsets[5], (20.0, 10.0));
    // Check third row
    assert_eq!(offsets[6], (0.0, 20.0));
    assert_eq!(offsets[7], (10.0, 20.0));
    assert_eq!(offsets[8], (20.0, 20.0));
}

#[test]
fn test_linear_array_rectangular_spacing() {
    let params = LinearArrayParams::new(2, 2, 15.0, 25.0);
    let operation = ArrayOperation::Linear(params);

    let result = ArrayGenerator::generate(&operation);
    let offsets = result.unwrap();

    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (15.0, 0.0));
    assert_eq!(offsets[2], (0.0, 25.0));
    assert_eq!(offsets[3], (15.0, 25.0));
}

#[test]
fn test_circular_array_basic() {
    let center = Point::new(50.0, 50.0);
    let params = CircularArrayParams::new(4, center, 30.0, 0.0, false);
    let operation = ArrayOperation::Circular(params);

    assert_eq!(operation.array_type(), ArrayType::Circular);
    assert!(operation.is_valid());
    assert_eq!(operation.total_copies(), 4);
}

#[test]
fn test_circular_array_offset_calculation() {
    let center = Point::new(0.0, 0.0);
    let params = CircularArrayParams::new(4, center, 10.0, 0.0, false);

    // First copy at 0 degrees
    let (x0, y0) = params.get_offset(0);
    assert_eq!(x0, 0.0);
    assert_eq!(y0, 0.0);

    // Second copy at 90 degrees should be near (0, 10)
    let (x1, y1) = params.get_offset(1);
    assert!((x1 - 0.0).abs() < 0.01); // cos(90°) ≈ 0
    assert!((y1 - 10.0).abs() < 0.01); // sin(90°) ≈ 1
}

#[test]
fn test_circular_array_multiple_sizes() {
    let center = Point::new(0.0, 0.0);

    // 6-copy array (360/6 = 60° each)
    let params6 = CircularArrayParams::new(6, center, 20.0, 0.0, false);
    assert_eq!(params6.angle_step(), 60.0);

    // 8-copy array (360/8 = 45° each)
    let params8 = CircularArrayParams::new(8, center, 20.0, 0.0, false);
    assert_eq!(params8.angle_step(), 45.0);

    // 12-copy array (360/12 = 30° each)
    let params12 = CircularArrayParams::new(12, center, 20.0, 0.0, false);
    assert_eq!(params12.angle_step(), 30.0);
}

#[test]
fn test_circular_array_clockwise_direction() {
    let center = Point::new(0.0, 0.0);
    let params_ccw = CircularArrayParams::new(4, center, 10.0, 0.0, false);
    let params_cw = CircularArrayParams::new(4, center, 10.0, 0.0, true);

    let offset_ccw_1 = params_ccw.get_offset(1);
    let offset_cw_1 = params_cw.get_offset(1);

    // Clockwise and counter-clockwise should have different signs for Y
    assert!(offset_ccw_1.1 > 0.0);
    assert!(offset_cw_1.1 < 0.0);
}

#[test]
fn test_circular_array_with_start_angle() {
    let center = Point::new(0.0, 0.0);
    let params_0 = CircularArrayParams::new(4, center, 10.0, 0.0, false);
    let params_90 = CircularArrayParams::new(4, center, 10.0, 90.0, false);

    let offset_0 = params_0.get_offset(1);
    let offset_90 = params_90.get_offset(1);

    // With different start angles, first copy should be at different positions
    assert!(offset_0 != offset_90);
}

#[test]
fn test_grid_array_operation_creation() {
    let params = GridArrayParams::new(4, 3, 15.0, 25.0);
    let operation = ArrayOperation::Grid(params);

    assert_eq!(operation.array_type(), ArrayType::Grid);
    assert!(operation.is_valid());
    assert_eq!(operation.total_copies(), 12);
}

#[test]
fn test_grid_array_single_column() {
    let params = GridArrayParams::new(1, 5, 10.0, 10.0);
    let operation = ArrayOperation::Grid(params);

    assert_eq!(operation.total_copies(), 5);
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 5);
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (0.0, 10.0));
    assert_eq!(offsets[2], (0.0, 20.0));
    assert_eq!(offsets[3], (0.0, 30.0));
    assert_eq!(offsets[4], (0.0, 40.0));
}

#[test]
fn test_grid_array_single_row() {
    let params = GridArrayParams::new(5, 1, 10.0, 10.0);
    let operation = ArrayOperation::Grid(params);

    assert_eq!(operation.total_copies(), 5);
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 5);
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (10.0, 0.0));
    assert_eq!(offsets[2], (20.0, 0.0));
    assert_eq!(offsets[3], (30.0, 0.0));
    assert_eq!(offsets[4], (40.0, 0.0));
}

#[test]
fn test_grid_array_rectangular_pattern() {
    let params = GridArrayParams::new(2, 3, 15.0, 20.0);
    let operation = ArrayOperation::Grid(params);

    assert_eq!(operation.total_copies(), 6);
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 6);
    // Row 0
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (15.0, 0.0));
    // Row 1
    assert_eq!(offsets[2], (0.0, 20.0));
    assert_eq!(offsets[3], (15.0, 20.0));
    // Row 2
    assert_eq!(offsets[4], (0.0, 40.0));
    assert_eq!(offsets[5], (15.0, 40.0));
}

#[test]
fn test_grid_array_bounds_calculation() {
    let params = GridArrayParams::new(3, 2, 10.0, 15.0);
    let original_bounds = (0.0, 0.0, 10.0, 10.0); // width=10, height=10

    let bounds = params.calculate_bounds(original_bounds);

    // Array width = 10 + (3-1)*10 = 30
    // Array height = 10 + (2-1)*15 = 25
    assert_eq!(bounds.0, 0.0); // min_x
    assert_eq!(bounds.1, 0.0); // min_y
    assert_eq!(bounds.2, 30.0); // max_x
    assert_eq!(bounds.3, 25.0); // max_y
}

#[test]
fn test_array_types_distinct() {
    let linear_op = ArrayOperation::Linear(LinearArrayParams::new(2, 2, 10.0, 10.0));
    let center = Point::new(0.0, 0.0);
    let circular_op =
        ArrayOperation::Circular(CircularArrayParams::new(4, center, 10.0, 0.0, false));
    let grid_op = ArrayOperation::Grid(GridArrayParams::new(2, 2, 10.0, 10.0));

    assert_ne!(linear_op.array_type(), circular_op.array_type());
    assert_ne!(linear_op.array_type(), grid_op.array_type());
    assert_ne!(circular_op.array_type(), grid_op.array_type());
}

#[test]
fn test_array_large_copies() {
    // Test with large arrays
    let large_linear = ArrayOperation::Linear(LinearArrayParams::new(10, 10, 5.0, 5.0));
    assert_eq!(large_linear.total_copies(), 100);

    let center = Point::new(0.0, 0.0);
    let large_circular =
        ArrayOperation::Circular(CircularArrayParams::new(50, center, 10.0, 0.0, false));
    assert_eq!(large_circular.total_copies(), 50);

    let large_grid = ArrayOperation::Grid(GridArrayParams::new(20, 20, 2.0, 2.0));
    assert_eq!(large_grid.total_copies(), 400);
}

#[test]
fn test_array_zero_spacing() {
    // Zero spacing should still work
    let params = LinearArrayParams::new(3, 3, 0.0, 0.0);
    let operation = ArrayOperation::Linear(params);

    assert!(operation.is_valid());
    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    // All offsets should be at origin when spacing is zero
    let offsets = result.unwrap();
    for offset in offsets {
        assert_eq!(offset, (0.0, 0.0));
    }
}

#[test]
fn test_circular_array_generation() {
    let center = Point::new(0.0, 0.0);
    let params = CircularArrayParams::new(8, center, 10.0, 0.0, false);
    let operation = ArrayOperation::Circular(params);

    let result = ArrayGenerator::generate(&operation);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 8);

    // First offset should always be zero
    assert_eq!(offsets[0], (0.0, 0.0));
}

#[test]
fn test_linear_array_with_different_spacing() {
    // Non-uniform spacing
    let params_x = LinearArrayParams::new(3, 1, 20.0, 0.0);
    let params_y = LinearArrayParams::new(1, 3, 0.0, 20.0);
    let params_xy = LinearArrayParams::new(2, 2, 15.0, 25.0);

    assert_eq!(params_x.total_copies(), 3);
    assert_eq!(params_y.total_copies(), 3);
    assert_eq!(params_xy.total_copies(), 4);

    let result_x = ArrayGenerator::generate_linear(&params_x);
    let result_y = ArrayGenerator::generate_linear(&params_y);
    let result_xy = ArrayGenerator::generate_linear(&params_xy);

    assert!(result_x.is_ok());
    assert!(result_y.is_ok());
    assert!(result_xy.is_ok());
}
