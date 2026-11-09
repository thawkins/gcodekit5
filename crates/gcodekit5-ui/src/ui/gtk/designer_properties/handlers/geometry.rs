//! Geometry property handlers (rotation, corner radius, slot, polygon sides).
//!
//! Text entries apply on commit (Enter / focus-out); see [`super::connect_entry_commit`].

use super::{change_selected_with_undo, connect_entry_commit};
use crate::ui::gtk::designer_properties::SharedMeasurementSystem;
use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::commands::{ChangeProperty, DesignerCommand};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gtk4::prelude::*;
use gtk4::{CheckButton, Entry};
use std::rc::Rc;

/// Invokes the redraw callback (if any) without holding its borrow during the call.
#[allow(clippy::type_complexity)]
fn redraw(redraw_callback: &SharedOption<Rc<dyn Fn()>>) {
    let cb = redraw_callback.borrow().clone();
    if let Some(cb) = cb {
        cb();
    }
}

/// Setup rotation entry handler (applied on Enter / focus-out).
///
/// For a single shape the value is the absolute angle in degrees. For a multi-shape
/// selection the panel shows `0` and the value is the angle to rotate the whole
/// selection by; after the commit the panel refresh resets it to `0`, so the rotation
/// is applied exactly once per edit.
#[allow(clippy::type_complexity)]
pub fn setup_rotation_handler(
    rotation_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(rotation_entry, state, updating, has_focus, move |entry| {
        let Ok(val) = entry.text().trim().replace(',', ".").parse::<f64>() else {
            entry.add_css_class("entry-invalid");
            return;
        };
        entry.remove_css_class("entry-invalid");
        apply_state.borrow_mut().set_selected_rotation(val);
        redraw(&redraw_callback);
    });
}

/// Setup corner radius entry handler (applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_corner_radius_handler(
    corner_radius_entry: &Entry,
    state: Shared<DesignerState>,
    measurement_system: SharedMeasurementSystem,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(
        corner_radius_entry,
        state,
        updating,
        has_focus,
        move |entry| {
            let system = measurement_system.get();
            if let Ok(val) = units::parse_length(&entry.text(), system) {
                entry.remove_css_class("entry-invalid");
                apply_state
                    .borrow_mut()
                    .set_selected_corner_radius(val as f64);
                redraw(&redraw_callback);
            } else {
                entry.add_css_class("entry-invalid");
            }
        },
    );
}

/// Setup is-slot checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_is_slot_handler(
    is_slot_check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    is_slot_check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }
        let mut designer_state = state.borrow_mut();
        designer_state.set_selected_is_slot(check.is_active());
        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}

/// Setup polygon sides entry handler (applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_sides_handler(
    sides_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(
        sides_entry,
        state,
        updating,
        has_focus,
        move |entry| match entry.text().trim().parse::<u32>() {
            Ok(val) if val >= 3 => {
                entry.remove_css_class("entry-invalid");
                {
                    let mut designer_state = apply_state.borrow_mut();
                    change_selected_with_undo(&mut designer_state, "Change Polygon Sides", |obj| {
                        if let Shape::Polygon(polygon) = &mut obj.shape {
                            let changed = polygon.sides != val;
                            polygon.sides = val;
                            changed
                        } else {
                            false
                        }
                    });
                }
                redraw(&redraw_callback);
            }
            _ => entry.add_css_class("entry-invalid"),
        },
    );
}

/// Setup path closed checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_path_closed_handler(
    path_closed_check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
) {
    path_closed_check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }

        let mut designer_state = state.borrow_mut();
        if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
            if let Some(obj) = designer_state.canvas.get_shape(id) {
                let old_obj = obj.clone();
                let mut new_obj = old_obj.clone();
                if let Shape::Path(path) = &mut new_obj.shape {
                    path.set_closed(check.is_active());
                    let cmd = DesignerCommand::ChangeProperty(ChangeProperty {
                        id,
                        old_state: old_obj,
                        new_state: new_obj,
                    });
                    designer_state.push_command(cmd);
                }
            }
        }

        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}
