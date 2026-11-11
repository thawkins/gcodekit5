//! Single integration-test binary for `gcodekit5-core`.
//!
//! Each test file is included as a child module, so the crate links one test
//! binary instead of one per file. Run one file with
//! `cargo nextest run -p gcodekit5-core -E 'test({module}::)'`.

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

#[path = "audit_regression_tests.rs"]
mod audit_regression_tests;
#[path = "six_axis_position_tests.rs"]
mod six_axis_position_tests;
#[path = "test_type_aliases.rs"]
mod test_type_aliases;
#[path = "test_units.rs"]
mod test_units;
