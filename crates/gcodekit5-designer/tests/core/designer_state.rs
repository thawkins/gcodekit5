use gcodekit5_designer::canvas::DrawingMode;
use gcodekit5_designer::designer_state::DesignerState;
// Point not used directly in this test file

#[test]
fn test_designer_state_new() {
    let state = DesignerState::new();
    assert_eq!(state.canvas.shape_count(), 0);
    assert!(!state.gcode_generated);
}

#[test]
fn test_set_mode() {
    let mut state = DesignerState::new();
    state.set_mode(1);
    assert_eq!(state.canvas.mode(), DrawingMode::Rectangle);

    state.set_mode(2);
    assert_eq!(state.canvas.mode(), DrawingMode::Circle);
}

#[test]
fn test_zoom() {
    let mut state = DesignerState::new();
    let initial = state.canvas.zoom();

    state.zoom_in();
    assert!(state.canvas.zoom() > initial);

    state.zoom_out();
    assert!(state.canvas.zoom() <= initial * 1.1);
}

#[test]
fn test_generate_gcode() {
    let mut state = DesignerState::new();
    state.canvas.add_rectangle(0.0, 0.0, 10.0, 10.0);

    let gcode = state.generate_gcode();
    assert!(!gcode.is_empty());
    assert!(state.gcode_generated);
    assert!(gcode.contains("G90"));
}
