//! Single integration-test binary for `gcodekit5-ui`.
//!
//! Each `test_*.rs` (or legacy) file is included as a child module, so the crate
//! links one test binary instead of one per file. Run one file with
//! `cargo nextest run -p gcodekit5-ui -E 'test(<module>::)'`.

/// Binary-wide lock for tests that touch process-global state.
pub fn global_env_mutex() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

/// Takes [`global_env_mutex`], recovering it if a previous holder panicked.
pub fn global_env_lock() -> std::sync::MutexGuard<'static, ()> {
    global_env_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[path = "machine_control_streaming.rs"]
mod machine_control_streaming;
#[path = "test_array_dialog.rs"]
mod test_array_dialog;
#[path = "test_command_history.rs"]
mod test_command_history;
#[path = "test_device_console_manager.rs"]
mod test_device_console_manager;
#[path = "test_dialog_window.rs"]
mod test_dialog_window;
#[path = "test_glib_log_filter.rs"]
mod test_glib_log_filter;
#[path = "test_job_preferences.rs"]
mod test_job_preferences;
#[path = "test_machine_busy_guard.rs"]
mod test_machine_busy_guard;
#[path = "test_materials_manager_backend.rs"]
mod test_materials_manager_backend;
#[path = "test_report_units.rs"]
mod test_report_units;
#[path = "test_tools_manager_backend.rs"]
mod test_tools_manager_backend;
#[path = "test_ui_leaks.rs"]
mod test_ui_leaks;
