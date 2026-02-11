//! Effects property handlers (offset, fillet, chamfer with live preview).

use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gtk4::prelude::*;
use gtk4::{Entry, EventControllerFocus};
use std::cell::RefCell;
use std::rc::Rc;

/// Setup offset entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_offset_handler(
    offset_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    preview_shapes: Rc<RefCell<Vec<Shape>>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
    has_focus: Rc<RefCell<bool>>,
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
        move || {
            if *updating.borrow() {
                return;
            }
            if let Ok(dist) = offset_entry.text().parse::<f64>() {
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
    offset_entry.connect_activate(move |_| {
        if let Ok(dist) = offset_entry_activate.text().parse::<f64>() {
            let mut designer_state = state_activate.borrow_mut();
            designer_state.set_offset_selected(dist);
            preview_activate.borrow_mut().clear();
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
    let has_focus_enter = has_focus.clone();
    focus_controller.connect_enter(move |_| {
        *has_focus_enter.borrow_mut() = true;
    });
    let has_focus_leave = has_focus.clone();
    focus_controller.connect_leave(move |_| {
        *has_focus_leave.borrow_mut() = false;
        if let Ok(dist) = offset_entry_focus.text().parse::<f64>() {
            let mut designer_state = state_focus.borrow_mut();
            designer_state.set_offset_selected(dist);
            preview_focus.borrow_mut().clear();
            drop(designer_state);
            if let Some(ref cb) = *redraw_focus.borrow() {
                cb();
            }
        }
    });
    offset_entry.add_controller(focus_controller);
}

/// Setup fillet entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_fillet_handler(
    fillet_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    preview_shapes: Rc<RefCell<Vec<Shape>>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
    has_focus: Rc<RefCell<bool>>,
) {
    let preview = preview_shapes.clone();
    let redraw = redraw_callback.clone();
    let fillet_entry_clone = fillet_entry.clone();
    let state_clone = state.clone();
    let updating_clone = updating.clone();

    // Live preview on change
    let update_preview = {
        let state = state_clone.clone();
        let preview = preview.clone();
        let redraw = redraw.clone();
        let fillet_entry = fillet_entry_clone.clone();
        let updating = updating_clone.clone();
        move || {
            if *updating.borrow() {
                return;
            }
            if let Ok(radius) = fillet_entry.text().parse::<f64>() {
                fillet_entry.remove_css_class("entry-invalid");
                let designer_state = state.borrow();
                let preview_shapes: Vec<Shape> = designer_state
                    .canvas
                    .shapes()
                    .filter(|s| s.selected)
                    .map(|s| gcodekit5_designer::ops::perform_fillet(&s.shape, radius))
                    .collect();
                *preview.borrow_mut() = preview_shapes;
            } else {
                fillet_entry.add_css_class("entry-invalid");
                preview.borrow_mut().clear();
            }
            if let Some(ref cb) = *redraw.borrow() {
                cb();
            }
        }
    };

    let update_preview_clone = update_preview.clone();
    fillet_entry.connect_changed(move |_| {
        update_preview_clone();
    });

    // Apply on activate (Enter)
    let state_activate = state.clone();
    let preview_activate = preview_shapes.clone();
    let redraw_activate = redraw_callback.clone();
    let fillet_entry_activate = fillet_entry.clone();
    fillet_entry.connect_activate(move |_| {
        if let Ok(radius) = fillet_entry_activate.text().parse::<f64>() {
            let mut designer_state = state_activate.borrow_mut();
            designer_state.set_fillet_selected(radius);
            preview_activate.borrow_mut().clear();
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
    let fillet_entry_focus = fillet_entry.clone();
    let has_focus_enter = has_focus.clone();
    focus_controller.connect_enter(move |_| {
        *has_focus_enter.borrow_mut() = true;
    });
    let has_focus_leave = has_focus.clone();
    focus_controller.connect_leave(move |_| {
        *has_focus_leave.borrow_mut() = false;
        if let Ok(radius) = fillet_entry_focus.text().parse::<f64>() {
            let mut designer_state = state_focus.borrow_mut();
            designer_state.set_fillet_selected(radius);
            preview_focus.borrow_mut().clear();
            drop(designer_state);
            if let Some(ref cb) = *redraw_focus.borrow() {
                cb();
            }
        }
    });
    fillet_entry.add_controller(focus_controller);
}

/// Setup chamfer entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_chamfer_handler(
    chamfer_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    preview_shapes: Rc<RefCell<Vec<Shape>>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
    has_focus: Rc<RefCell<bool>>,
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
        move || {
            if *updating.borrow() {
                return;
            }
            if let Ok(dist) = chamfer_entry.text().parse::<f64>() {
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
    chamfer_entry.connect_activate(move |_| {
        if let Ok(dist) = chamfer_entry_activate.text().parse::<f64>() {
            let mut designer_state = state_activate.borrow_mut();
            designer_state.set_chamfer_selected(dist);
            preview_activate.borrow_mut().clear();
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
    let has_focus_enter = has_focus.clone();
    focus_controller.connect_enter(move |_| {
        *has_focus_enter.borrow_mut() = true;
    });
    let has_focus_leave = has_focus.clone();
    focus_controller.connect_leave(move |_| {
        *has_focus_leave.borrow_mut() = false;
        if let Ok(dist) = chamfer_entry_focus.text().parse::<f64>() {
            let mut designer_state = state_focus.borrow_mut();
            designer_state.set_chamfer_selected(dist);
            preview_focus.borrow_mut().clear();
            drop(designer_state);
            if let Some(ref cb) = *redraw_focus.borrow() {
                cb();
            }
        }
    });
    chamfer_entry.add_controller(focus_controller);
}
