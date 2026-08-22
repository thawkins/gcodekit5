//! # Windows Platform Support
//!
//! Windows-specific UI utilities for window handle management and native dialogs.
//!
//! ## Win32 Parent Handle
//!
//! This module provides `Win32ParentHandle` which wraps a valid non-zero HWND
//! obtained from `GetForegroundWindow()`. This allows rfd dialogs to be properly
//! parented to the application window on Windows.

#![cfg(target_os = "windows")]

use raw_window_handle::{
    HasDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle, Win32DisplayHandle,
    Win32WindowHandle,
};

/// Wrapper for a valid Win32 window handle (HWND).
///
/// This struct ensures the HWND is non-zero and provides the necessary
/// traits for rfd to use it as a parent window for file dialogs.
pub struct Win32ParentHandle(pub std::num::NonZeroIsize);

// SAFETY: Win32ParentHandle wraps a valid non-zero HWND obtained from
// GetForegroundWindow. The handle is valid for the lifetime of the window.
unsafe impl HasRawWindowHandle for Win32ParentHandle {
    fn raw_window_handle(&self) -> Result<RawWindowHandle, raw_window_handle::HandleError> {
        let handle = Win32WindowHandle::new(self.0);
        Ok(RawWindowHandle::Win32(handle))
    }
}

// SAFETY: Win32ParentHandle provides a valid display handle via GetModuleHandleW.
// The module handle is valid for the lifetime of the process.
unsafe impl HasDisplayHandle for Win32ParentHandle {
    fn raw_display_handle(&self) -> Result<RawDisplayHandle, raw_window_handle::HandleError> {
        // Get module handle for the current process as hinstance
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        // SAFETY: GetModuleHandleW(null) returns the module handle of the
        // current process, which is always valid.
        let hinst = unsafe { GetModuleHandleW(std::ptr::null()) } as isize;
        if let Some(nz) = std::num::NonZeroIsize::new(hinst) {
            let dh = Win32DisplayHandle::new(nz);
            Ok(RawDisplayHandle::Win32(dh))
        } else {
            Err(raw_window_handle::HandleError::UnsupportedPlatform)
        }
    }
}

/// Get the HWND of the current foreground window.
///
/// Returns `None` if no window is currently in the foreground.
/// Uses `GetForegroundWindow()` from the Win32 API.
pub fn get_foreground_hwnd() -> Option<Win32ParentHandle> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: GetForegroundWindow returns the HWND of the current foreground
    // window, or null if none. We check for null via NonZeroIsize.
    let hwnd_val = unsafe { GetForegroundWindow() } as isize;
    std::num::NonZeroIsize::new(hwnd_val).map(Win32ParentHandle)
}
