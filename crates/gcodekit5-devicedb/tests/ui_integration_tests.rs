//! Regression tests for the device profile UI integration layer.

use std::sync::Arc;

use gcodekit5_devicedb::{
    ControllerType, DeviceManager, DeviceProfile, DeviceProfileUiModel, DeviceUiController,
};

#[test]
fn test_devicedb_grblhal_profile_round_trips_through_ui_model() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manager = Arc::new(DeviceManager::new(dir.path().join("devices.json")));
    let profile = DeviceProfile {
        controller_type: ControllerType::GrblHal,
        ..DeviceProfile::default()
    };
    let id = profile.id.clone();
    manager.save_profile(profile.clone()).expect("save");

    let controller = DeviceUiController::new(Arc::clone(&manager));
    let ui_model: DeviceProfileUiModel = profile.into();
    controller
        .update_profile_from_ui(ui_model)
        .expect("grblHAL profile should be savable from the UI model");
    assert_eq!(
        manager.get_profile(&id).map(|p| p.controller_type),
        Some(ControllerType::GrblHal)
    );

    // Saved file must reload cleanly (atomic write leaves no temp file behind)
    let reloaded = DeviceManager::new(dir.path().join("devices.json"));
    reloaded.load().expect("reload");
    assert!(reloaded.get_profile(&id).is_some());
    assert!(!dir.path().join("devices.json.tmp").exists());
}
