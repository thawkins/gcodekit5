use gcodekit5_designer::designer_state::MachineMode;
use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_designer::shapes::OperationType;
use gcodekit5_designer::{DesignerState, Rectangle, Shape};

#[test]
fn new_shapes_inherit_default_properties_from_designer_state() {
    let mut state = DesignerState::new();
    state.tool_settings.machine_mode = MachineMode::Cnc3D;

    state.default_properties_shape.start_depth = 2.5;
    state.default_properties_shape.pocket_depth = 3.0;
    state.default_properties_shape.step_down = 0.25;
    state.default_properties_shape.step_in = 0.5;
    state.default_properties_shape.ramp_angle = 10.0;
    state.default_properties_shape.pocket_strategy = PocketStrategy::ContourParallel;
    state.default_properties_shape.raster_fill_ratio = 0.6;
    state.default_properties_shape.offset = 1.0;
    state.default_properties_shape.fillet = 2.0;
    state.default_properties_shape.chamfer = 3.0;
    state.default_properties_shape.use_custom_values = true;
    state.default_properties_shape.operation_type = OperationType::Pocket;

    let id = state.add_shape_with_undo(Shape::Rectangle(Rectangle::new(0.0, 0.0, 10.0, 10.0)));
    let created = state.canvas.get_shape(id).expect("new shape should be created");

    assert!((created.start_depth - 2.5).abs() < f64::EPSILON);
    assert!((created.pocket_depth - 3.0).abs() < f64::EPSILON);
    assert!((created.step_down as f64 - 0.25).abs() < f64::EPSILON);
    assert!((created.step_in as f64 - 0.5).abs() < f64::EPSILON);
    assert!((created.ramp_angle as f64 - 10.0).abs() < f64::EPSILON);
    assert!(created.use_custom_values);
    assert_eq!(created.operation_type, OperationType::Pocket);
}
