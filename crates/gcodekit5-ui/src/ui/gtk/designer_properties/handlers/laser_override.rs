//! Handlers for laser override configuration

use crate::ui::gtk::designer_properties::PropertiesPanel;
use gcodekit5_designer::model::{LaserParams, Shape};
use gtk4::prelude::*;
use std::rc::Rc;

pub(crate) fn setup_laser_override_handlers(panel: &PropertiesPanel) {
    let panel_rc = Rc::new(panel.clone());

    let update_selected_shape = {
        let panel = panel_rc.clone();
        move || {
            let use_global = panel.laser_use_global_check.is_active();

            {
                let mut designer_state = panel.state.borrow_mut();

                let (feed_rate, power, passes) = if use_global {
                    let global_feed = designer_state.tool_settings.feed_rate;
                    let global_power = designer_state.tool_settings.spindle_speed as f64;
                    let global_passes = (designer_state.tool_settings.step_down as u32).max(1);
                    (global_feed, global_power, global_passes)
                } else {
                    let fr = panel.laser_feed_rate_entry.text().parse::<f64>().unwrap_or(1000.0);
                    let pw = panel.laser_power_entry.text().parse::<f64>().unwrap_or(100.0);
                    let ps = panel.laser_passes_entry.text().parse::<u32>().unwrap_or(1);
                    (fr, pw, ps)
                };

                for shape in designer_state.canvas.shapes_mut() {
                    if shape.selected {
                        match &mut shape.shape {
                            Shape::Rectangle(rect) => {
                                rect.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Circle(circle) => {
                                circle.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Ellipse(ellipse) => {
                                ellipse.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Line(line) => {
                                line.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Path(path) => {
                                path.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Polygon(polygon) => {
                                polygon.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Triangle(triangle) => {
                                triangle.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Text(text) => {
                                text.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Gear(gear) => {
                                gear.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::Sprocket(sprocket) => {
                                sprocket.laser_params = LaserParams { feed_rate, power_percent: power, passes, use_global };
                                shape.use_global_laser = use_global;
                            }
                            Shape::RasterImage(_) => {}
                        }
                        break;
                    }
                }
            }

            if let Some(ref cb) = *panel.redraw_callback.borrow() {
                cb();
            }
        }
    };


    // Handler para el checkbox "Use global values"
    let panel_clone = panel_rc.clone();
    let update_fn = update_selected_shape.clone();
    panel.laser_use_global_check.connect_toggled(move |check| {
        let use_global = check.is_active();

        // Habilitar/deshabilitar los inputs visualmente
        panel_clone.laser_feed_rate_entry.set_sensitive(!use_global);
        panel_clone.laser_power_entry.set_sensitive(!use_global);
        panel_clone.laser_passes_entry.set_sensitive(!use_global);

        // Guardar el cambio del checkbox en el modelo de la figura
        if !*panel_clone.updating.borrow() {
            update_fn();
        }
    });

    // Feed rate handler
    let panel_clone = panel_rc.clone();
    let update_fn = update_selected_shape.clone();
    panel.laser_feed_rate_entry.connect_changed(move |_| {
        if !*panel_clone.updating.borrow() {
            update_fn();
        }
    });

    // Power handler
    let panel_clone = panel_rc.clone();
    let update_fn = update_selected_shape.clone();
    panel.laser_power_entry.connect_changed(move |_| {
        if !*panel_clone.updating.borrow() {
            update_fn();
        }
    });

    // Passes handler
    let panel_clone = panel_rc.clone();
    let update_fn = update_selected_shape;
    panel.laser_passes_entry.connect_changed(move |_| {
        if !*panel_clone.updating.borrow() {
            update_fn();
        }
    });

}
