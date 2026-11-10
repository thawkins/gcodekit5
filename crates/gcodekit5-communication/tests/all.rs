//! Single integration-test binary for `gcodekit5-communication`.
//!
//! Each test file is included as a child module, so the crate links one test
//! binary instead of one per file. Run one file with
//! `cargo nextest run -p gcodekit5-communication -E 'test(<module>::)'`.

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

#[path = "audit_followup_tests.rs"]
mod audit_followup_tests;
#[path = "grbl_protocol_tests.rs"]
mod grbl_protocol_tests;
#[path = "override_sequencer_tests.rs"]
mod override_sequencer_tests;
#[path = "tcp_transport_tests.rs"]
mod tcp_transport_tests;
#[path = "test_error_decoder.rs"]
mod test_error_decoder;
#[path = "test_protocol_registry.rs"]
mod test_protocol_registry;
#[path = "test_repetier.rs"]
mod test_repetier;
