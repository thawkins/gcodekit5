use gcodekit5_designer::arrays::{
    ArrayGenerator, ArrayOperation, ArrayType, CircularArrayParams, GridArrayParams,
    LinearArrayParams,
};
use gcodekit5_designer::model::Point;

#[test]
fn test_linear_array_params_creation() {
    let params = LinearArrayParams::new(3, 2, 10.0, 20.0);
    assert_eq!(params.count_x, 3);
    assert_eq!(params.count_y, 2);
    assert_eq!(params.spacing_x, 10.0);
    assert_eq!(params.spacing_y, 20.0);
}

#[test]
fn test_linear_array_validation() {
    let valid = LinearArrayParams::new(3, 2, 10.0, 20.0);
    assert!(valid.is_valid());

    let invalid = LinearArrayParams::new(0, 2, 10.0, 20.0);
    assert!(!invalid.is_valid());

    let negative_spacing = LinearArrayParams::new(3, 2, -10.0, 20.0);
    assert!(!negative_spacing.is_valid());
}

#[test]
fn test_linear_array_total_copies() {
    let params = LinearArrayParams::new(3, 4, 10.0, 20.0);
    assert_eq!(params.total_copies(), 12);
}

#[test]
fn test_linear_array_bounds() {
    let params = LinearArrayParams::new(3, 2, 10.0, 20.0);
    let original_bounds = (0.0, 0.0, 5.0, 5.0);
    let bounds = params.calculate_bounds(original_bounds);

    assert_eq!(bounds.0, 0.0); // min_x
    assert_eq!(bounds.1, 0.0); // min_y
    assert_eq!(bounds.2, 25.0); // max_x = 5 + 2*10
    assert_eq!(bounds.3, 25.0); // max_y = 5 + 1*20
}

#[test]
fn test_circular_array_params_creation() {
    let center = Point::new(50.0, 50.0);
    let params = CircularArrayParams::new(8, center, 30.0, 0.0, false);
    assert_eq!(params.count, 8);
    assert_eq!(params.radius, 30.0);
}

#[test]
fn test_circular_array_validation() {
    let center = Point::new(50.0, 50.0);
    let valid = CircularArrayParams::new(8, center, 30.0, 0.0, false);
    assert!(valid.is_valid());

    let invalid_count = CircularArrayParams::new(0, center, 30.0, 0.0, false);
    assert!(!invalid_count.is_valid());

    let invalid_angle = CircularArrayParams::new(8, center, 30.0, 400.0, false);
    assert!(!invalid_angle.is_valid());
}

#[test]
fn test_circular_array_angle_step() {
    let center = Point::new(50.0, 50.0);
    let params = CircularArrayParams::new(4, center, 30.0, 0.0, false);
    assert_eq!(params.angle_step(), 90.0);

    let params8 = CircularArrayParams::new(8, center, 30.0, 0.0, false);
    assert_eq!(params8.angle_step(), 45.0);
}

#[test]
fn test_circular_array_offset_zero() {
    let center = Point::new(0.0, 0.0);
    let params = CircularArrayParams::new(4, center, 10.0, 0.0, false);
    let (x, y) = params.get_offset(0);
    assert_eq!(x, 0.0);
    assert_eq!(y, 0.0);
}

#[test]
fn test_grid_array_params_creation() {
    let params = GridArrayParams::new(5, 3, 15.0, 25.0);
    assert_eq!(params.columns, 5);
    assert_eq!(params.rows, 3);
    assert_eq!(params.column_spacing, 15.0);
    assert_eq!(params.row_spacing, 25.0);
}

#[test]
fn test_grid_array_validation() {
    let valid = GridArrayParams::new(5, 3, 15.0, 25.0);
    assert!(valid.is_valid());

    let invalid = GridArrayParams::new(0, 3, 15.0, 25.0);
    assert!(!invalid.is_valid());
}

#[test]
fn test_grid_array_total_copies() {
    let params = GridArrayParams::new(5, 4, 15.0, 25.0);
    assert_eq!(params.total_copies(), 20);
}

