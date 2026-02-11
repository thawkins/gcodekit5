//! Test virtual port detection (e.g., /dev/ttyGRBL for grblHAL simulator)

use gcodekit5_communication::communication::serial::list_ports;

#[test]
fn test_virtual_port_detection() {
    // This test verifies that virtual ports like /dev/ttyGRBL are detected
    // when they exist on the system

    match list_ports() {
        Ok(ports) => {
            // Check if /dev/ttyGRBL exists on the system
            #[cfg(target_os = "linux")]
            {
                let tty_grbl_path = std::path::Path::new("/dev/ttyGRBL");
                if tty_grbl_path.exists() {
                    // If the device exists, it should be in the list
                    let found = ports.iter().any(|p| p.port_name == "/dev/ttyGRBL");
                    assert!(
                        found,
                        "/dev/ttyGRBL exists but was not detected in port listing"
                    );
                }
            }
        }
        Err(e) => {
            panic!("Failed to list ports: {}", e);
        }
    }
}
