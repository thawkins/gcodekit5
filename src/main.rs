// On Windows, hide the console window for GUI applications
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use gcodekit5::init_logging;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging()?;

    // Launch GTK Application
    gcodekit5_ui::gtk_app::main();

    Ok(())
}
