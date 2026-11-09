//! Effects property handlers (offset, chamfer with live preview).

use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption, SharedVec};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{Entry, EventControllerFocus};
use std::rc::Rc;

/// Smallest difference (mm) that counts as a real edit, given that values are shown
/// with 3 decimals in the active measurement system; anything below is display rounding.
fn edit_tolerance_mm(system: units::MeasurementSystem) -> f64 {
    match system {
        units::MeasurementSystem::Metric => 0.0005 + 1e-9,
        units::MeasurementSystem::Imperial => 0.0005 * 25.4 + 1e-9,
    }
}

/// Setup offset entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_offset_handler(
    offset_entry: &Entry,
    state: Shared<DesignerState>,
    preview_shapes: SharedVec<Shape>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let preview = preview_shapes.clone();
    let redraw = redraw_callback.clone();
    let offset_entry_clone = offset_entry.clone();
    let state_clone = state.clone();
    let updating_clone = updating.clone();

    // Live preview on change
    let update_preview = {
        let state = state_clone.clone();
        let preview = preview.clone();
        let redraw = redraw.clone();
        let offset_entry = offset_entry_clone.clone();
        let updating = updating_clone.clone();
        let settings = settings.clone();
        move || {
            if *updating.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(dist) = units::parse_length(&offset_entry.text(), system).map(f64::from) {
                offset_entry.remove_css_class("entry-invalid");
                let designer_state = state.borrow();
                let preview_shapes: Vec<Shape> = designer_state
                    .canvas
                    .shapes()
                    .filter(|s| s.selected)
                    .map(|s| gcodekit5_designer::ops::perform_offset(&s.shape, dist))
                    .collect();
                *preview.borrow_mut() = preview_shapes;
            } else {
                offset_entry.add_css_class("entry-invalid");
                preview.borrow_mut().clear();
            }
            if let Some(ref cb) = *redraw.borrow() {
                cb();
            }
        }
    };

    let update_preview_clone = update_preview.clone();
    offset_entry.connect_changed(move |_| {
        update_preview_clone();
    });

    // Apply on activate (Enter)
    let state_activate = state.clone();
    let preview_activate = preview_shapes.clone();
    let redraw_activate = redraw_callback.clone();
    let offset_entry_activate = offset_entry.clone();
    let settings_activate = settings.clone();
    offset_entry.connect_activate(move |_| {
        let system = settings_activate.borrow().config().ui.measurement_system;
        if let Ok(dist) = units::parse_length(&offset_entry_activate.text(), system).map(f64::from) {
            preview_activate.borrow_mut().clear();
            let mut designer_state = state_activate.borrow_mut();
            // Only record a change when the value actually differs; otherwise merely
            // tabbing through the field would push no-op undo entries (and for chamfer,
            // reset a path's fillet).
            let changed = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .any(|s| (s.offset - dist).abs() > edit_tolerance_mm(system));
            if changed {
                designer_state.set_offset_selected(dist);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_activate.borrow() {
                cb();
            }
        }
    });

    // Apply on focus out
    let focus_controller = EventControllerFocus::new();
    let state_focus = state.clone();
    let preview_focus = preview_shapes.clone();
    let redraw_focus = redraw_callback.clone();
    let offset_entry_focus = offset_entry.clone();
    let settings_focus = settings.clone();
    let has_focus_enter = has_focus.clone();
    focus_controller.connect_enter(move |_| {
        *has_focus_enter.borrow_mut() = true;
    });
    let has_focus_leave = has_focus.clone();
    focus_controller.connect_leave(move |_| {
        *has_focus_leave.borrow_mut() = false;
        let system = settings_focus.borrow().config().ui.measurement_system;
        if let Ok(dist) = units::parse_length(&offset_entry_focus.text(), system).map(f64::from) {
            preview_focus.borrow_mut().clear();
            let mut designer_state = state_focus.borrow_mut();
            // Only record a change when the value actually differs; otherwise merely
            // tabbing through the field would push no-op undo entries (and for chamfer,
            // reset a path's fillet).
            let changed = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .any(|s| (s.offset - dist).abs() > edit_tolerance_mm(system));
            if changed {
                designer_state.set_offset_selected(dist);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_focus.borrow() {
                cb();
            }
        }
    });
    offset_entry.add_controller(focus_controller);
}

/// Setup chamfer entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_chamfer_handler(
    chamfer_entry: &Entry,
    state: Shared<DesignerState>,
    preview_shapes: SharedVec<Shape>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let preview = preview_shapes.clone();
    let redraw = redraw_callback.clone();
    let chamfer_entry_clone = chamfer_entry.clone();
    let state_clone = state.clone();
    let updating_clone = updating.clone();

    // Live preview on change
    let update_preview = {
        let state = state_clone.clone();
        let preview = preview.clone();
        let redraw = redraw.clone();
        let chamfer_entry = chamfer_entry_clone.clone();
        let updating = updating_clone.clone();
        let settings = settings.clone();
        move || {
            if *updating.borrow() {
                return;
            }
            let system = settings.borrow().config().ui.measurement_system;
            if let Ok(dist) = units::parse_length(&chamfer_entry.text(), system).map(f64::from) {
                chamfer_entry.remove_css_class("entry-invalid");
                let designer_state = state.borrow();
                let preview_shapes: Vec<Shape> = designer_state
                    .canvas
                    .shapes()
                    .filter(|s| s.selected)
                    .map(|s| gcodekit5_designer::ops::perform_chamfer(&s.shape, dist))
                    .collect();
                *preview.borrow_mut() = preview_shapes;
            } else {
                chamfer_entry.add_css_class("entry-invalid");
                preview.borrow_mut().clear();
            }
            if let Some(ref cb) = *redraw.borrow() {
                cb();
            }
        }
    };

    let update_preview_clone = update_preview.clone();
    chamfer_entry.connect_changed(move |_| {
        update_preview_clone();
    });

    // Apply on activate (Enter)
    let state_activate = state.clone();
    let preview_activate = preview_shapes.clone();
    let redraw_activate = redraw_callback.clone();
    let chamfer_entry_activate = chamfer_entry.clone();
    let settings_activate = settings.clone();
    chamfer_entry.connect_activate(move |_| {
        let system = settings_activate.borrow().config().ui.measurement_system;
        if let Ok(dist) = units::parse_length(&chamfer_entry_activate.text(), system).map(f64::from) {
            preview_activate.borrow_mut().clear();
            let mut designer_state = state_activate.borrow_mut();
            // Only record a change when the value actually differs; otherwise merely
            // tabbing through the field would push no-op undo entries (and for chamfer,
            // reset a path's fillet).
            let changed = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .any(|s| (s.chamfer - dist).abs() > edit_tolerance_mm(system));
            if changed {
                designer_state.set_chamfer_selected(dist);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_activate.borrow() {
                cb();
            }
        }
    });

    // Apply on focus out
    let focus_controller = EventControllerFocus::new();
    let state_focus = state.clone();
    let preview_focus = preview_shapes.clone();
    let redraw_focus = redraw_callback.clone();
    let chamfer_entry_focus = chamfer_entry.clone();
    let settings_focus = settings.clone();
    let has_focus_enter = has_focus.clone();
    focus_controller.connect_enter(move |_| {
        *has_focus_enter.borrow_mut() = true;
    });
    let has_focus_leave = has_focus.clone();
    focus_controller.connect_leave(move |_| {
        *has_focus_leave.borrow_mut() = false;
        let system = settings_focus.borrow().config().ui.measurement_system;
        if let Ok(dist) = units::parse_length(&chamfer_entry_focus.text(), system).map(f64::from) {
            preview_focus.borrow_mut().clear();
            let mut designer_state = state_focus.borrow_mut();
            // Only record a change when the value actually differs; otherwise merely
            // tabbing through the field would push no-op undo entries (and for chamfer,
            // reset a path's fillet).
            let changed = designer_state
                .canvas
                .shapes()
                .filter(|s| s.selected)
                .any(|s| (s.chamfer - dist).abs() > edit_tolerance_mm(system));
            if changed {
                designer_state.set_chamfer_selected(dist);
            }
            drop(designer_state);
            if let Some(ref cb) = *redraw_focus.borrow() {
                cb();
            }
        }
    });
    chamfer_entry.add_controller(focus_controller);
}