#[test]
fn test_grid_array_get_offset() {
    let params = GridArrayParams::new(3, 2, 10.0, 20.0);

    let offset00 = params.get_offset(0, 0);
    assert_eq!(offset00, Some((0.0, 0.0)));

    let offset10 = params.get_offset(1, 0);
    assert_eq!(offset10, Some((10.0, 0.0)));

    let offset01 = params.get_offset(0, 1);
    assert_eq!(offset01, Some((0.0, 20.0)));

    let offset11 = params.get_offset(1, 1);
    assert_eq!(offset11, Some((10.0, 20.0)));

    let out_of_bounds = params.get_offset(5, 5);
    assert_eq!(out_of_bounds, None);
}

#[test]
fn test_grid_array_bounds() {
    let params = GridArrayParams::new(3, 2, 10.0, 20.0);
    let original_bounds = (0.0, 0.0, 5.0, 5.0);
    let bounds = params.calculate_bounds(original_bounds);

    assert_eq!(bounds.0, 0.0); // min_x
    assert_eq!(bounds.1, 0.0); // min_y
    assert_eq!(bounds.2, 25.0); // max_x = 5 + 2*10
    assert_eq!(bounds.3, 25.0); // max_y = 5 + 1*20
}

#[test]
fn test_array_operation_enum() {
    let linear = ArrayOperation::Linear(LinearArrayParams::new(2, 2, 10.0, 10.0));
    assert_eq!(linear.array_type(), ArrayType::Linear);
    assert!(linear.is_valid());
    assert_eq!(linear.total_copies(), 4);

    let center = Point::new(0.0, 0.0);
    let circular = ArrayOperation::Circular(CircularArrayParams::new(6, center, 20.0, 0.0, false));
    assert_eq!(circular.array_type(), ArrayType::Circular);
    assert!(circular.is_valid());
    assert_eq!(circular.total_copies(), 6);

    let grid = ArrayOperation::Grid(GridArrayParams::new(3, 3, 10.0, 10.0));
    assert_eq!(grid.array_type(), ArrayType::Grid);
    assert!(grid.is_valid());
    assert_eq!(grid.total_copies(), 9);
}

#[test]
fn test_array_generator_linear() {
    let params = LinearArrayParams::new(2, 2, 10.0, 20.0);
    let result = ArrayGenerator::generate_linear(&params);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 4);
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (10.0, 0.0));
    assert_eq!(offsets[2], (0.0, 20.0));
    assert_eq!(offsets[3], (10.0, 20.0));
}

#[test]
fn test_array_generator_circular() {
    let center = Point::new(0.0, 0.0);
    let params = CircularArrayParams::new(4, center, 10.0, 0.0, false);
    let result = ArrayGenerator::generate_circular(&params);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 4);
}

#[test]
fn test_array_generator_grid() {
    let params = GridArrayParams::new(2, 3, 10.0, 20.0);
    let result = ArrayGenerator::generate_grid(&params);
    assert!(result.is_ok());

    let offsets = result.unwrap();
    assert_eq!(offsets.len(), 6);
    assert_eq!(offsets[0], (0.0, 0.0));
    assert_eq!(offsets[1], (10.0, 0.0));
    assert_eq!(offsets[2], (0.0, 20.0));
    assert_eq!(offsets[3], (10.0, 20.0));
    assert_eq!(offsets[4], (0.0, 40.0));
    assert_eq!(offsets[5], (10.0, 40.0));
}

#[test]
fn test_array_generator_main() {
    let linear = ArrayOperation::Linear(LinearArrayParams::new(2, 2, 10.0, 10.0));
    let result = ArrayGenerator::generate(&linear);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 4);
}

#[test]
fn test_invalid_linear_array() {
    let invalid = LinearArrayParams::new(0, 0, 10.0, 10.0);
    let result = ArrayGenerator::generate_linear(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_invalid_circular_array() {
    let center = Point::new(0.0, 0.0);
    let invalid = CircularArrayParams::new(0, center, 10.0, 0.0, false);
    let result = ArrayGenerator::generate_circular(&invalid);
    assert!(result.is_err());
}

#[test]
fn test_invalid_grid_array() {
    let invalid = GridArrayParams::new(0, 0, 10.0, 10.0);
    let result = ArrayGenerator::generate_grid(&invalid);
    assert!(result.is_err());
}
