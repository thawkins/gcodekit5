//! Text property handlers (content, font family, size, bold, italic).

use super::{change_selected_with_undo, connect_entry_commit};
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{DesignText, Shape};
use gtk4::prelude::*;
use gtk4::{CheckButton, DropDown, Entry};
use std::rc::Rc;

use crate::ui::gtk::designer_canvas::pt_to_mm;

fn parse_font_points_mm(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches("pt").trim().replace(',', ".");
    let pt = s.parse::<f64>().ok()?;
    if pt <= 0.0 {
        return None;
    }
    Some(pt_to_mm(pt))
}

/// Applies `modify` to every selected text shape as one undo step.
fn modify_selected_text_with_undo<F>(designer_state: &mut DesignerState, name: &str, modify: F)
where
    F: Fn(&mut DesignText) -> bool,
{
    change_selected_with_undo(designer_state, name, |obj| match &mut obj.shape {
        Shape::Text(text_shape) => modify(text_shape),
        _ => false,
    });
}

/// Invokes the redraw callback (if any) without holding its borrow during the call.
#[allow(clippy::type_complexity)]
fn redraw(redraw_callback: &SharedOption<Rc<dyn Fn()>>) {
    let cb = redraw_callback.borrow().clone();
    if let Some(cb) = cb {
        cb();
    }
}

/// Setup text content entry handler (applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_text_content_handler(
    text_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(text_entry, state, updating, has_focus, move |entry| {
        let text = entry.text().to_string();
        modify_selected_text_with_undo(&mut apply_state.borrow_mut(), "Change Text", |t| {
            let changed = t.text != text;
            t.text = text.clone();
            changed
        });
        redraw(&redraw_callback);
    });
}

/// Setup font size entry handler (entered in points, stored in mm; applied on
/// Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_font_size_handler(
    font_size_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(font_size_entry, state, updating, has_focus, move |entry| {
        if let Some(size_mm) = parse_font_points_mm(&entry.text()) {
            entry.remove_css_class("entry-invalid");
            modify_selected_text_with_undo(
                &mut apply_state.borrow_mut(),
                "Change Font Size",
                |t| {
                    let changed = (t.font_size - size_mm).abs() > f64::EPSILON;
                    t.font_size = size_mm;
                    changed
                },
            );
            redraw(&redraw_callback);
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
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
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

        modify_selected_text_with_undo(&mut state.borrow_mut(), "Change Font", |t| {
            let changed = t.font_family != family || t.bold != bold || t.italic != italic;
            t.font_family = family.clone();
            t.bold = bold;
            t.italic = italic;
            changed
        });
        redraw(&redraw_callback);
    });
}

/// Setup font bold checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_font_bold_handler(
    font_bold_check: &CheckButton,
    font_family_combo: &DropDown,
    font_italic_check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
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

        modify_selected_text_with_undo(&mut state.borrow_mut(), "Change Font", |t| {
            let changed = t.font_family != family || t.bold != bold || t.italic != italic;
            t.font_family = family.clone();
            t.bold = bold;
            t.italic = italic;
            changed
        });
        redraw(&redraw_callback);
    });
}

/// Setup font italic checkbox handler
#[allow(clippy::type_complexity)]
pub fn setup_font_italic_handler(
    font_italic_check: &CheckButton,
    font_family_combo: &DropDown,
    font_bold_check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
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

        modify_selected_text_with_undo(&mut state.borrow_mut(), "Change Font", |t| {
            let changed = t.font_family != family || t.bold != bold || t.italic != italic;
            t.font_family = family.clone();
            t.bold = bold;
            t.italic = italic;
            changed
        });
        redraw(&redraw_callback);
    });
}
