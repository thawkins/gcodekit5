//! Tests for `glib_log_filter::is_suppressed`: only GDK's `VK_SUBOPTIMAL_KHR`
//! warning is dropped; everything else reaches the default writer.

use gcodekit5_ui::glib_log_filter::is_suppressed;

const VK_MSG: &str = "vkAcquireNextImageKHR(): A swapchain no longer matches the surface \
     properties exactly, but can still be used to present to the surface successfully. \
     (VK_SUBOPTIMAL_KHR) (1000001003)";

#[test]
fn test_glib_log_filter_drops_gdk_vk_suboptimal() {
    assert!(is_suppressed(Some("Gdk"), Some(VK_MSG)));
}

#[test]
fn test_glib_log_filter_keeps_other_gdk_warnings() {
    assert!(!is_suppressed(
        Some("Gdk"),
        Some("vkQueuePresentKHR(): VK_ERROR_DEVICE_LOST")
    ));
}

#[test]
fn test_glib_log_filter_keeps_other_domains() {
    assert!(!is_suppressed(Some("Gtk"), Some(VK_MSG)));
    assert!(!is_suppressed(None, Some(VK_MSG)));
}

#[test]
fn test_glib_log_filter_keeps_missing_message() {
    assert!(!is_suppressed(Some("Gdk"), None));
}
