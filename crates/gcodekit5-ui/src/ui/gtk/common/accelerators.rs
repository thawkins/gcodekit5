//! # Keyboard Accelerator/Shortcut Management
//!
//! Centralized keyboard shortcut constants and helper functions.
//!
//! This module provides standardized accelerator definitions that can be
//! used with GTK4's Action system and ShortcutController.
//!
//! ## Usage
//!
//! ```rust
//! use crate::ui::gtk::common::accelerators;
//!
//! // Reference standard accelerators:
//! let open_accel = accelerators::StandardShortcuts::FILE_OPEN;
//! ```

use gtk4::gdk::ModifierType;

/// Standard modifier combinations for cross-platform compatibility
pub struct Modifiers {
    /// Primary modifier (Ctrl on Linux/Windows, Cmd on macOS)
    pub primary: ModifierType,
    /// Shift modifier
    pub shift: ModifierType,
    /// Alt modifier
    pub alt: ModifierType,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            primary: ModifierType::CONTROL_MASK,
            shift: ModifierType::SHIFT_MASK,
            alt: ModifierType::ALT_MASK,
        }
    }
}

/// Common accelerator keys used throughout the application
pub mod keys {
    /// File operations
    pub const OPEN: &str = "<Ctrl>O";
    pub const SAVE: &str = "<Ctrl>S";
    pub const QUIT: &str = "<Ctrl>Q";

    /// Machine control
    pub const HOME: &str = "<Ctrl>Home";
    pub const ESTOP: &str = "Escape";

    /// Real-time commands
    pub const FEED_HOLD: &str = "!";
    pub const CYCLE_START: &str = "~";
}

/// Standard application shortcuts registry
///
/// This struct contains accelerator strings for all standard application
/// shortcuts. These can be used with gtk4::ShortcutTrigger::parse_string()
/// or when setting up GtkApplication actions.
pub struct StandardShortcuts;

impl StandardShortcuts {
    /// File menu shortcuts
    pub const FILE_NEW: &str = "<Ctrl>n";
    pub const FILE_OPEN: &str = "<Ctrl>o";
    pub const FILE_SAVE: &str = "<Ctrl>s";
    pub const FILE_SAVE_AS: &str = "<Ctrl><Shift>s";
    pub const FILE_QUIT: &str = "<Ctrl>q";

    /// Machine control shortcuts
    pub const MACHINE_HOME: &str = "<Ctrl>Home";
    pub const MACHINE_RESET: &str = "<Ctrl>x";
    pub const MACHINE_UNLOCK: &str = "<Ctrl>U";

    /// Jogging shortcuts (when in jog mode)
    pub const JOG_X_POS: &str = "Right";
    pub const JOG_X_NEG: &str = "Left";
    pub const JOG_Y_POS: &str = "Up";
    pub const JOG_Y_NEG: &str = "Down";
    pub const JOG_Z_POS: &str = "Page_Up";
    pub const JOG_Z_NEG: &str = "Page_Down";

    /// Real-time commands
    pub const FEED_HOLD: &str = "!";
    pub const CYCLE_START: &str = "~";
    pub const ESTOP: &str = "Escape";

    /// Edit menu shortcuts
    pub const EDIT_UNDO: &str = "<Ctrl>z";
    pub const EDIT_REDO: &str = "<Ctrl>y";
    pub const EDIT_CUT: &str = "<Ctrl>x";
    pub const EDIT_COPY: &str = "<Ctrl>c";
    pub const EDIT_PASTE: &str = "<Ctrl>v";

    /// View shortcuts
    pub const FIT_VIEW: &str = "f";
    pub const ZOOM_IN: &str = "plus";
    pub const ZOOM_OUT: &str = "minus";
    pub const RESET_VIEW: &str = "<Ctrl>0";

    /// Help shortcuts
    pub const HELP_DOCS: &str = "F1";

    /// Run shortcuts
    pub const FILE_RUN: &str = "<Ctrl>r";
}

/// Convert a key event to a normalized accelerator string
pub fn key_event_to_accel(key: &gtk4::gdk::Key, modifiers: ModifierType) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(ModifierType::CONTROL_MASK) {
        parts.push("<Ctrl>".to_string());
    }
    if modifiers.contains(ModifierType::SHIFT_MASK) {
        parts.push("<Shift>".to_string());
    }
    if modifiers.contains(ModifierType::ALT_MASK) {
        parts.push("<Alt>".to_string());
    }
    if modifiers.contains(ModifierType::SUPER_MASK) {
        parts.push("<Super>".to_string());
    }

    parts.push(format!("{}", key.name().unwrap_or_default()));
    parts.concat()
}

/// Platform-aware modifier detection
#[cfg(target_os = "macos")]
pub fn is_primary_modifier(modifiers: ModifierType) -> bool {
    modifiers.contains(ModifierType::META_MASK)
}

#[cfg(not(target_os = "macos"))]
pub fn is_primary_modifier(modifiers: ModifierType) -> bool {
    modifiers.contains(ModifierType::CONTROL_MASK)
}

/// Get the display string for the primary modifier on this platform
pub fn primary_modifier_label() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "⌘"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl"
    }
}
