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
// Win32 FFI module: every unsafe block below carries its own SAFETY comment.
#![allow(unsafe_code)]

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    Win32WindowHandle, WindowHandle,
};

/// Wrapper for a valid Win32 window handle (HWND).
///
/// This struct ensures the HWND is non-zero and provides the necessary
/// traits for rfd to use it as a parent window for file dialogs.
pub struct Win32ParentHandle(pub std::num::NonZeroIsize);

impl HasWindowHandle for Win32ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        let mut handle = Win32WindowHandle::new(self.0);
        // SAFETY: GetModuleHandleW(null) returns the module handle of the
        // current process, which is always valid.
        let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) } as isize;
        handle.hinstance = std::num::NonZeroIsize::new(hinstance);
        // SAFETY: the HWND is a non-zero handle from GetForegroundWindow, and
        // the dialog that borrows it is modal, so it is shown and closed while
        // that window still exists.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

impl HasDisplayHandle for Win32ParentHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
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
