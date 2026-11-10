//! Tests for `gcodekit5_ui::ui::gtk::command_history` (moved from the inline test module).

use gcodekit5_ui::ui::gtk::command_history::*;

#[test]
fn test_add_command() {
    let mut history = CommandHistory::new();
    history.add("G0 X10".to_string());
    history.add("G0 Y20".to_string());

    assert_eq!(history.len(), 2);
}

#[test]
fn test_no_duplicate_consecutive() {
    let mut history = CommandHistory::new();
    history.add("G0 X10".to_string());
    history.add("G0 X10".to_string());

    assert_eq!(history.len(), 1);
}

#[test]
fn test_navigate_up() {
    let mut history = CommandHistory::new();
    history.add("cmd1".to_string());
    history.add("cmd2".to_string());
    history.add("cmd3".to_string());

    let prev = history.previous("current");
    assert_eq!(prev, Some("cmd3".to_string()));

    let prev = history.previous("current");
    assert_eq!(prev, Some("cmd2".to_string()));
}

#[test]
fn test_navigate_down() {
    let mut history = CommandHistory::new();
    history.add("cmd1".to_string());
    history.add("cmd2".to_string());

    history.previous("current");
    history.previous("current");

    let next = history.next();
    assert_eq!(next, Some("cmd2".to_string()));

    let next = history.next();
    assert_eq!(next, Some("current".to_string()));
}

#[test]
fn test_max_history() {
    let mut history = CommandHistory::new();

    for i in 0..300 {
        history.add(format!("cmd{}", i));
    }

    assert_eq!(history.len(), MAX_HISTORY);
    assert_eq!(history.get(0), Some("cmd50"));
}
