//! CAM property handlers (operation type, depth, step down, step in, ramp angle, strategy, raster fill).

use gcodekit5_core::units;
use gcodekit5_core::Shared;
use gcodekit5_designer::designer_state::DesignerState;
use gcodekit5_designer::pocket_operations::PocketStrategy;
use gcodekit5_settings::SettingsPersistence;
use gtk4::prelude::*;
use gtk4::{CheckButton, DropDown, Entry};

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

/// Setup pocket depth entry handler
pub fn setup_depth_handler(
    depth_entry: &Entry,
    op_type_combo: &DropDown,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
) {
    let op_combo = op_type_combo.clone();

    depth_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            let is_pocket = op_combo.selected() == 1;
            designer_state.set_selected_pocket_properties(is_pocket, val as f64);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup Z offset entry handler
pub fn setup_z_offset_handler(
    z_offset_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
) {
    z_offset_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_z_offset(val as f64);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup step down entry handler
pub fn setup_step_down_handler(
    step_down_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
) {
    step_down_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_step_down(val as f64);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup step in entry handler
pub fn setup_step_in_handler(
    step_in_entry: &Entry,
    state: Shared<DesignerState>,
    settings: Shared<SettingsPersistence>,
    updating: Shared<bool>,
) {
    step_in_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        let system = settings.borrow().config().ui.measurement_system;
        if let Ok(val) = units::parse_length(&entry.text(), system) {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_step_in(val as f64);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup raster fill entry handler (percentage 0-100)
pub fn setup_raster_fill_handler(
    raster_fill_entry: &Entry,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
) {
    raster_fill_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<f64>() {
            let clamped = val.clamp(0.0, 100.0);
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_raster_fill_ratio(clamped / 100.0);
        } else {
            entry.add_css_class("entry-invalid");
        }
    });
}

/// Setup ramp angle entry handler
pub fn setup_ramp_angle_handler(
    ramp_angle_entry: &Entry,
    state: Shared<DesignerState>,
    updating: Shared<bool>,
) {
    ramp_angle_entry.connect_changed(move |entry| {
        if *updating.borrow() {
            return;
        }
        if let Ok(val) = entry.text().parse::<f64>() {
            entry.remove_css_class("entry-invalid");
            let mut designer_state = state.borrow_mut();
            designer_state.set_selected_ramp_angle(val);
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
