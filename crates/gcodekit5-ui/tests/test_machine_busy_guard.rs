//! Tests for the shared busy guard and the Stop sequence state check
//! (audit car 3, issue #74).

use gcodekit5_ui::device_status;

#[test]
fn test_console_and_settings_are_blocked_while_a_job_runs() {
    let _guard = crate::global_env_lock();
    device_status::update_state("Run".to_string());
    device_status::set_job_active(true);
    assert!(device_status::manual_command_blocked().is_some());
    assert!(device_status::settings_command_blocked().is_some());

    device_status::set_job_active(false);
    assert!(device_status::manual_command_blocked().is_none());
    // GRBL rejects `$` commands while it is moving, job or not.
    assert!(device_status::settings_command_blocked().is_some());

    device_status::update_state("Idle".to_string());
    assert!(device_status::settings_command_blocked().is_none());
    device_status::update_state("Alarm:1".to_string());
    assert!(device_status::settings_command_blocked().is_none());
    device_status::update_state(String::new());
}

#[test]
fn test_stop_resets_only_once_the_machine_has_stopped() {
    for moving in ["Run", "Jog", "Hold:1", "Home", "Door:1"] {
        assert!(!device_status::machine_is_stopped(moving), "{moving}");
    }
    for stopped in ["Hold:0", "Hold", "Idle", "Alarm:3", "Check", "Door:0", ""] {
        assert!(device_status::machine_is_stopped(stopped), "{stopped}");
    }
}
