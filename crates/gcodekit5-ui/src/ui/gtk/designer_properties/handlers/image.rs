//! Image engraving property handlers for the properties panel.

use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gtk4::prelude::*;
use gtk4::{CheckButton, ComboBoxText, Entry};
use std::rc::Rc;

/// Setup handler for image feed rate entry
pub fn setup_feed_rate_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }

        if let Ok(value) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let value = value.max(0.0);
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.feed_rate = value;
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup handler for image travel rate entry
pub fn setup_travel_rate_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }

        if let Ok(value) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let value = value.max(0.0);
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.travel_rate = value;
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup handler for image min power entry
pub fn setup_min_power_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }

        if let Ok(value) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let value = value.clamp(0.0, 100.0);
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.min_power = value;
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup handler for image max power entry
pub fn setup_max_power_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }

        if let Ok(value) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let value = value.clamp(0.0, 100.0);
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.max_power = value;
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup handler for image PPI entry
pub fn setup_ppi_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }

        if let Ok(value) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let value = value.clamp(1.0, 1000.0);
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.ppi = value;
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup handler for image scan direction combo box
pub fn setup_scan_direction_handler(
    combo: &ComboBoxText,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    combo.connect_changed(move |combo| {
        if *updating.borrow() {
            return;
        }

        if let Some(direction) = combo.active_id() {
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.scan_direction = direction.to_string();
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        }
    });
}

/// Setup handler for image bidirectional check button
pub fn setup_bidirectional_handler(
    check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }

        let bidirectional = check.is_active();
        let mut designer_state = state.borrow_mut();

        for shape in designer_state.canvas.shapes_mut() {
            if shape.selected {
                if let Shape::RasterImage(img) = &mut shape.shape {
                    img.bidirectional = bidirectional;
                    break;
                }
            }
        }
        drop(designer_state);

        if let Some(cb) = redraw_callback.borrow().as_ref() {
            cb();
        }
    });
}

pub fn setup_invert_handler(
    check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {

// --- Invert
    check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }

        // Obtain button state (true/false)
        let invert = check.is_active();
        let mut designer_state = state.borrow_mut();

        for shape in designer_state.canvas.shapes_mut() {
            if shape.selected {
                if let Shape::RasterImage(img) = &mut shape.shape {
                    img.invert = invert;
                    break;
                }
            }
        }
// ---

        drop(designer_state);

        if let Some(cb) = redraw_callback.borrow().as_ref() {
            cb();
        }
    });
}

/// Setup handler for image dithering combo box
pub fn setup_dithering_handler(
    combo: &ComboBoxText,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    combo.connect_changed(move |combo| {
        if *updating.borrow() {
            return;
        }

        if let Some(dithering) = combo.active_id() {
            let mut designer_state = state.borrow_mut();

            for shape in designer_state.canvas.shapes_mut() {
                if shape.selected {
                    if let Shape::RasterImage(img) = &mut shape.shape {
                        img.dithering = dithering.to_string();
                        break;
                    }
                }
            }
            drop(designer_state);

            if let Some(cb) = redraw_callback.borrow().as_ref() {
                cb();
            }
        }
    });
}
