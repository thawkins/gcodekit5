//! Tests for the unsaved-changes prompt's button mapping: Cancel, Discard and
//! Save buttons, with a dismissed dialog treated as Cancel.

use gcodekit5_ui::ui::gtk::common::dialog::UnsavedChangesChoice;

#[test]
fn test_unsaved_changes_buttons_map_to_choices() {
    assert_eq!(
        UnsavedChangesChoice::from_button_index(Some(0)),
        UnsavedChangesChoice::Cancel
    );
    assert_eq!(
        UnsavedChangesChoice::from_button_index(Some(1)),
        UnsavedChangesChoice::Discard
    );
    assert_eq!(
        UnsavedChangesChoice::from_button_index(Some(2)),
        UnsavedChangesChoice::Save
    );
}

#[test]
fn test_unsaved_changes_dismissed_or_unknown_is_cancel() {
    // Escape / closing the dialog reports no index; nothing may be lost.
    for index in [None, Some(-1), Some(3), Some(i32::MAX)] {
        assert_eq!(
            UnsavedChangesChoice::from_button_index(index),
            UnsavedChangesChoice::Cancel,
            "{index:?}"
        );
    }
}

#[test]
fn test_unsaved_changes_button_order_has_cancel_first_save_last() {
    // `show_unsaved_changes` makes button 0 the cancel button and the last
    // button the default, so this order must hold.
    assert_eq!(
        UnsavedChangesChoice::BUTTONS,
        [
            UnsavedChangesChoice::Cancel,
            UnsavedChangesChoice::Discard,
            UnsavedChangesChoice::Save
        ]
    );
}
