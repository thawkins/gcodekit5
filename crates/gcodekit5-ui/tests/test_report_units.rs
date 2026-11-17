//! Tests for keeping the `$13` report-units cache in step with what is sent
//! (review of #86), and for applying a setting only once the controller
//! accepts it.

use gcodekit5_ui::device_status;

/// Sends `command` and answers it with `ok`.
fn accept(command: &str) {
    let id = device_status::begin_setting_change(command).expect("assignment");
    device_status::note_setting_reply(id, None);
    assert_eq!(device_status::take_setting_reply(id), Some(Ok(())));
}

#[test]
fn test_sent_setting_updates_report_units() {
    let _guard = crate::global_env_lock();
    accept("$13=1");
    assert!(device_status::reports_in_inches());
    accept("$13=0");
    assert!(!device_status::reports_in_inches());
    // Not a setting assignment: not tracked.
    assert_eq!(device_status::begin_setting_change("G0 X1"), None);
    assert_eq!(device_status::begin_setting_change("$J=G91 X1 F100"), None);
}

#[test]
fn test_rejected_setting_keeps_cached_value() {
    let _guard = crate::global_env_lock();
    accept("$13=1");

    // Sent but not answered yet: the cache keeps the old value.
    let id = device_status::begin_setting_change("$13=0").expect("assignment");
    assert!(device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(id), None);

    // Rejected: the old value stays and the error is reported once.
    device_status::note_setting_reply(id, Some("3"));
    assert!(device_status::reports_in_inches());
    assert_eq!(
        device_status::take_setting_reply(id),
        Some(Err("3".to_string()))
    );
    assert_eq!(device_status::take_setting_reply(id), None);

    accept("$13=0");
}

#[test]
fn test_setting_replies_stay_with_their_request() {
    let _guard = crate::global_env_lock();
    accept("$13=0");

    // Two outstanding edits: each reply applies only its own value.
    let first = device_status::begin_setting_change("$13=1").expect("assignment");
    let second = device_status::begin_setting_change("$13=0").expect("assignment");
    device_status::note_setting_reply(first, Some("3"));
    device_status::note_setting_reply(second, None);
    assert!(!device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(second), Some(Ok(())));
    assert_eq!(
        device_status::take_setting_reply(first),
        Some(Err("3".to_string()))
    );
}

#[test]
fn test_disconnect_forgets_pending_settings() {
    let _guard = crate::global_env_lock();
    accept("$13=0");

    let id = device_status::begin_setting_change("$13=1").expect("assignment");
    device_status::update_connection_status(false, None);
    // A reply after the session ended does not apply the old assignment.
    device_status::note_setting_reply(id, None);
    assert!(!device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(id), None);
}

#[test]
fn test_dispatch_reply_routes_only_tagged_replies() {
    let _guard = crate::global_env_lock();
    accept("$13=0");

    // An untagged reply (a job line, `$I`, ...) is not a setting's.
    assert!(!device_status::dispatch_reply(None, None));

    // A tagged reply resolves its own setting, even inside a job stream.
    let id = device_status::begin_setting_change("$13=1").expect("assignment");
    assert!(device_status::dispatch_reply(Some(id), None));
    assert!(device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(id), Some(Ok(())));

    // A rejected setting is the setting's error, not the job's.
    let id = device_status::begin_setting_change("$13=0").expect("assignment");
    assert!(device_status::dispatch_reply(Some(id), Some("3")));
    assert!(device_status::reports_in_inches());
    assert_eq!(
        device_status::take_setting_reply(id),
        Some(Err("3".to_string()))
    );

    accept("$13=0");
}
