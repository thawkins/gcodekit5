//! Single integration-test binary for `gcodekit5-visualizer`.
//!
//! Each test file is included as a child module, so the crate links one test
//! binary instead of one per file. Run one file with
//! `cargo nextest run -p gcodekit5-visualizer -E 'test({module}::)'`.

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

#[path = "audit_followup_regressions.rs"]
mod audit_followup_regressions;
#[path = "gcode_parsing_regressions.rs"]
mod gcode_parsing_regressions;
#[path = "test_stock_removal_heightmap.rs"]
mod test_stock_removal_heightmap;
