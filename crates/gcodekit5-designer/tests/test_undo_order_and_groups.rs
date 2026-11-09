//! Undo/redo regression tests for draw order, group membership, multi-shape
//! transactions and property setters that previously bypassed the undo stack.

use gcodekit5_designer::commands::{DesignerCommand, RemoveShape};
use gcodekit5_designer::model::{DesignRectangle, DesignerShape, Shape};
use gcodekit5_designer::DesignerState;

/// Adds an `n`-shape design (10 mm squares spaced 20 mm apart) and returns the ids.
fn state_with_rects(n: usize) -> (DesignerState, Vec<u64>) {
    let mut state = DesignerState::new();
    let ids = (0..n)
        .map(|i| {
            let x = i as f64 * 20.0;
            state.add_shape_with_undo(Shape::Rectangle(DesignRectangle::new(x, 0.0, 10.0, 10.0)))
        })
        .collect();
    (state, ids)
}

fn draw_order(state: &DesignerState) -> Vec<u64> {
    state.canvas.shape_store.draw_order_iter().collect()
}

fn select_only(state: &mut DesignerState, ids: &[u64]) {
    state.canvas.deselect_all();
    for (i, id) in ids.iter().enumerate() {
        state.canvas.select_shape(*id, i > 0);
    }
}

fn group_of(state: &DesignerState, id: u64) -> Option<u64> {
    state.canvas.get_shape(id).expect("shape exists").group_id
}

#[test]
fn test_undo_delete_restores_middle_shape_draw_order() {
    let (mut state, ids) = state_with_rects(3);
    let before = draw_order(&state);

    select_only(&mut state, &[ids[1]]);
    state.delete_selected();
    assert_eq!(draw_order(&state), vec![ids[0], ids[2]]);

    state.undo();
    assert_eq!(draw_order(&state), before);

    state.redo();
    assert_eq!(draw_order(&state), vec![ids[0], ids[2]]);
    state.undo();
    assert_eq!(draw_order(&state), before);
}

#[test]
fn test_undo_delete_multiple_restores_order_in_one_step() {
    let (mut state, ids) = state_with_rects(4);
    let before = draw_order(&state);

    select_only(&mut state, &[ids[0], ids[2]]);
    state.delete_selected();
    assert_eq!(draw_order(&state), vec![ids[1], ids[3]]);

    // One undo step brings back every deleted shape at its original z-position.
    state.undo();
    assert_eq!(draw_order(&state), before);
}

#[test]
fn test_undo_remove_shape_command_restores_index() {
    let (mut state, ids) = state_with_rects(3);
    let before = draw_order(&state);

    state.push_command(DesignerCommand::RemoveShape(RemoveShape {
        id: ids[0],
        object: None,
    }));
    assert_eq!(draw_order(&state), vec![ids[1], ids[2]]);
    state.undo();
    assert_eq!(draw_order(&state), before);
}

#[test]
fn test_undo_group_restores_previous_group_ids() {
    let (mut state, ids) = state_with_rects(3);

    select_only(&mut state, &[ids[0], ids[1]]);
    state.group_selected();
    let first_group = group_of(&state, ids[0]);
    assert!(first_group.is_some());
    assert_eq!(group_of(&state, ids[1]), first_group);

    select_only(&mut state, &[ids[0], ids[1], ids[2]]);
    state.group_selected();
    let second_group = group_of(&state, ids[2]);
    assert!(second_group.is_some());
    assert_ne!(second_group, first_group);
    assert_eq!(group_of(&state, ids[0]), second_group);

    state.undo();
    assert_eq!(group_of(&state, ids[0]), first_group);
    assert_eq!(group_of(&state, ids[1]), first_group);
    assert_eq!(group_of(&state, ids[2]), None);

    state.redo();
    assert_eq!(group_of(&state, ids[0]), second_group);
    assert_eq!(group_of(&state, ids[2]), second_group);
}

