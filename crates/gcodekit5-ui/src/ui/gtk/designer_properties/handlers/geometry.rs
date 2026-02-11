//! Geometry property handlers (rotation, corner radius, slot, polygon sides).

use gcodekit5_core::units;
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{ChangeProperty, DesignerCommand};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{CheckButton, Entry};
use std::cell::RefCell;
use std::rc::Rc;

fn modify_selected_shape_with_undo<F>(designer_state: &mut DesignerState, modifier: F)
where
    F: FnOnce(&mut DrawingObject),
{
    if let Some(id) = designer_state.canvas.selection_manager.selected_id() {
        if let Some(old_obj) = designer_state.canvas.get_shape(id) {
            let old_obj = old_obj.clone();

            if let Some(obj) = designer_state.canvas.get_shape_mut(id) {
                modifier(obj);
                let new_obj = obj.clone();

                let cmd = DesignerCommand::ChangeProperty(ChangeProperty {
                    id,
                    old_state: old_obj,
                    new_state: new_obj,
                });
                designer_state.record_command(cmd);
            }
        }
    }
}

/// Setup rotation entry handler
#[allow(clippy::type_complexity)]
pub fn setup_rotation_handler(
    rotation_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    rotation_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_rotation(val);
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup corner radius entry handler
#[allow(clippy::type_complexity)]
pub fn setup_corner_radius_handler(
    corner_radius_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    settings: Rc<RefCell<SettingsPersistence>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    corner_radius_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_corner_radius(val as f64);
            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup is-slot checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_is_slot_handler(
    is_slot_check: &CheckButton,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
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

/// Setup polygon sides entry handler
#[allow(clippy::type_complexity)]
pub fn setup_sides_handler(
    sides_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    sides_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<u32>() {
            if val >= 3 {
                entry.remove_css_class("entry-invalid");
                let mut designer_state = state.borrow_mut();

                modify_selected_shape_with_undo(&mut designer_state, |obj| {
                    if let Shape::Polygon(polygon) = &mut obj.shape {
                        polygon.sides = val;
                    }
                });

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
