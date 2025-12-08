#![cfg(feature = "slint_legacy_tests")]
use gcodekit5_gcodeeditor::EditorBridge;

#[test]
#[ignore = "Slint legacy: ignored until port to GTK-based EditorBridge or equivalent"]
fn test_bridge_load_text() {
    let bridge = EditorBridge::new(400.0, 20.0);
    bridge.load_text("Hello\nWorld");
    assert_eq!(bridge.get_text(), "Hello\nWorld");
}

#[test]
#[ignore = "Slint legacy: ignored until port to GTK-based EditorBridge or equivalent"]
fn test_bridge_undo_redo() {
    let bridge = EditorBridge::new(400.0, 20.0);
    bridge.load_text("");
    bridge.insert_text("Hello");

    assert!(bridge.can_undo());
    assert!(bridge.undo());
    assert_eq!(bridge.get_text(), "");

    assert!(bridge.can_redo());
    assert!(bridge.redo());
    assert_eq!(bridge.get_text(), "Hello");
}

#[test]
#[ignore = "Slint legacy: ignored until port to GTK-based EditorBridge or equivalent"]
fn test_bridge_line_count() {
    let bridge = EditorBridge::new(400.0, 20.0);
    bridge.load_text("Line 1\nLine 2\nLine 3");
    assert_eq!(bridge.line_count(), 3);
}

#[test]
#[ignore = "Slint legacy: ignored until port to GTK-based EditorBridge or equivalent"]
fn test_bridge_modified_state() {
    let bridge = EditorBridge::new(400.0, 20.0);
    bridge.load_text("Test");
    assert!(!bridge.is_modified());

    bridge.insert_text(" more");
    assert!(bridge.is_modified());

    bridge.mark_unmodified();
    assert!(!bridge.is_modified());
}
