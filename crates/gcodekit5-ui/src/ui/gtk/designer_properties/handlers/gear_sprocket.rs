//! Gear and sprocket property handlers.

use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{Shape, DesignSprocket};
use gtk4::prelude::*;
use gtk4::Entry;
use std::rc::Rc;

/// Setup gear pressure angle entry handler
#[allow(clippy::type_complexity)]
pub fn setup_gear_pressure_angle_handler(
    gear_pressure_angle_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    gear_pressure_angle_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val_deg) = entry.text().parse::<f64>() {
            if val_deg > 0.0 && val_deg < 45.0 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let module = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.module)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(2.0);
                let teeth = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.teeth)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(20);

                designer_state.set_selected_gear_properties(module, teeth, val_deg);
                drop(designer_state);
                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup gear module handler
pub fn setup_gear_module_handler(
    gear_module_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    gear_module_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<f64>() {
            if val > 0.0 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let teeth = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.teeth)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(20);
                let pa_deg = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.pressure_angle_deg)  // Ya en grados
                        } else {
                            None
                        }
                    })
                    .unwrap_or(20.0);

                designer_state.set_selected_gear_properties(val, teeth, pa_deg);
                drop(designer_state);
                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup gear teeth handler
pub fn setup_gear_teeth_handler(
    gear_teeth_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    gear_teeth_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<usize>() {
            if val >= 3 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let module = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.module)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(2.0);
                let pa_deg = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Gear(g) = &s.shape {
                            Some(g.pressure_angle_deg)  // Ya en grados
                        } else {
                            None
                        }
                    })
                    .unwrap_or(20.0);

                designer_state.set_selected_gear_properties(module, val, pa_deg);
                drop(designer_state);
                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup sprocket pitch entry handler
#[allow(clippy::type_complexity)]
pub fn setup_sprocket_pitch_handler(
    sprocket_pitch_entry: &Entry,
    sprocket_roller_diameter_entry: &Entry,  // NUEVO parámetro
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    let roller_entry = sprocket_roller_diameter_entry.clone();
    sprocket_pitch_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(pitch) = entry.text().parse::<f64>() {
            if pitch > 0.0 {
                entry.remove_css_class("entry-invalid");

                let standard_roller = DesignSprocket::standard_roller_diameter(pitch);

                // Actualizar el campo visual del roller diameter
                roller_entry.set_text(&format!("{:.2}", standard_roller));

                let mut designer_state = state.borrow_mut();
                let teeth = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Sprocket(sp) = &s.shape {
                            Some(sp.teeth)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(15);

                designer_state.set_selected_sprocket_properties(pitch, teeth, standard_roller);
                drop(designer_state);

                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup sprocket teeth entry handler
#[allow(clippy::type_complexity)]
pub fn setup_sprocket_teeth_handler(
    sprocket_teeth_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    sprocket_teeth_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<usize>() {
            if val >= 3 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let pitch = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Sprocket(sp) = &s.shape {
                            Some(sp.pitch)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(12.7);
                let roller = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Sprocket(sp) = &s.shape {
                            Some(sp.roller_diameter)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(7.75);

                designer_state.set_selected_sprocket_properties(pitch, val, roller);
                drop(designer_state);
                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup sprocket roller diameter entry handler
#[allow(clippy::type_complexity)]
pub fn setup_sprocket_roller_diameter_handler(
    sprocket_roller_diameter_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    sprocket_roller_diameter_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<f64>() {
            if val > 0.0 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();
                let pitch = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Sprocket(sp) = &s.shape {
                            Some(sp.pitch)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(12.7);
                let teeth = designer_state
                    .canvas
                    .shapes()
                    .find(|s| s.selected)
                    .and_then(|s| {
                        if let Shape::Sprocket(sp) = &s.shape {
                            Some(sp.teeth)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(15);

                designer_state.set_selected_sprocket_properties(pitch, teeth, val);
                drop(designer_state);
                if let Some(ref cb) = *redraw_callback.borrow() {
                    cb();
                }
            } else {
                entry.add_css_class("entry-invalid");
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}
