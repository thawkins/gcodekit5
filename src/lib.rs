//! # GCodeKit5
//!
//! The application binary's library: logging set-up. The application itself
//! is in the workspace crates (`gcodekit5-ui` starts the GTK application).
#![deny(unsafe_code)]

/// Initialize logging with the default configuration
///
/// Sets up structured logging with:
/// - Console output with pretty formatting
/// - RUST_LOG environment variable support
/// - UTF timestamps
#[cfg_attr(test, mutants::skip)]
pub fn init_logging() -> anyhow::Result<()> {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    // RUST_LOG wins when set; otherwise log at INFO.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // In Windows GUI mode (release builds), log to a file instead of stdout
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use std::fs::OpenOptions;

        let log_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        let log_file = log_dir.join("gcodekit5.log");

        // Try to open log file, but if it fails, just disable logging rather than crash
        match OpenOptions::new().create(true).append(true).open(&log_file) {
            Ok(file) => {
                let fmt_layer = fmt::layer()
                    .with_writer(file)
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_line_number(true)
                    .pretty();

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            }
            Err(_) => {
                // If file creation fails, just use a no-op subscriber
                tracing_subscriber::registry().with(env_filter).init();
            }
        }
    }

    // In debug mode or non-Windows, log to stdout as before
    #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
    {
        let fmt_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_target(true)
            .with_level(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_line_number(true)
            .pretty();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}
