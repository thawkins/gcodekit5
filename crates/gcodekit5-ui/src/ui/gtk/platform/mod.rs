//! # Platform-Specific UI Utilities
//!
//! This module provides platform-specific implementations for UI operations
//! that vary between operating systems (Windows, macOS, Linux).
//!
//! ## Module Structure
//!
//! - `mod.rs` - Common platform interface and re-exports
//! - `windows.rs` - Windows-specific implementations (Win32 handles, file dialogs)
//! - `unix.rs` - Unix/Linux-specific implementations
//! - `macos.rs` - macOS-specific implementations
//!
//! ## Usage
//!
//! Use the functions exported from this module directly - they automatically
//! dispatch to the correct platform implementation based on `#[cfg(target_os)]`.

use std::path::PathBuf;

mod windows;

/// Pick a file using a platform-native file dialog with proper parent window handling.
///
/// On Windows, this ensures the dialog is modal to the current application window.
/// On other platforms, this falls back to the standard dialog.
pub fn pick_file_with_parent(dialog: rfd::FileDialog) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(parent) = windows::get_foreground_hwnd() {
            return dialog.set_parent(&parent).pick_file();
        }
    }
    dialog.pick_file()
}

/// Save a file using a platform-native file dialog with proper parent window handling.
///
/// On Windows, this ensures the dialog is modal to the current application window.
/// On other platforms, this falls back to the standard dialog.
pub fn save_file_with_parent(dialog: rfd::FileDialog) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(parent) = windows::get_foreground_hwnd() {
            return dialog.set_parent(&parent).save_file();
        }
    }
    dialog.save_file()
}

/// Pick a folder using a platform-native file dialog with proper parent window handling.
///
/// On Windows, this ensures the dialog is modal to the current application window.
/// On other platforms, this falls back to the standard dialog.
pub fn pick_folder_with_parent(dialog: rfd::FileDialog) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(parent) = windows::get_foreground_hwnd() {
            return dialog.set_parent(&parent).pick_folder();
        }
    }
    dialog.pick_folder()
}
