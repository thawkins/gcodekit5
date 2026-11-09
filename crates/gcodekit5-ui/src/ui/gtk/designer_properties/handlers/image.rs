//! Image engraving property handlers for the properties panel.
//!
//! Every edit is applied to all selected raster images as a single undoable change.
//! Text entries apply on commit (Enter / focus-out) via [`super::connect_entry_commit`].

use super::{change_selected_with_undo, connect_entry_commit};
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::{RasterImage, Shape};
use gtk4::prelude::*;
use gtk4::{CheckButton, ComboBoxText, Entry};
use std::rc::Rc;

/// Callback type used to redraw the canvas and refresh the panel.
type RedrawCallback = SharedOption<Rc<dyn Fn()>>;

/// Applies `modify` to every selected raster image as one undo step, then redraws.
fn apply_to_selected_images<F>(
    state: &Shared<DesignerState>,
    redraw_callback: &RedrawCallback,
    name: &str,
    modify: F,
) where
    F: Fn(&mut RasterImage) -> bool,
{
    let changed = {
        let mut designer_state = state.borrow_mut();
        change_selected_with_undo(&mut designer_state, name, |obj| match &mut obj.shape {
            Shape::RasterImage(img) => modify(img),
            _ => false,
        })
    };
    if changed {
        let cb = redraw_callback.borrow().clone();
        if let Some(cb) = cb {
            cb();
        }
    }
}

/// Connects a numeric image entry: parses an `f64`, clamps it with `clamp` and stores
/// it with `store` on commit.
#[allow(clippy::too_many_arguments)]
fn setup_numeric_entry(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
    name: &'static str,
    clamp: fn(f64) -> f64,
    store: fn(&mut RasterImage, f64) -> bool,
) {
    let apply_state = state.clone();
    connect_entry_commit(entry, state, updating, has_focus, move |entry| match entry
        .text()
        .trim()
        .replace(',', ".")
        .parse::<f64>()
    {
        Ok(value) => {
            entry.remove_css_class("entry-invalid");
            let value = clamp(value);
            apply_to_selected_images(&apply_state, &redraw_callback, name, |img| {
                store(img, value)
            });
        }
        Err(_) => entry.add_css_class("entry-invalid"),
    });
}

/// Replaces `*field` with `value`, returning whether it changed.
fn set_f64(field: &mut f64, value: f64) -> bool {
    let changed = (*field - value).abs() > f64::EPSILON;
    *field = value;
    changed
}

/// Setup handler for image feed rate entry.
pub fn setup_feed_rate_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_numeric_entry(
        entry,
        state,
        redraw_callback,
        updating,
        has_focus,
        "Change Image Feed Rate",
        |v| v.max(0.0),
        |img, v| set_f64(&mut img.feed_rate, v),
    );
}

/// Setup handler for image travel rate entry.
pub fn setup_travel_rate_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_numeric_entry(
        entry,
        state,
        redraw_callback,
        updating,
        has_focus,
        "Change Image Travel Rate",
        |v| v.max(0.0),
        |img, v| set_f64(&mut img.travel_rate, v),
    );
}

/// Setup handler for image min power entry (percent).
pub fn setup_min_power_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_numeric_entry(
        entry,
        state,
        redraw_callback,
        updating,
        has_focus,
        "Change Image Min Power",
        |v| v.clamp(0.0, 100.0),
        |img, v| set_f64(&mut img.min_power, v),
    );
}

/// Setup handler for image max power entry (percent).
pub fn setup_max_power_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_numeric_entry(
        entry,
        state,
        redraw_callback,
        updating,
        has_focus,
        "Change Image Max Power",
        |v| v.clamp(0.0, 100.0),
        |img, v| set_f64(&mut img.max_power, v),
    );
}

/// Setup handler for image PPI entry.
pub fn setup_ppi_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_numeric_entry(
        entry,
        state,
        redraw_callback,
        updating,
        has_focus,
        "Change Image PPI",
        |v| v.clamp(1.0, 1000.0),
        |img, v| set_f64(&mut img.ppi, v),
    );
}

/// Setup handler for image scan direction combo box.
pub fn setup_scan_direction_handler(
    combo: &ComboBoxText,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
) {
    combo.connect_changed(move |combo| {
        if *updating.borrow() {
            return;
        }
        if let Some(direction) = combo.active_id() {
            let direction = direction.to_string();
            apply_to_selected_images(&state, &redraw_callback, "Change Scan Direction", |img| {
                let changed = img.scan_direction != direction;
                img.scan_direction = direction.clone();
                changed
            });
        }
    });
}

/// Setup handler for image bidirectional check button.
pub fn setup_bidirectional_handler(
    check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
) {
    check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }
        let bidirectional = check.is_active();
        apply_to_selected_images(&state, &redraw_callback, "Change Bidirectional", |img| {
            let changed = img.bidirectional != bidirectional;
            img.bidirectional = bidirectional;
            changed
        });
    });
}

/// Setup handler for image invert check button.
pub fn setup_invert_handler(
    check: &CheckButton,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
) {
    check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }
        let invert = check.is_active();
        apply_to_selected_images(&state, &redraw_callback, "Change Invert", |img| {
            let changed = img.invert != invert;
            img.invert = invert;
            changed
        });
    });
}

/// Setup handler for image dithering combo box.
pub fn setup_dithering_handler(
    combo: &ComboBoxText,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
) {
    combo.connect_changed(move |combo| {
        if *updating.borrow() {
            return;
        }
        if let Some(dithering) = combo.active_id() {
            let dithering = dithering.to_string();
            apply_to_selected_images(&state, &redraw_callback, "Change Dithering", |img| {
                let changed = img.dithering != dithering;
                img.dithering = dithering.clone();
                changed
            });
        }
    });
}

/// Setup handler for the halftone threshold entry (0-255).
pub fn setup_halftone_threshold_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: RedrawCallback,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(entry, state, updating, has_focus, move |entry| match entry
        .text()
        .trim()
        .parse::<u8>()
    {
        Ok(threshold) => {
            entry.remove_css_class("entry-invalid");
            apply_to_selected_images(
                &apply_state,
                &redraw_callback,
                "Change Halftone Threshold",
                |img| {
                    let changed = img.halftone_threshold != threshold;
                    img.halftone_threshold = threshold;
                    changed
                },
            );
        }
        Err(_) => entry.add_css_class("entry-invalid"),
    });
}
