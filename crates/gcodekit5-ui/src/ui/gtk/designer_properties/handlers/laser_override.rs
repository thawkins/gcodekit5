//! Handlers for the per-shape laser override (feed rate, power, passes).
//!
//! Changes apply to *all* selected shapes as a single undoable step. The entries
//! apply on commit (Enter / focus-out) via [`super::connect_entry_commit`].

use super::{change_selected_with_undo, connect_entry_commit};
use crate::ui::gtk::designer_properties::PropertiesPanel;
use gcodekit5_designer::model::{LaserParams, Shape};
use gtk4::prelude::*;

/// Writes `params` into the shape-embedded laser parameters of `shape`.
///
/// Raster images have no embedded laser parameters and are left unchanged.
fn set_shape_laser_params(shape: &mut Shape, params: LaserParams) {
    match shape {
        Shape::Rectangle(s) => s.laser_params = params,
        Shape::Circle(s) => s.laser_params = params,
        Shape::Ellipse(s) => s.laser_params = params,
        Shape::Line(s) => s.laser_params = params,
        Shape::Path(s) => s.laser_params = params,
        Shape::Polygon(s) => s.laser_params = params,
        Shape::Triangle(s) => s.laser_params = params,
        Shape::Text(s) => s.laser_params = params,
        Shape::Gear(s) => s.laser_params = params,
        Shape::Sprocket(s) => s.laser_params = params,
        Shape::RasterImage(_) => {}
    }
}

/// Reads the override widgets and applies them to every selected shape (one undo step).
fn apply_laser_override(panel: &PropertiesPanel) {
    let use_global = panel.laser_use_global_check.is_active();

    {
        let mut designer_state = panel.state.borrow_mut();

        let (feed_rate, power_percent, passes) = if use_global {
            (
                designer_state.tool_settings.feed_rate,
                (designer_state.tool_settings.spindle_speed as f64) / 10.0,
                (designer_state.tool_settings.step_down as u32).max(1),
            )
        } else {
            let fr = panel
                .laser_feed_rate_entry
                .text()
                .trim()
                .parse::<f64>()
                .unwrap_or(1000.0);
            let pw = panel
                .laser_power_entry
                .text()
                .trim()
                .parse::<f64>()
                .unwrap_or(100.0)
                .clamp(0.0, 100.0);
            let ps = panel
                .laser_passes_entry
                .text()
                .trim()
                .parse::<u32>()
                .unwrap_or(1)
                .max(1);
            (fr, pw, ps)
        };

        let params = LaserParams {
            feed_rate,
            power_percent,
            passes,
            use_global,
        };

        change_selected_with_undo(&mut designer_state, "Change Laser Override", |obj| {
            if matches!(obj.shape, Shape::RasterImage(_)) {
                return false;
            }
            let old = obj.laser_params;
            let changed = obj.use_global_laser != use_global
                || old.use_global != use_global
                || (old.feed_rate - feed_rate).abs() > f64::EPSILON
                || (old.power_percent - power_percent).abs() > f64::EPSILON
                || old.passes != passes;
            // DrawingObject fields are what gets serialised; the shape-embedded copy is
            // what 2D toolpath generation reads. Keep both in sync.
            obj.use_global_laser = use_global;
            obj.laser_params = params;
            set_shape_laser_params(&mut obj.shape, params);
            changed
        });
    }

    let cb = panel.redraw_callback.borrow().clone();
    if let Some(cb) = cb {
        cb();
    }
}

/// Connects the laser override checkbox and entries of `panel`.
pub(crate) fn setup_laser_override_handlers(panel: &PropertiesPanel) {
    // "Use global values" checkbox
    {
        let p = panel.clone();
        panel.laser_use_global_check.connect_toggled(move |check| {
            let use_global = check.is_active();

            p.laser_feed_rate_entry.set_sensitive(!use_global);
            p.laser_power_entry.set_sensitive(!use_global);
            p.laser_passes_entry.set_sensitive(!use_global);

            if !*p.updating.borrow() {
                apply_laser_override(&p);
            }
        });
    }

    for entry in [
        &panel.laser_feed_rate_entry,
        &panel.laser_power_entry,
        &panel.laser_passes_entry,
    ] {
        let p = panel.clone();
        connect_entry_commit(
            entry,
            panel.state.clone(),
            panel.updating.clone(),
            panel.has_focus.clone(),
            move |_| apply_laser_override(&p),
        );
    }
}
