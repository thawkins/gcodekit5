//! Regression tests for the communication audit follow-ups: serial port
//! filtering and GRBL probe report parsing.

use gcodekit5_communication::communication::serial::is_valid_cnc_port_name;
use gcodekit5_communication::firmware::grbl::status_parser::parse_probe_report;

// ---------- Item 5: serial port filtering ----------

#[test]
fn test_serial_port_filter_includes_uarts_and_bluetooth() {
    for name in [
        "/dev/ttyUSB0",
        "/dev/ttyACM1",
        "/dev/ttyS0",
        "/dev/ttyAMA0",
        "/dev/rfcomm0",
        "/dev/ttyGRBL",
        "COM3",
        "/dev/cu.usbmodem1421",
        "/dev/cu.HC-05",
    ] {
        assert!(is_valid_cnc_port_name(name), "{name} should be listed");
    }
}

#[test]
fn test_serial_port_filter_excludes_non_devices() {
    for name in [
        "/dev/tty1",
        "/dev/tty",
        "/dev/console",
        "/dev/ptmx",
        "/dev/pts/0",
        "/dev/ttyS",
        "/dev/tty.usbmodem1421",
        "/dev/cu.Bluetooth-Incoming-Port",
        "/dev/cu.debug-console",
        "COM",
        "COMX",
    ] {
        assert!(!is_valid_cnc_port_name(name), "{name} should be hidden");
    }
}

// ---------- Item 10: GRBL probing and work offsets ----------

#[test]
fn test_grbl_probe_report_parsing() {
    let ok = parse_probe_report("[PRB:1.500,-2.250,-10.125:1]").expect("PRB");
    assert!(ok.success);
    assert_eq!(ok.position.x, 1.5);
    assert_eq!(ok.position.y, -2.25);
    assert_eq!(ok.position.z, -10.125);

    let miss = parse_probe_report("[PRB:0.000,0.000,-5.000:0]").expect("PRB");
    assert!(!miss.success);

    let four_axis = parse_probe_report("[PRB:1.0,2.0,3.0,45.0:1]").expect("PRB 4-axis");
    assert_eq!(four_axis.position.a, Some(45.0));

    assert!(parse_probe_report("[PRB:1.0,2.0]").is_none());
    assert!(parse_probe_report("[GC:G0 G54]").is_none());
    assert!(parse_probe_report("ok").is_none());
}
