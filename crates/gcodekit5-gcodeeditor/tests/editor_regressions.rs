//! Regression tests for editor undo/redo and viewport handling.

use gcodekit5_gcodeeditor::{EditorState, TextBuffer, TextChange, UndoManager, Viewport};

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

#[test]
fn test_undo_batch_is_one_atomic_step() {
    let mut editor = EditorState::new(400.0, 20.0);
    editor.load_text("G1 X1");

    editor.undo_manager_mut().begin_batch();
    editor.set_cursor(0);
    editor.insert_text("; a\n");
    editor.set_cursor(editor.get_text().chars().count());
    editor.insert_text("\nM5");
    editor.undo_manager_mut().end_batch();

    assert_eq!(editor.get_text(), "; a\nG1 X1\nM5");
    assert_eq!(editor.undo_manager().undo_count(), 1);

    assert!(editor.undo());
    assert_eq!(editor.get_text(), "G1 X1");
    assert!(!editor.can_undo());

    assert!(editor.redo());
    assert_eq!(editor.get_text(), "; a\nG1 X1\nM5");
}

#[test]
fn test_undo_manager_nested_batch_stays_single_step() {
    let mut manager = UndoManager::new();
    manager.begin_batch();
    manager.record(TextChange::new(0..0, String::new(), "a".into(), 0, 1));
    manager.begin_batch();
    manager.record(TextChange::new(1..1, String::new(), "b".into(), 1, 2));
    manager.end_batch();
    assert_eq!(manager.undo_count(), 1);
    let inverses = manager.undo().expect("undo");
    // Inverses come last-change-first.
    assert_eq!(inverses.len(), 2);
    assert_eq!(inverses[0].char_range, 1..2);
    assert_eq!(inverses[1].char_range, 0..1);
}

#[test]
fn test_undo_manager_depth_limit_drops_oldest() {
    let mut manager = UndoManager::with_depth(2);
    for i in 0..5 {
        manager.record(TextChange::new(i..i, String::new(), "x".into(), i, i + 1));
    }
    assert_eq!(manager.undo_count(), 2);
    let newest = manager.undo().expect("undo");
    assert_eq!(newest[0].char_range, 4..5);
}

#[test]
fn test_editor_insert_text_at_cursor_records_undo() {
    let mut editor = EditorState::new(400.0, 20.0);
    editor.load_text("G1 X1");
    editor.set_cursor(2);
    editor.insert_text("°");
    assert_eq!(editor.get_text(), "G1° X1");
    assert_eq!(editor.cursor(), 3);
    assert!(editor.is_modified());
    assert!(editor.can_undo());

    assert!(editor.undo());
    assert_eq!(editor.get_text(), "G1 X1");
    assert_eq!(editor.cursor(), 2);
    assert!(editor.redo());
    assert_eq!(editor.get_text(), "G1° X1");
}

#[test]
fn test_editor_set_cursor_clamps_to_length() {
    let mut editor = EditorState::new(400.0, 20.0);
    editor.load_text("abc");
    editor.set_cursor(99);
    assert_eq!(editor.cursor(), 3);
    editor.insert_text("d");
    assert_eq!(editor.get_text(), "abcd");
}

#[test]
fn test_text_buffer_dirty_lines_are_deduplicated_and_sorted() {
    let mut buffer = TextBuffer::from("a\nb\nc\nd\n");
    for _ in 0..1000 {
        let line3 = buffer.line_col_to_char(3, 0);
        buffer.insert(line3, "x");
        buffer.insert(0, "y");
    }
    assert!(buffer.is_line_dirty(0));
    assert!(buffer.is_line_dirty(3));
    assert_eq!(buffer.take_dirty_lines(), vec![0, 3]);
    assert!(buffer.take_dirty_lines().is_empty());
}
