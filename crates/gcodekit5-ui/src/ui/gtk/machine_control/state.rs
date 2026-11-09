//! # Machine Control State Management
//!
//! Internal state management and helper functions for the Machine Control panel.

use gtk4::prelude::*;
use gtk4::{Button, ComboBoxText, ToggleButton};

/// Disable/enable connection-dependent controls
/// Includes support for 6-axis machines (X, Y, Z, A, B, C)
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
    // Rotary axis controls (optional - pass None if not available)
    a_zero_btn: Option<&Button>,
    b_zero_btn: Option<&Button>,
    c_zero_btn: Option<&Button>,
    jog_a_pos: Option<&Button>,
    jog_a_neg: Option<&Button>,
    jog_b_pos: Option<&Button>,
    jog_b_neg: Option<&Button>,
    jog_c_pos: Option<&Button>,
    jog_c_neg: Option<&Button>,
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

    // Rotary axis controls (only if they exist)
    if let Some(btn) = a_zero_btn {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = b_zero_btn {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = c_zero_btn {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_a_pos {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_a_neg {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_b_pos {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_b_neg {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_c_pos {
        btn.set_sensitive(enabled);
    }
    if let Some(btn) = jog_c_neg {
        btn.set_sensitive(enabled);
    }
}

/// Simpler version for non-6-axis setups (backward compatibility)
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn set_controls_enabled_basic(
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
    set_controls_enabled(
        send_btn,
        stop_btn,
        pause_btn,
        resume_btn,
        home_btn,
        unlock_btn,
        wcs_btns,
        x_zero_btn,
        y_zero_btn,
        z_zero_btn,
        zero_all_btn,
        goto_zero_btn,
        step_combo,
        jog_feed_entry,
        jog_x_pos,
        jog_x_neg,
        jog_y_pos,
        jog_y_neg,
        jog_z_pos,
        jog_z_neg,
        estop_btn,
        None,
        None,
        None, // a_zero, b_zero, c_zero
        None,
        None,
        None,
        None,
        None,
        None, // jog_a_pos, jog_a_neg, jog_b_pos, jog_b_neg, jog_c_pos, jog_c_neg
        enabled,
    );
}
