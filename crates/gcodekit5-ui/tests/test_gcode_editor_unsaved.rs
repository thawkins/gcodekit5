//! Tests for the G-code editor's unsaved-changes tracking: only hand edits
//! count; opening, generating (`set_text`) and clearing do not.
//!
//! These need a display. Without one (`gtk4::init()` fails) the test returns early.
//! GTK must be driven from one thread, so every case runs inside a single test.

use std::cell::Cell;
use std::rc::Rc;

use gcodekit5_ui::ui::gtk::editor::GcodeEditor;
use gtk4::prelude::*;

#[test]
fn test_gcode_editor_unsaved_tracks_hand_edits_only() {
    if gtk4::init().is_err() {
        eprintln!("skipping test_gcode_editor_unsaved_tracks_hand_edits_only: no display");
        return;
    }
    let editor = GcodeEditor::new(None, None);
    assert!(!editor.has_unsaved_changes(), "a new editor is clean");

    // Generated G-code can be regenerated, so it is not unsaved work.
    editor.set_text("G0 X0 Y0\nG1 X10 F100\n");
    assert!(!editor.has_unsaved_changes(), "generated text is clean");

    // A hand edit is.
    let mut end = editor.buffer.end_iter();
    editor.buffer.insert(&mut end, "G1 Y10\n");
    assert!(editor.has_unsaved_changes(), "a hand edit is unsaved");

    // Nothing unsaved: the pending action runs at once, without a prompt.
    editor.set_text("");
    assert!(!editor.has_unsaved_changes(), "clearing resets the flag");
    let ran = Rc::new(Cell::new(false));
    let seen = ran.clone();
    editor.confirm_unsaved_changes(move || seen.set(true));
    assert!(ran.get(), "a clean editor proceeds immediately");

    // With an unsaved edit the action waits for the user's answer.
    let mut end = editor.buffer.end_iter();
    editor.buffer.insert(&mut end, "M5\n");
    let ran = Rc::new(Cell::new(false));
    let seen = ran.clone();
    editor.confirm_unsaved_changes(move || seen.set(true));
    assert!(!ran.get(), "an unsaved edit waits for the prompt");
}