#[test]
fn test_undo_set_selected_position_and_size_is_single_step() {
    let (mut state, ids) = state_with_rects(2);
    select_only(&mut state, &ids);
    let before: Vec<_> = ids
        .iter()
        .map(|id| state.canvas.get_shape(*id).expect("shape").shape.bounds())
        .collect();

    state.set_selected_x(50.0);
    let moved = state
        .canvas
        .get_shape(ids[0])
        .expect("shape")
        .shape
        .bounds();
    assert!((moved.0 - 50.0).abs() < 1e-9, "x not applied: {moved:?}");
    state.set_selected_width(5.0);
    let resized = state
        .canvas
        .get_shape(ids[0])
        .expect("shape")
        .shape
        .bounds();
    assert!(
        ((resized.2 - resized.0) - 5.0).abs() < 1e-9,
        "width: {resized:?}"
    );

    state.undo(); // width
    state.undo(); // x
    let after: Vec<_> = ids
        .iter()
        .map(|id| state.canvas.get_shape(*id).expect("shape").shape.bounds())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn test_undo_set_selected_y_and_height_record_undo() {
    let (mut state, ids) = state_with_rects(1);
    select_only(&mut state, &ids);

    state.set_selected_y(30.0);
    let b = state
        .canvas
        .get_shape(ids[0])
        .expect("shape")
        .shape
        .bounds();
    assert!((b.1 - 30.0).abs() < 1e-9, "y: {b:?}");
    state.set_selected_height(4.0);
    let b = state
        .canvas
        .get_shape(ids[0])
        .expect("shape")
        .shape
        .bounds();
    assert!(((b.3 - b.1) - 4.0).abs() < 1e-9, "height: {b:?}");

    state.undo();
    state.undo();
    let b = state
        .canvas
        .get_shape(ids[0])
        .expect("shape")
        .shape
        .bounds();
    assert_eq!(b, (0.0, 0.0, 10.0, 10.0));
}

#[test]
fn test_undo_noop_position_edit_records_nothing() {
    let (mut state, ids) = state_with_rects(1);
    // Discard the history of the shape creation so only the edit could be recorded.
    state.clear_history();
    select_only(&mut state, &ids);

    // Re-applying the current centre and size must not create an undo step.
    state.set_selected_position_and_size_with_flags(5.0, 5.0, 10.0, 10.0, true, true);
    assert!(!state.can_undo());
    state.set_selected_x(0.0);
    state.set_selected_rotation(0.0);
    assert!(!state.can_undo());
}

#[test]
fn test_undo_single_rotation_restores_and_keeps_index() {
    let (mut state, ids) = state_with_rects(1);
    select_only(&mut state, &ids);

    state.set_selected_rotation(30.0);
    assert!(
        (state
            .canvas
            .get_shape(ids[0])
            .expect("shape")
            .shape
            .rotation()
            - 30.0)
            .abs()
            < 1e-9
    );
    state.undo();
    assert!(
        state
            .canvas
            .get_shape(ids[0])
            .expect("shape")
            .shape
            .rotation()
            .abs()
            < 1e-9
    );

    // The spatial index must still find the shape at its location after undo.
    let point = gcodekit5_designer::model::Point::new(5.0, 5.0);
    let hit = state.canvas.select_at(&point, 0.5, false, false);
    assert_eq!(hit, Some(ids[0]));
}

#[test]
fn test_undo_multi_rotation_is_relative_and_single_step() {
    let (mut state, ids) = state_with_rects(2);
    select_only(&mut state, &ids);

    state.set_selected_rotation(45.0);
    for id in &ids {
        let rot = state.canvas.get_shape(*id).expect("shape").shape.rotation();
        assert!((rot - 45.0).abs() < 1e-9, "rotation {rot}");
    }
    state.undo();
    for id in &ids {
        let rot = state.canvas.get_shape(*id).expect("shape").shape.rotation();
        assert!(rot.abs() < 1e-9, "rotation {rot}");
    }
    assert!(state.can_undo(), "shape creation steps must remain");
}
