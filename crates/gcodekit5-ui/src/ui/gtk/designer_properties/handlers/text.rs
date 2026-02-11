//! Text property handlers (content, font family, size, bold, italic).

use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::commands::{ChangeProperty, DesignerCommand};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
use gtk4::prelude::*;
use gtk4::{CheckButton, DropDown, Entry};
use std::cell::RefCell;
use std::rc::Rc;

const MM_PER_PT: f64 = 25.4 / 72.0;

fn pt_to_mm(pt: f64) -> f64 {
    pt * MM_PER_PT
}

fn parse_font_points_mm(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches("pt").trim().replace(',', ".");
    let pt = s.parse::<f64>().ok()?;
    if pt <= 0.0 {
        return None;
    }
    Some(pt_to_mm(pt))
}

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

/// Setup text content entry handler
#[allow(clippy::type_complexity)]
pub fn setup_text_content_handler(
    text_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    text_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let text = entry.text().to_string();
        let mut designer_state = state.borrow_mut();

        modify_selected_shape_with_undo(&mut designer_state, |obj| {
            if let Shape::Text(text_shape) = &mut obj.shape {
                text_shape.text = text;
            }
        });

        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}

/// Setup font size entry handler (entered in points, stored in mm)
#[allow(clippy::type_complexity)]
pub fn setup_font_size_handler(
    font_size_entry: &Entry,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    font_size_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Some(size_mm) = parse_font_points_mm(&entry.text()) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();

            modify_selected_shape_with_undo(&mut designer_state, |obj| {
                if let Shape::Text(text_shape) = &mut obj.shape {
                    text_shape.font_size = size_mm;
                }
            });

            drop(designer_state);
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup font family dropdown handler
#[allow(clippy::type_complexity)]
pub fn setup_font_family_handler(
    font_family_combo: &DropDown,
    font_bold_check: &CheckButton,
    font_italic_check: &CheckButton,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    let bold_check = font_bold_check.clone();
    let italic_check = font_italic_check.clone();

    font_family_combo.connect_selected_notify(move |combo| {
        if *updating.borrow() {
            return;
        }
        let family = combo
            .selected_item()
            .and_downcast::<gtk4::StringObject>()
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "Sans".to_string());

        let bold = bold_check.is_active();
        let italic = italic_check.is_active();

        let mut designer_state = state.borrow_mut();
        let family_clone = family.clone();
        modify_selected_shape_with_undo(&mut designer_state, |obj| {
            if let Shape::Text(text_shape) = &mut obj.shape {
                text_shape.font_family = family_clone;
                text_shape.bold = bold;
                text_shape.italic = italic;
            }
        });
        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}

/// Setup font bold checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_font_bold_handler(
    font_bold_check: &CheckButton,
    font_family_combo: &DropDown,
    font_italic_check: &CheckButton,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    let font_combo = font_family_combo.clone();
    let italic_check = font_italic_check.clone();
    let bold_check = font_bold_check.clone();

    font_bold_check.connect_toggled(move |_| {
        if *updating.borrow() {
            return;
        }
        let family = font_combo
            .selected_item()
            .and_downcast::<gtk4::StringObject>()
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "Sans".to_string());
        let bold = bold_check.is_active();
        let italic = italic_check.is_active();

        let mut designer_state = state.borrow_mut();
        let family_clone = family.clone();
        modify_selected_shape_with_undo(&mut designer_state, |obj| {
            if let Shape::Text(text_shape) = &mut obj.shape {
                text_shape.font_family = family_clone;
                text_shape.bold = bold;
                text_shape.italic = italic;
            }
        });
        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}

/// Setup font italic checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_font_italic_handler(
    font_italic_check: &CheckButton,
    font_family_combo: &DropDown,
    font_bold_check: &CheckButton,
    state: Rc<RefCell<DesignerState>>,
    redraw_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    updating: Rc<RefCell<bool>>,
) {
    let font_combo = font_family_combo.clone();
    let bold_check = font_bold_check.clone();
    let italic_check = font_italic_check.clone();

    font_italic_check.connect_toggled(move |_| {
        if *updating.borrow() {
            return;
        }
        let family = font_combo
            .selected_item()
            .and_downcast::<gtk4::StringObject>()
            .map(|s| s.string().to_string())
            .unwrap_or_else(|| "Sans".to_string());
        let bold = bold_check.is_active();
        let italic = italic_check.is_active();

        let mut designer_state = state.borrow_mut();
        let family_clone = family.clone();
        modify_selected_shape_with_undo(&mut designer_state, |obj| {
            if let Shape::Text(text_shape) = &mut obj.shape {
                text_shape.font_family = family_clone;
                text_shape.bold = bold;
                text_shape.italic = italic;
            }
        });
        drop(designer_state);
        if let Some(ref cb) = *redraw_callback.borrow() {
            cb();
        }
    });
}
