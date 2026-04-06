//! Dimension property handlers (position, size, aspect ratio).

use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{CheckButton, Entry};
use std::rc::Rc;

/// Setup position X entry handler
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_position_x_handler(
    pos_x_entry: &Entry,
    pos_y_entry: &Entry,
    width_entry: &Entry,
    height_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    let pos_y = pos_y_entry.clone();
    let width = width_entry.clone();
    let height = height_entry.clone();

    pos_x_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            let selected_count = designer_state.selected_count();

            if selected_count >= 1 {
                let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;
                let w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;
                let h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;
                let x = val as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, true, false);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup position Y entry handler
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_position_y_handler(
    pos_y_entry: &Entry,
    pos_x_entry: &Entry,
    width_entry: &Entry,
    height_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    let pos_x = pos_x_entry.clone();
    let width = width_entry.clone();
    let height = height_entry.clone();

    pos_y_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            let selected_count = designer_state.selected_count();

            if selected_count >= 1 {
                let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                let w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;
                let h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;
                let y = val as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, true, false);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup width entry activate handler
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_width_activate_handler(
    width_entry: &Entry,
    height_entry: &Entry,
    pos_x_entry: &Entry,
    pos_y_entry: &Entry,
    lock_aspect_ratio: &CheckButton,
    aspect_ratio: Shared<f64>,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    let height = height_entry.clone();
    let pos_x = pos_x_entry.clone();
    let pos_y = pos_y_entry.clone();
    let lock_aspect_check = lock_aspect_ratio.clone();

    width_entry.connect_activate(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            let selected_count = designer_state.selected_count();

            let w = val as f64;
            let mut h = units::parse_length(&height.text(), system).unwrap_or(0.0) as f64;

            // If aspect ratio is locked, adjust height to maintain ratio
            if lock_aspect_check.is_active() {
                let ratio = *aspect_ratio.borrow();
                if ratio > 0.0 {
                    h = w / ratio;
                    *updating.borrow_mut() = true;
                    height.set_text(&units::format_length(h as f32, system));
                    *updating.borrow_mut() = false;
                }
            }

            if selected_count >= 1 {
                let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, false, true);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup height entry activate handler
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_height_activate_handler(
    height_entry: &Entry,
    width_entry: &Entry,
    pos_x_entry: &Entry,
    pos_y_entry: &Entry,
    lock_aspect_ratio: &CheckButton,
    aspect_ratio: Shared<f64>,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    let width = width_entry.clone();
    let pos_x = pos_x_entry.clone();
    let pos_y = pos_y_entry.clone();
    let lock_aspect_check = lock_aspect_ratio.clone();

    height_entry.connect_activate(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            let selected_count = designer_state.selected_count();

            let h = val as f64;
            let mut w = units::parse_length(&width.text(), system).unwrap_or(0.0) as f64;

            // If aspect ratio is locked, adjust width to maintain ratio
            if lock_aspect_check.is_active() {
                let ratio = *aspect_ratio.borrow();
                if ratio > 0.0 {
                    w = h * ratio;
                    *updating.borrow_mut() = true;
                    width.set_text(&units::format_length(w as f32, system));
                    *updating.borrow_mut() = false;
                }
            }

            if selected_count >= 1 {
                let x = units::parse_length(&pos_x.text(), system).unwrap_or(0.0) as f64;
                let y = units::parse_length(&pos_y.text(), system).unwrap_or(0.0) as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, false, true);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup lock aspect ratio toggle handler
pub fn setup_lock_aspect_handler(
    lock_aspect_ratio: &CheckButton,
    width_entry: &Entry,
    height_entry: &Entry,
    aspect_ratio: Shared<f64>,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
) {
    let width = width_entry.clone();
    let height = height_entry.clone();

    lock_aspect_ratio.connect_toggled(move |check| {
        let is_active = check.is_active();

        if is_active {
            // Store current aspect ratio when locked
            let system = settings.borrow().config().ui.measurement_system;
            if let (Ok(w), Ok(h)) = (
                units::parse_length(&width.text(), system),
                units::parse_length(&height.text(), system),
            ) {
                if h > 0.0 {
                    *aspect_ratio.borrow_mut() = w as f64 / h as f64;
                }
            }
        }

        // Save lock_aspect_ratio to the selected shape(s)
        let mut designer_state = state.borrow_mut();
        let selected_ids: Vec<u64> = designer_state
            .canvas
            .shapes()
            .filter(|s| s.selected)
            .map(|s| s.id)
            .collect();

        for id in selected_ids {
            if let Some(obj) = designer_state.canvas.get_shape_mut(id) {
                obj.lock_aspect_ratio = is_active;
            }
        }
    });
}

/// Setup width focus-out handler to apply changes when tabbing away
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_width_focus_out_handler(
    width_entry: &Entry,
    height_entry: &Entry,
    pos_x_entry: &Entry,
    pos_y_entry: &Entry,
    lock_aspect_ratio: &CheckButton,
    aspect_ratio: Shared<f64>,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    use gtk4::EventControllerFocus;

    let focus_controller = EventControllerFocus::new();
    let width_state = state;
    let width_settings = settings;
    let width_pos_x = pos_x_entry.clone();
    let width_pos_y = pos_y_entry.clone();
    let width_height = height_entry.clone();
    let width_redraw = redraw_callback;
    let width_updating = updating;
    let width_entry_clone = width_entry.clone();
    let width_lock_aspect = lock_aspect_ratio.clone();
    let width_aspect_ratio = aspect_ratio;

    focus_controller.connect_leave(move |_| {
        if *width_updating.borrow() {
            return;
        }
        let system = width_settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&width_entry_clone.text(), system) {
            width_entry_clone.remove_css_class("entry-invalid");
            let mut designer_state = width_state.borrow_mut();
            let selected_count = designer_state.selected_count();

            let w = val as f64;
            let mut h = units::parse_length(&width_height.text(), system).unwrap_or(0.0) as f64;

            // If aspect ratio is locked, adjust height to maintain ratio
            if width_lock_aspect.is_active() {
                let ratio = *width_aspect_ratio.borrow();
                if ratio > 0.0 {
                    h = w / ratio;
                    *width_updating.borrow_mut() = true;
                    width_height.set_text(&units::format_length(h as f32, system));
                    *width_updating.borrow_mut() = false;
                }
            }

            if selected_count >= 1 {
                let x = units::parse_length(&width_pos_x.text(), system).unwrap_or(0.0) as f64;
                let y = units::parse_length(&width_pos_y.text(), system).unwrap_or(0.0) as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, false, true);
            }
            drop(designer_state);
            if let Some(ref cb) = *width_redraw.borrow() {
                cb();
            }
        } else {
            width_entry_clone.add_css_class("entry-invalid");
        }
    });
    width_entry.add_controller(focus_controller);
}

/// Setup height focus-out handler to apply changes when tabbing away
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn setup_height_focus_out_handler(
    height_entry: &Entry,
    width_entry: &Entry,
    pos_x_entry: &Entry,
    pos_y_entry: &Entry,
    lock_aspect_ratio: &CheckButton,
    aspect_ratio: Shared<f64>,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    use gtk4::EventControllerFocus;

    let focus_controller = EventControllerFocus::new();
    let height_state = state;
    let height_settings = settings;
    let height_pos_x = pos_x_entry.clone();
    let height_pos_y = pos_y_entry.clone();
    let height_width = width_entry.clone();
    let height_redraw = redraw_callback;
    let height_updating = updating;
    let height_entry_clone = height_entry.clone();
    let height_lock_aspect = lock_aspect_ratio.clone();
    let height_aspect_ratio = aspect_ratio;

    focus_controller.connect_leave(move |_| {
        if *height_updating.borrow() {
            return;
        }
        let system = height_settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&height_entry_clone.text(), system) {
            height_entry_clone.remove_css_class("entry-invalid");
            let mut designer_state = height_state.borrow_mut();
            let selected_count = designer_state.selected_count();

            let h = val as f64;
            let mut w = units::parse_length(&height_width.text(), system).unwrap_or(0.0) as f64;

            // If aspect ratio is locked, adjust width to maintain ratio
            if height_lock_aspect.is_active() {
                let ratio = *height_aspect_ratio.borrow();
                if ratio > 0.0 {
                    w = h * ratio;
                    *height_updating.borrow_mut() = true;
                    height_width.set_text(&units::format_length(w as f32, system));
                    *height_updating.borrow_mut() = false;
                }
            }

            if selected_count >= 1 {
                let x = units::parse_length(&height_pos_x.text(), system).unwrap_or(0.0) as f64;
                let y = units::parse_length(&height_pos_y.text(), system).unwrap_or(0.0) as f64;

                designer_state.set_selected_position_and_size_with_flags(x, y, w, h, false, true);
            }
            drop(designer_state);
            if let Some(ref cb) = *height_redraw.borrow() {
                cb();
            }
        } else {
            height_entry_clone.add_css_class("entry-invalid");
        }
    });
    height_entry.add_controller(focus_controller);
}
