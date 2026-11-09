//! # GLib Log Filter
//!
//! Installs a GLib structured-log writer that drops known-harmless GTK warnings
//! and passes every other message to GLib's default writer unchanged.
//!
//! Currently dropped: GDK's Vulkan `VK_SUBOPTIMAL_KHR` swapchain warning, which
//! GTK prints each time the window surface changes (resize, maximise, monitor or
//! scale change) even though presenting still succeeds.

use gtk4::glib;

/// Returns `true` for log messages that should be suppressed.
///
/// # Arguments
/// * `domain` - The GLib log domain (`GLIB_DOMAIN` field), e.g. `"Gdk"`.
/// * `message` - The log message text (`MESSAGE` field).
pub fn is_suppressed(domain: Option<&str>, message: Option<&str>) -> bool {
    domain == Some("Gdk") && message.is_some_and(|m| m.contains("VK_SUBOPTIMAL_KHR"))
}

/// Installs the filtering log writer.
///
/// Must be called once, before GTK is initialised; GLib allows the writer to be
/// set only once per process.
pub fn install() {
    glib::log_set_writer_func(|level, fields| {
        let field = |key: &str| {
            fields
                .iter()
                .find(|f| f.key() == key)
                .and_then(|f| f.value_str())
        };
        if is_suppressed(field("GLIB_DOMAIN"), field("MESSAGE")) {
            return glib::LogWriterOutput::Handled;
        }
        glib::log_writer_default(level, fields)
    });
}
