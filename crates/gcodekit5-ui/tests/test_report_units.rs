//! Tests for keeping the `$13` report-units cache in step with what is sent
//! (review of #86).

use gcodekit5_ui::device_status;

#[test]
fn test_sent_setting_updates_report_units() {
    let _guard = crate::global_env_lock();
    // The cache follows a sent assignment only once the controller accepts it.
    device_status::note_setting_sent("$13=1");
    device_status::note_command_reply(None);
    assert!(device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(), Some(Ok(())));
    device_status::note_setting_sent("$13=0");
    device_status::note_command_reply(None);
    assert!(!device_status::reports_in_inches());
    let _ = device_status::take_setting_reply();
    // Not a setting assignment: ignored.
    device_status::note_setting_sent("G0 X1");
    device_status::note_setting_sent("$J=G91 X1 F100");
    device_status::note_command_reply(None);
    assert!(!device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(), None);
}

#[test]
fn test_rejected_setting_keeps_cached_value() {
    let _guard = crate::global_env_lock();
    device_status::note_setting_sent("$13=1");
    device_status::note_command_reply(None);
    let _ = device_status::take_setting_reply();
    assert!(device_status::reports_in_inches());

    // Sent but not answered yet: the cache keeps the old value.
    device_status::note_setting_sent("$13=0");
    assert!(device_status::reports_in_inches());
    assert_eq!(device_status::take_setting_reply(), None);

    // Rejected: the old value stays and the error is reported once.
    device_status::note_command_reply(Some("3"));
    assert!(device_status::reports_in_inches());
    assert_eq!(
        device_status::take_setting_reply(),
        Some(Err("3".to_string()))
    );
    assert_eq!(device_status::take_setting_reply(), None);

    // A later reply does not belong to the rejected assignment.
    device_status::note_command_reply(None);
    assert!(device_status::reports_in_inches());

    // Leave the cache as other tests expect it.
    device_status::note_setting_sent("$13=0");
    device_status::note_command_reply(None);
    let _ = device_status::take_setting_reply();
}
