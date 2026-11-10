//! Tests for keeping the `$13` report-units cache in step with what is sent
//! (review of #86).

use gcodekit5_ui::device_status;

#[test]
fn test_sent_setting_updates_report_units() {
    let _guard = crate::global_env_lock();
    device_status::note_setting_sent("$13=1");
    assert!(device_status::reports_in_inches());
    device_status::note_setting_sent("$13=0");
    assert!(!device_status::reports_in_inches());
    // Not a setting assignment: ignored.
    device_status::note_setting_sent("G0 X1");
    device_status::note_setting_sent("$J=G91 X1 F100");
    assert!(!device_status::reports_in_inches());
}
