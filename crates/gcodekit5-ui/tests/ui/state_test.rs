#![cfg(feature = "slint_legacy_tests")]
use gcodekit5_ui::ui::state::{UiState, ConnectionState};

#[test]
fn test_ui_state_creation() {
    let state = UiState::new();
    assert_eq!(state.connection_state, ConnectionState::Disconnected);
    assert!(!state.file_state.file_loaded);
}

#[test]
fn test_connection_state() {
    let mut state = UiState::new();
    state.set_connection_state(ConnectionState::Connected);
    assert_eq!(state.connection_state, ConnectionState::Connected);
}

#[test]
fn test_position_update() {
    let mut state = UiState::new();
    state.update_position(10.0, 20.0, 5.0);
    assert_eq!(state.controller_state.position_x, 10.0);
    assert_eq!(state.controller_state.position_y, 20.0);
    assert_eq!(state.controller_state.position_z, 5.0);
}

#[test]
fn test_file_state() {
    let mut state = UiState::new();
    state.load_file("test.gcode".to_string(), 100);
    assert!(state.file_state.file_loaded);
    assert_eq!(state.file_state.total_lines, 100);
}

#[test]
fn test_settings() {
    let mut state = UiState::new();
    state.set_setting("theme", "dark");
    assert_eq!(state.get_setting("theme"), Some("dark"));
}
