//! Regression tests for editor undo/redo and viewport handling.

use gcodekit5_gcodeeditor::{EditorState, TextChange, Viewport};

#[test]
fn test_undo_inverse_uses_char_count() {
    let change = TextChange::new(2..2, String::new(), "°°".to_string(), 2, 4);
    let inverse = change.inverse();
    assert_eq!(inverse.char_range, 2..4);
}

#[test]
fn test_editor_undo_redo_restores_text() {
    let mut editor = EditorState::new(400.0, 20.0);
    editor.load_text("G1 X1");
    editor.buffer_mut().insert(5, " ; 90°");
    editor.undo_manager_mut().record(TextChange::new(
        5..5,
        String::new(),
        " ; 90°".to_string(),
        5,
        11,
    ));

    assert!(editor.undo());
    assert_eq!(editor.get_text(), "G1 X1");
    assert!(editor.redo());
    assert_eq!(editor.get_text(), "G1 X1 ; 90°");
}

#[test]
fn test_viewport_zero_height_scroll_does_not_panic() {
    let mut viewport = Viewport::new(0.0, 20.0);
    viewport.set_total_lines(100);
    viewport.scroll_to_line(50);
    assert!(viewport.start_line <= viewport.end_line);
}

#[test]
fn test_viewport_clamps_scroll_when_document_shrinks() {
    let mut viewport = Viewport::new(100.0, 10.0);
    viewport.set_total_lines(1000);
    viewport.set_scroll_offset(900);
    viewport.set_total_lines(20);
    assert!(viewport.start_line <= viewport.end_line);
    assert_eq!(viewport.end_line, 20);
}
