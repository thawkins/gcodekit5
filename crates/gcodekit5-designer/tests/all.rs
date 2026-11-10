//! Single integration-test binary for `gcodekit5-designer`.
//!
//! Each test file is included as a child module, so the crate links one test
//! binary instead of one per file. Run one file with
//! `cargo nextest run -p gcodekit5-designer -E 'test({module}::)'`.

/// Binary-wide lock for tests that touch process-global state.
pub fn global_env_mutex() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

/// Takes [`global_env_mutex`], recovering it if a previous holder panicked.
pub fn global_env_lock() -> std::sync::MutexGuard<'static, ()> {
    global_env_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[path = "test_audit_regressions.rs"]
mod test_audit_regressions;
#[path = "test_cam_depth_and_gcode.rs"]
mod test_cam_depth_and_gcode;
#[path = "test_chamfer_dimensions.rs"]
mod test_chamfer_dimensions;
#[path = "test_circular_array_step.rs"]
mod test_circular_array_step;
#[path = "test_default_properties_inheritance.rs"]
mod test_default_properties_inheritance;
#[path = "test_dxf_polyline_splinefit.rs"]
mod test_dxf_polyline_splinefit;
#[path = "test_dxf_spline_import.rs"]
mod test_dxf_spline_import;
#[path = "test_dxf_z_metadata.rs"]
mod test_dxf_z_metadata;
#[path = "test_import_transforms_units.rs"]
mod test_import_transforms_units;
#[path = "test_pocket_holes.rs"]
mod test_pocket_holes;
#[path = "test_shape_model_mirror_bounds.rs"]
mod test_shape_model_mirror_bounds;
#[path = "test_undo_order_and_groups.rs"]
mod test_undo_order_and_groups;
