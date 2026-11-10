//! Effects property handlers (offset, chamfer with live preview).

use crate::ui::gtk::designer_properties::SharedMeasurementSystem;
use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption, SharedVec};
use gcodekit5_designer::canvas::DrawingObject;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::model::Shape;
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

/// The parts of a live-preview effect (offset, chamfer) that differ.
struct Effect {
    /// The previewed shape for a distance.
    preview: fn(&Shape, f64) -> Shape,
    /// The shape's current value of the effect.
    current: fn(&DrawingObject) -> f64,
    /// Applies the distance to the selected shapes (one undo step).
    apply: fn(&mut DesignerState, f64),
}

/// Setup offset entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_offset_handler(
    offset_entry: &Entry,
    state: Shared<DesignerState>,
    preview_shapes: SharedVec<Shape>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    measurement_system: SharedMeasurementSystem,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_effect_handler(
        offset_entry,
        state,
        preview_shapes,
        redraw_callback,
        measurement_system,
        updating,
        has_focus,
        Effect {
            preview: gcodekit5_designer::ops::perform_offset,
            current: |o| o.offset,
            apply: DesignerState::set_offset_selected,
        },
    );
}

/// Setup chamfer entry handler with preview
#[allow(clippy::type_complexity)]
pub fn setup_chamfer_handler(
    chamfer_entry: &Entry,
    state: Shared<DesignerState>,
    preview_shapes: SharedVec<Shape>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    measurement_system: SharedMeasurementSystem,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_effect_handler(
        chamfer_entry,
        state,
        preview_shapes,
        redraw_callback,
        measurement_system,
        updating,
        has_focus,
        Effect {
            preview: gcodekit5_designer::ops::perform_chamfer,
            current: |o| o.chamfer,
            apply: DesignerState::set_chamfer_selected,
        },
    );
}

/// Wires an effect entry: live preview while typing, and apply on Enter or
/// when the entry loses focus.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn setup_effect_handler(
    entry: &Entry,
    state: Shared<DesignerState>,
    preview_shapes: SharedVec<Shape>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    measurement_system: SharedMeasurementSystem,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
    effect: Effect,
) {
    let redraw = {
        let redraw_callback = redraw_callback.clone();
        move || {
            if let Some(ref cb) = *redraw_callback.borrow() {
                cb();
            }
        }
    };

    // Live preview on change
    {
        let state = state.clone();
        let preview = preview_shapes.clone();
        let entry_c = entry.clone();
        let measurement_system = measurement_system.clone();
        let redraw = redraw.clone();
        entry.connect_changed(move |_| {
            if *updating.borrow() {
                return;
            }
            let system = measurement_system.get();
            if let Ok(dist) = units::parse_length(&entry_c.text(), system).map(f64::from) {
                entry_c.remove_css_class("entry-invalid");
                let designer_state = state.borrow();
                *preview.borrow_mut() = designer_state
                    .canvas
                    .shapes()
                    .filter(|s| s.selected)
                    .map(|s| (effect.preview)(&s.shape, dist))
                    .collect();
            } else {
                entry_c.add_css_class("entry-invalid");
                preview.borrow_mut().clear();
            }
            redraw();
        });
    }

    // Apply the entered value (Enter or focus out).
    let apply: Rc<dyn Fn()> = {
        let entry_c = entry.clone();
        Rc::new(move || {
            let system = measurement_system.get();
            if let Ok(dist) = units::parse_length(&entry_c.text(), system).map(f64::from) {
                preview_shapes.borrow_mut().clear();
                let mut designer_state = state.borrow_mut();
                // Only record a change when the value actually differs; otherwise merely
                // tabbing through the field would push no-op undo entries (and for chamfer,
                // reset a path's fillet).
                let changed = designer_state
                    .canvas
                    .shapes()
                    .filter(|s| s.selected)
                    .any(|s| ((effect.current)(s) - dist).abs() > edit_tolerance_mm(system));
                if changed {
                    (effect.apply)(&mut designer_state, dist);
                }
                drop(designer_state);
                redraw();
            }
        })
    };

    {
        let apply = apply.clone();
        entry.connect_activate(move |_| apply());
    }

    let focus_controller = EventControllerFocus::new();
    {
        let has_focus = has_focus.clone();
        focus_controller.connect_enter(move |_| {
            *has_focus.borrow_mut() = true;
        });
    }
    focus_controller.connect_leave(move |_| {
        *has_focus.borrow_mut() = false;
        apply();
    });
    entry.add_controller(focus_controller);
}
