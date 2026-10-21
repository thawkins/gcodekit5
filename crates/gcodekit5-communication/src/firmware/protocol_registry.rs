//! gcodekit5-communication/src/firmware/protocol_registry.rs
//!
//! # Protocol Registry
//!
//! Single source of truth for per-protocol behaviour.
//!
//! The registry maps a canonical `controller_type` string (matching
//! `ControllerType::as_str()` in `gcodekit5-devicedb`) to a `ProtocolProfile`
//! describing how the app should behave for that protocol:
//!
//! - Which line processor (if any) the `SerialCommunicator` should install.
//! - Which sender (Direct vs FlowControl) to use for streaming.
//! - Whether the firmware announces its identity (for autodetection).
//! - Whether it is currently wired up and supported.
//!
//! ## Design notes
//!
//! - All data is `&'static` → zero runtime allocation, zero lookup cost.
//! - Unknown protocols fall back to a safe GRBL-like default.
//! - Adding a new protocol is a one-entry change plus a sender/processor
//!   implementation (if needed).

/// How the streaming sender should behave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenderKind {
    /// Direct command emission. No flow window, no per-command ack.
    /// Used by GRBL and GRBL-compatible firmwares.
    Direct,
    /// Flow-control window. Waits for `ok` per command before sending
    /// the next. Used by Repetier.
    FlowControl,
}

/// Per-protocol behaviour profile.
///
/// Looked up via [`protocol_profile`] using the canonical string
/// returned by `ControllerType::as_str()`.
#[derive(Debug, Clone, Copy)]
pub struct ProtocolProfile {
    /// Canonical string. Matches `ControllerType::as_str()`.
    /// Example: `"GRBL"`, `"grblHAL"`, `"g2core"`, `"Repetier"`.
    pub id: &'static str,

    /// Human-readable label. Matches `ControllerType` `Display`.
    /// Example: `"GRBL"`, `"grblHAL"`, `"g2core"`, `"Repetier"`.
    pub display_name: &'static str,

    /// Whether the protocol is fully wired up in the app.
    ///
    /// `false` here does NOT mean the device cannot be used; it means
    /// the app has no protocol-specific behaviour for it. In practice,
    /// GRBL-compatible firmwares still work via `SenderKind::Direct`.
    ///
    /// Used by the UI (fase 6) to mark unsupported entries in the
    /// controller-type combo.
    pub is_supported: bool,

    /// Which sender to use for streaming.
    pub sender: SenderKind,

    /// Whether the firmware emits an identity banner the app can detect.
    ///
    /// Used by the divergence check to decide whether "no detection"
    /// is expected or suspicious. If `false`, the divergence warning is
    /// skipped (fase 5).
    pub announces_identity: bool,

    /// Commands sent right after connecting, in order, spaced by a small
    /// delay (currently 300 ms) between them.
    ///
    /// These are queries and safe setup commands only. They must not
    /// change the machine's state in a way the user did not ask for
    /// (e.g. do not touch `$10`, `$32` or any setting that some firmware
    /// variants may reject).
    ///
    /// - GRBL/grblHAL/FluidNC: `&["$I", "$$"]`
    /// - Repetier: `&["M110", "M115"]`
    /// - TinyG/g2core: `&["{\"fv\":null}"]`
    /// - Smoothieware: `&["version"]`
    /// - Marlin: `&["M115"]`
    /// - Unknown/unsupported: `&[]`
    pub init_commands: &'static [&'static str],

    pub detection_signatures: &'static [&'static str],
}

/// The full registry. Add new protocols by inserting an entry here.
///
pub const PROTOCOLS: &[ProtocolProfile] = &[
    // ─── Repetier ───
    ProtocolProfile {
        id: "Repetier",
        display_name: "Repetier",
        is_supported: true,
        sender: SenderKind::FlowControl,
        announces_identity: true,
        init_commands: &["M110", "M115"],
        detection_signatures: &[
            "Free RAM",
            "FIRMWARE_NAME:Repetier",
            "Repetier",
            "Resend:",
        ],
    },

    // ─── grblHAL ───
    ProtocolProfile {
        id: "grblHAL",
        display_name: "grblHAL",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["$I", "$$"],
        detection_signatures: &["grblHAL", "[FIRMWARE:grblHAL]"],
    },

    // ─── FluidNC ───
    ProtocolProfile {
        id: "FluidNC",
        display_name: "FluidNC",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["$I", "$$"],
        detection_signatures: &["FluidNC"],
    },

    // ─── GRBL ───
    ProtocolProfile {
        id: "GRBL",
        display_name: "GRBL",
        is_supported: true,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["$I", "$$"],
        detection_signatures: &["Grbl"],
    },

    // ──────────────────────────
    // Protocols not implemented
    // ──────────────────────────
    ProtocolProfile {
        id: "TinyG",
        display_name: "TinyG",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["{\"fv\":null}"],
        detection_signatures: &["TinyG", "tinyg"],
    },
    ProtocolProfile {
        id: "g2core",
        display_name: "g2core",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["{\"fv\":null}"],
        detection_signatures: &["g2core", "G2Core"],
    },
    ProtocolProfile {
        id: "Smoothieware",
        display_name: "Smoothieware",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["version"],
        detection_signatures: &["Smoothieware", "Build version:"],
    },
    ProtocolProfile {
        id: "Marlin",
        display_name: "Marlin",
        is_supported: false,
        sender: SenderKind::Direct,
        announces_identity: true,
        init_commands: &["M115"],
        detection_signatures: &["FIRMWARE_NAME:Marlin", "Marlin"],
    },
];

/// Safe fallback for unknown or empty protocol strings.
pub const FALLBACK_PROFILE: ProtocolProfile = ProtocolProfile {
    id: "Unknown",
    display_name: "Unknown",
    is_supported: false,
    sender: SenderKind::Direct,
    announces_identity: false,
    init_commands: &[],
    detection_signatures: &[],
};

/// Look up the profile for a canonical `controller_type` string.
///
/// Returns [`FALLBACK_PROFILE`] if the string is not in the registry.
/// This is safe: an unknown protocol behaves like GRBL-compatible
/// (Direct sender, no processor).
/// Look up the profile for a canonical `controller_type` string.
///
/// Returns [`FALLBACK_PROFILE`] if the string is not in the registry.
/// This is safe: an unknown protocol behaves like GRBL-compatible
/// (Direct sender, no processor).
pub fn protocol_profile(id: &str) -> ProtocolProfile {
    PROTOCOLS
        .iter()
        .find(|p| p.id == id)
        .copied()
        .unwrap_or(FALLBACK_PROFILE)
}

/// Detects the protocol based on incoming bytes.
pub fn detect_protocol(text: &str) -> Option<ControllerProtocol> {
    let text_lower = text.to_lowercase();
    PROTOCOLS
        .iter()
        .find(|p| {
            p.detection_signatures
                .iter()
                .any(|sig| text_lower.contains(&sig.to_lowercase()))
        })
        .map(|p| ControllerProtocol::from_str(p.id))
}

pub use super::ControllerProtocol;

// ────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
