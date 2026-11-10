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
#[path = "test_dialog_window.rs"]
mod test_dialog_window;
#[path = "test_glib_log_filter.rs"]
mod test_glib_log_filter;
#[path = "test_job_preferences.rs"]
mod test_job_preferences;
