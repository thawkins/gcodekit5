//! CAM property handlers (operation type, depth, step down, step in, ramp angle, strategy, raster fill).

use super::connect_entry_commit;
use gcodekit5_core::units;
use gcodekit5_core::{Shared, SharedOption};
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{CheckButton, DropDown, Entry};
use std::rc::Rc;

/// Setup CAM use-global checkbox handler.
///
/// Checked => use global CAM values (disable object-specific edition).
/// Unchecked => use object-specific CAM values.
pub fn setup_cam_use_global_handler(
    use_global_check: &CheckButton,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
) {
    use_global_check.connect_toggled(move |check| {
        if *updating.borrow() {
            return;
        }
        let mut designer_state = state.borrow_mut();
        let use_custom = !check.is_active();
        designer_state.set_selected_use_custom_values(use_custom);
    });
}

/// Setup operation type dropdown handler
pub fn setup_operation_type_handler(
    op_type_combo: &DropDown,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
) {
    op_type_combo.connect_selected_notify(move |combo| {
        if *updating.borrow() {
            return;
        }
        let mut designer_state = state.borrow_mut();
        let is_pocket = combo.selected() == 1;
        let depth = designer_state
            .canvas
            .shapes()
            .find(|s| s.selected)
            .map(|s| s.pocket_depth)
            .unwrap_or(0.0);
        designer_state.set_selected_pocket_properties(is_pocket, depth);
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

/// Setup a length-valued CAM entry that applies on Enter / focus-out.
#[allow(clippy::type_complexity)]
fn setup_length_entry(
    entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
    setter: fn(&mut DesignerState, f64),
) {
    let apply_state = state.clone();
    connect_entry_commit(entry, state, updating, has_focus, move |entry| {
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            setter(&mut apply_state.borrow_mut(), val as f64);
            redraw(&redraw_callback);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup step down entry handler (applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_step_down_handler(
    step_down_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_length_entry(
        step_down_entry,
        state,
        settings,
        redraw_callback,
        updating,
        has_focus,
        DesignerState::set_selected_step_down,
    );
}

/// Setup step in entry handler (applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_step_in_handler(
    step_in_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    setup_length_entry(
        step_in_entry,
        state,
        settings,
        redraw_callback,
        updating,
        has_focus,
        DesignerState::set_selected_step_in,
    );
}

/// Setup raster fill entry handler (percentage 0-100, applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_raster_fill_handler(
    raster_fill_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(
        raster_fill_entry,
        state,
        updating,
        has_focus,
        move |entry| {
            if let Ok(val) = entry.text().trim().parse::<f64>() {
                let clamped = val.clamp(0.0, 100.0);
                entry.remove_css_class("entry-invalid");
                apply_state
                    .borrow_mut()
                    .set_selected_raster_fill_ratio(clamped / 100.0);
                redraw(&redraw_callback);
            } else {
                entry.add_css_class("entry-invalid");
            }
        },
    );
}

/// Setup ramp angle entry handler (degrees, applied on Enter / focus-out).
#[allow(clippy::type_complexity)]
pub fn setup_ramp_angle_handler(
    ramp_angle_entry: &Entry,
    state: Shared<DesignerState>,
    redraw_callback: SharedOption<Rc<dyn Fn()>>,
    updating: Shared<bool>,
    has_focus: Shared<bool>,
) {
    let apply_state = state.clone();
    connect_entry_commit(ramp_angle_entry, state, updating, has_focus, move |entry| {
        if let Ok(val) = entry.text().trim().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            apply_state.borrow_mut().set_selected_ramp_angle(val);
            redraw(&redraw_callback);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup strategy dropdown handler
pub fn setup_strategy_handler(
    strategy_combo: &DropDown,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
) {
    strategy_combo.connect_selected_notify(move |combo| {
        if *updating.borrow() {
            return;
        }
        let mut designer_state = state.borrow_mut();
        let strategy = match combo.selected() {
            0 => PocketStrategy::Raster {
                angle: 0.0,
                bidirectional: true,
            },
            1 => PocketStrategy::ContourParallel,
            2 => PocketStrategy::Adaptive,
            _ => PocketStrategy::ContourParallel,
        };
        designer_state.set_selected_pocket_strategy(strategy);
    });
}
