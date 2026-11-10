//! Tests for Preferences that change what is sent to the machine
//! (owner decision A-02, issue #76): comments, line length and jog direction.

use gcodekit5_ui::device_status;
use gcodekit5_ui::ui::gtk::machine_control::grbl_stream::prepare_program;

#[test]
fn test_comments_are_removed_unless_preserved() {
    let program = "G0 X1 (rapid)\n; header\nG1 X2 ; cut\n(only a comment)\n";
    assert_eq!(
        prepare_program(program, false, 256).expect("ok"),
        "G0 X1\nG1 X2\n"
    );
    assert_eq!(
        prepare_program(program, true, 256).expect("ok"),
        "G0 X1 (rapid)\n; header\nG1 X2 ; cut\n(only a comment)\n"
    );
}

#[test]
fn test_line_longer_than_maximum_is_refused() {
    let long = format!("G1 X1 ({})", "a".repeat(100));
    let err = prepare_program(&format!("G0 X0\n{long}\n"), true, 80).expect_err("too long");
    assert!(err.contains("line 2"), "{err}");
    // Comments are removed before the length is checked.
    assert!(prepare_program(&long, false, 80).is_ok());
    // 0 disables the check.
    assert!(prepare_program(&long, true, 0).is_ok());
}

#[test]
fn test_inverted_axis_jogs_the_other_way() {
    let _guard = crate::global_env_lock();
    device_status::set_jog_inverted('Y', true);
    assert_eq!(device_status::jog_direction('Y'), -1.0);
    assert_eq!(device_status::jog_direction('X'), 1.0);
    device_status::set_jog_inverted('Y', false);
    assert_eq!(device_status::jog_direction('Y'), 1.0);
}
