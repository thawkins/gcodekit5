//! Tests for `gcodekit5_communication::firmware::protocol_registry` (moved from the inline test module).

use gcodekit5_communication::firmware::protocol_registry::*;

#[test]
fn known_protocols_are_found() {
    assert_eq!(protocol_profile("GRBL").id, "GRBL");
    assert_eq!(protocol_profile("grblHAL").id, "grblHAL");
    assert_eq!(protocol_profile("FluidNC").id, "FluidNC");
    assert_eq!(protocol_profile("Repetier").id, "Repetier");
    assert_eq!(protocol_profile("TinyG").id, "TinyG");
    assert_eq!(protocol_profile("g2core").id, "g2core");
    assert_eq!(protocol_profile("Smoothieware").id, "Smoothieware");
    assert_eq!(protocol_profile("Marlin").id, "Marlin");
}

#[test]
fn unknown_falls_back_to_safe_default() {
    let p = protocol_profile("Siemens");
    assert_eq!(p.id, "Unknown");
    assert_eq!(p.sender, SenderKind::Direct);
    assert!(!p.is_supported);
}

#[test]
fn empty_falls_back_to_safe_default() {
    let p = protocol_profile("");
    assert_eq!(p.id, "Unknown");
    assert_eq!(p.sender, SenderKind::Direct);
}

#[test]
fn repetier_uses_flow_control_and_processor() {
    let p = protocol_profile("Repetier");
    assert_eq!(p.sender, SenderKind::FlowControl);
    assert!(p.is_supported);
}

#[test]
fn grbl_uses_direct_and_no_processor() {
    let p = protocol_profile("GRBL");
    assert_eq!(p.sender, SenderKind::Direct);
    assert!(p.is_supported);
}

#[test]
fn unsupported_protocols_fall_back_to_direct() {
    for id in ["TinyG", "g2core", "Smoothieware", "Marlin"] {
        let p = protocol_profile(id);
        assert_eq!(p.sender, SenderKind::Direct, "{} should use Direct", id);
        assert!(!p.is_supported, "{} should be unsupported", id);
    }
}

#[test]
fn ids_match_display_names_for_simple_cases() {
    // For most protocols id and display_name are the same string.
    // They differ only when we later want to shorten the id or
    // change the label without touching comparisons.
    assert_eq!(protocol_profile("GRBL").display_name, "GRBL");
    assert_eq!(protocol_profile("Repetier").display_name, "Repetier");
}
