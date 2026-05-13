//! # Machine Control State Management
//!
//! Internal state management and helper functions for the Machine Control panel.

use gtk4::prelude::*;
use gtk4::{Button, ComboBoxText, ToggleButton};

/// Disable/enable connection-dependent controls
#[allow(clippy::too_many_arguments)]
pub fn set_controls_enabled(
    send_btn: &Button,
    stop_btn: &Button,
    pause_btn: &Button,
    resume_btn: &Button,
    home_btn: &Button,
    unlock_btn: &Button,
    wcs_btns: &[ToggleButton],
    x_zero_btn: &Button,
    y_zero_btn: &Button,
    z_zero_btn: &Button,
    zero_all_btn: &Button,
    goto_zero_btn: &Button,
    step_combo: &ComboBoxText,
    jog_feed_entry: &gtk4::Entry,
    jog_x_pos: &Button,
    jog_x_neg: &Button,
    jog_y_pos: &Button,
    jog_y_neg: &Button,
    jog_z_pos: &Button,
    jog_z_neg: &Button,
    estop_btn: &Button,
    enabled: bool,
) {
    send_btn.set_sensitive(enabled);
    stop_btn.set_sensitive(enabled);
    pause_btn.set_sensitive(enabled);
    resume_btn.set_sensitive(enabled);
    home_btn.set_sensitive(enabled);
    unlock_btn.set_sensitive(enabled);
    for btn in wcs_btns {
        btn.set_sensitive(enabled);
    }
    x_zero_btn.set_sensitive(enabled);
    y_zero_btn.set_sensitive(enabled);
    z_zero_btn.set_sensitive(enabled);
    zero_all_btn.set_sensitive(enabled);
    goto_zero_btn.set_sensitive(enabled);
    step_combo.set_sensitive(enabled);
    jog_feed_entry.set_sensitive(enabled);
    jog_x_pos.set_sensitive(enabled);
    jog_x_neg.set_sensitive(enabled);
    jog_y_pos.set_sensitive(enabled);
    jog_y_neg.set_sensitive(enabled);
    jog_z_pos.set_sensitive(enabled);
    jog_z_neg.set_sensitive(enabled);
    estop_btn.set_sensitive(enabled);
}
