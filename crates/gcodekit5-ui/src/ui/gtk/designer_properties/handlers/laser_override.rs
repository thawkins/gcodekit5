//! Handlers for laser override configuration

use crate::ui::gtk::designer_properties::PropertiesPanel;
use gcodekit5_designer::designer_state::ToolSettings;
use gcodekit5_designer::model::{LaserParams, Shape};
use gtk4::prelude::*;
use std::rc::Rc;

/// Aplica un cambio a `laser_params` de TODOS los shapes seleccionados.
fn apply_to_selected<F>(panel: &PropertiesPanel, mut f: F, set_use_global: Option<bool>)
where
    F: FnMut(&mut LaserParams),
{
    let mut state = panel.state.borrow_mut();

    // Valores globales
    let global: Option<ToolSettings> = if set_use_global == Some(true) {
        Some(state.tool_settings.clone())
    } else {
        None
    };

    for shape in state.canvas.shapes_mut() {
        if !shape.selected {
            continue;
        }

        // 1) Actualizar el LaserParams del DrawingObject
        if let Some(true) = set_use_global {
            if let Some(g) = &global {
                shape.laser_params = LaserParams {
                    feed_rate: g.feed_rate,
                    power_percent: (g.spindle_speed as f64) / 10.0,
                    passes: (g.step_down as u32).max(1),
                    use_global: true,
                };
            }
            shape.use_global_laser = true;
        } else if let Some(false) = set_use_global {
            shape.use_global_laser = false;
        }

        if set_use_global != Some(true) {
            f(&mut shape.laser_params);
        }
        shape.laser_params.use_global = shape.use_global_laser;

        // 2) Reflejar en el LaserParams embebido en la Shape concreta
        let lp = shape.laser_params;
        match &mut shape.shape {
            Shape::Rectangle(r) => r.laser_params = lp,
            Shape::Circle(c) => c.laser_params = lp,
            Shape::Ellipse(e) => e.laser_params = lp,
            Shape::Line(l) => l.laser_params = lp,
            Shape::Path(p) => p.laser_params = lp,
            Shape::Polygon(p) => p.laser_params = lp,
            Shape::Triangle(t) => t.laser_params = lp,
            Shape::Text(t) => t.laser_params = lp,
            Shape::Gear(g) => g.laser_params = lp,
            Shape::Sprocket(s) => s.laser_params = lp,
            Shape::RasterImage(_) => {}
        }
    }

    drop(state);

    if let Some(cb) = &*panel.redraw_callback.borrow() {
        cb();
    }
}

pub(crate) fn setup_laser_override_handlers(panel: &PropertiesPanel) {
    let panel_rc = Rc::new(panel.clone());

    // --- Checkbox "usar global" ---
    {
        let panel_widget = panel_rc.clone();
        let panel_move = panel_rc.clone();
        panel_widget
            .laser_use_global_check
            .connect_toggled(move |check| {
                let use_global = check.is_active();

                // Habilitar/deshabilitar inputs
                panel_move
                    .laser_feed_rate_entry
                    .set_sensitive(!use_global);
                panel_move
                    .laser_power_entry
                    .set_sensitive(!use_global);
                panel_move
                    .laser_passes_entry
                    .set_sensitive(!use_global);

                if *panel_move.updating.borrow() {
                    return;
                }

                apply_to_selected(&panel_move, |_| {}, Some(use_global));
            });
    }

    // --- Feed rate ---
    {
        let panel_widget = panel_rc.clone();
        let panel_move = panel_rc.clone();
        panel_widget
            .laser_feed_rate_entry
            .connect_changed(move |entry| {
                if *panel_move.updating.borrow() {
                    return;
                }
                let Ok(v) = entry.text().parse::<f64>() else {
                    return;
                };
                apply_to_selected(&panel_move, |lp| lp.feed_rate = v, Some(false));
            });
    }

    // --- Power ---
    {
        let panel_widget = panel_rc.clone();
        let panel_move = panel_rc.clone();
        panel_widget
            .laser_power_entry
            .connect_changed(move |entry| {
                if *panel_move.updating.borrow() {
                    return;
                }
                let Ok(v) = entry.text().parse::<f64>() else {
                    return;
                };
                apply_to_selected(&panel_move, |lp| lp.power_percent = v, Some(false));
            });
    }

    // --- Passes ---
    {
        let panel_widget = panel_rc.clone();
        let panel_move = panel_rc.clone();
        panel_widget
            .laser_passes_entry
            .connect_changed(move |entry| {
                if *panel_move.updating.borrow() {
                    return;
                }
                let Ok(v) = entry.text().parse::<u32>() else {
                    return;
                };
                apply_to_selected(&panel_move, |lp| lp.passes = v, Some(false));
            });
    }
}

