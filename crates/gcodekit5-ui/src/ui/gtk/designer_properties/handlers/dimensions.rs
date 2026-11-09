//! Dimension property handlers (position, size, aspect ratio).
//!
//! All entries apply on commit (Enter or focus-out) via
//! [`super::connect_entry_commit`], so each edit is exactly one undo step.

use super::{change_selected_with_undo, connect_entry_commit};
use crate::ui::gtk::designer_properties::SharedMeasurementSystem;
use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gtk4::prelude::*;
use gtk4::{CheckButton, Entry};
use std::rc::Rc;

/// Widgets and shared flags needed by the dimension handlers.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct DimensionContext {
    /// Centre X entry.
    pub pos_x: Entry,
    /// Centre Y entry.
    pub pos_y: Entry,
    /// Width entry.
    pub width: Entry,
    /// Height entry.
    pub height: Entry,
    /// Lock-aspect-ratio checkbox.
    pub lock_aspect: CheckButton,
    /// Width / height ratio captured when the lock was enabled.
    pub aspect_ratio: Shared<f64>,
    /// Designer state.
    pub state: Shared<DesignerState>,
    /// Measurement system used to parse the entries.
    pub measurement_system: SharedMeasurementSystem,
    /// Canvas redraw + panel refresh callback.
    pub redraw_callback: SharedOption<Rc<dyn Fn()>>,
    /// Programmatic-update guard.
    pub updating: Shared<bool>,
    /// "User is editing" flag.
    pub has_focus: Shared<bool>,
}

impl DimensionContext {
    fn system(&self) -> units::MeasurementSystem {
        self.measurement_system.get()
    }

    fn parse(&self, entry: &Entry) -> Option<f64> {
        units::parse_length(&entry.text(), self.system())
            .ok()
            .map(|v| v as f64)
    }

    fn redraw(&self) {
        let cb = self.redraw_callback.borrow().clone();
        if let Some(cb) = cb {
            cb();
        }
    }

    /// Applies the X/Y/W/H entries to the selection (one undo step).
    fn apply_position_and_size(&self, update_position: bool, update_size: bool) {
        let x = self.parse(&self.pos_x).unwrap_or(0.0);
        let y = self.parse(&self.pos_y).unwrap_or(0.0);
        let w = self.parse(&self.width).unwrap_or(0.0);
        let h = self.parse(&self.height).unwrap_or(0.0);
        {
            let mut designer_state = self.state.borrow_mut();
            if designer_state.selected_count() >= 1 {
                designer_state.set_selected_position_and_size_with_flags(
                    x,
                    y,
                    w,
                    h,
                    update_position,
                    update_size,
                );
            }
        }
        self.redraw();
    }
}

/// Setup the centre X and centre Y entries (applied on Enter / focus-out).
pub fn setup_position_handlers(ctx: &DimensionContext) {
    for entry in [&ctx.pos_x, &ctx.pos_y] {
        let c = ctx.clone();
        connect_entry_commit(
            entry,
            ctx.state.clone(),
            ctx.updating.clone(),
            ctx.has_focus.clone(),
            move |entry| {
                if c.parse(entry).is_some() {
                    entry.remove_css_class("entry-invalid");
                    c.apply_position_and_size(true, false);
                } else {
                    entry.add_css_class("entry-invalid");
                }
            },
        );
    }
}

/// Setup the Z (start depth) entry (applied on Enter / focus-out).
pub fn setup_position_z_handler(pos_z_entry: &Entry, ctx: &DimensionContext) {
    let c = ctx.clone();
    connect_entry_commit(
        pos_z_entry,
        ctx.state.clone(),
        ctx.updating.clone(),
        ctx.has_focus.clone(),
        move |entry| {
            if let Some(val) = c.parse(entry) {
                entry.remove_css_class("entry-invalid");
                {
                    let mut designer_state = c.state.borrow_mut();
                    if designer_state.selected_count() >= 1 {
                        designer_state.set_selected_start_depth(val);
                    }
                }
                c.redraw();
            } else {
                entry.add_css_class("entry-invalid");
            }
        },
    );
}

/// Setup the width and height entries (applied on Enter / focus-out).
///
/// With the aspect ratio locked, editing one dimension updates the other entry
/// before the combined size is applied as a single undo step.
pub fn setup_size_handlers(ctx: &DimensionContext) {
    for is_width in [true, false] {
        let c = ctx.clone();
        let entry = if is_width { &ctx.width } else { &ctx.height };
        connect_entry_commit(
            entry,
            ctx.state.clone(),
            ctx.updating.clone(),
            ctx.has_focus.clone(),
            move |entry| {
                let Some(val) = c.parse(entry) else {
                    entry.add_css_class("entry-invalid");
                    return;
                };
                entry.remove_css_class("entry-invalid");

                if c.lock_aspect.is_active() {
                    let ratio = *c.aspect_ratio.borrow();
                    if ratio > 0.0 {
                        let (other, other_val) = if is_width {
                            (&c.height, val / ratio)
                        } else {
                            (&c.width, val * ratio)
                        };
                        *c.updating.borrow_mut() = true;
                        other.set_text(&units::format_length(other_val as f32, c.system()));
                        *c.updating.borrow_mut() = false;
                    }
                }

                c.apply_position_and_size(false, true);
            },
        );
    }
}

/// Setup lock aspect ratio toggle handler.
///
/// Captures the current ratio when locking and stores the flag on the selected shapes
/// as an undoable change.
pub fn setup_lock_aspect_handler(ctx: &DimensionContext) {
    let c = ctx.clone();
    ctx.lock_aspect.connect_toggled(move |check| {
        let is_active = check.is_active();

        if is_active {
            if let (Some(w), Some(h)) = (c.parse(&c.width), c.parse(&c.height)) {
                if h > 0.0 {
                    *c.aspect_ratio.borrow_mut() = w / h;
                }
            }
        }

        // Programmatic updates (selection refresh) must not write back into the
        // shapes: that would mutate the model just by selecting a shape and would
        // re-borrow the designer state while the panel is being refreshed.
        if *c.updating.borrow() {
            return;
        }

        let mut designer_state = c.state.borrow_mut();
        change_selected_with_undo(&mut designer_state, "Change Lock Aspect Ratio", |obj| {
            let changed = obj.lock_aspect_ratio != is_active;
            obj.lock_aspect_ratio = is_active;
            changed
        });
    });
}
