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

/// Files in `dir` whose name starts with `devices.json.bak-`.
fn backups(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("devices.json.bak-"))
                .unwrap_or(false)
        })
        .collect()
}

#[test]
fn test_devicedb_load_skips_bad_entry_and_backs_up_before_save() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("devices.json");
    let good = DeviceProfile {
        id: "good".to_string(),
        name: "Good".to_string(),
        ..DeviceProfile::default()
    };
    let original = serde_json::json!({
        "profiles": [
            good,
            { "id": "bad", "num_axes": "not a number" }
        ],
        "active_id": "good"
    })
    .to_string();
    std::fs::write(&path, &original).expect("write");

    let manager = DeviceManager::new(path.clone());
    manager
        .load()
        .expect("load must succeed despite one bad entry");
    assert!(manager.get_profile("good").is_some());
    assert!(manager.get_profile("bad").is_none());
    assert_eq!(
        manager.get_active_profile().map(|p| p.id),
        Some("good".to_string())
    );
    assert!(manager.has_unreadable_entries());
    assert!(backups(dir.path()).is_empty(), "load itself must not write");

    manager.save().expect("save");
    let baks = backups(dir.path());
    assert_eq!(baks.len(), 1, "one backup before the lossy save");
    assert_eq!(std::fs::read_to_string(&baks[0]).expect("bak"), original);
    assert!(!manager.has_unreadable_entries());

    // Later saves do not keep making backups.
    manager.save().expect("save again");
    assert_eq!(backups(dir.path()).len(), 1);
}

#[test]
fn test_devicedb_corrupt_file_is_backed_up_before_overwrite() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("devices.json");
    std::fs::write(&path, "{ not json").expect("write");

    let manager = DeviceManager::new(path.clone());
    assert!(manager.load().is_err());
    // The UI ignores the load error and may then save a new profile.
    manager
        .save_profile(DeviceProfile::default())
        .expect("save after failed load");
    let baks = backups(dir.path());
    assert_eq!(baks.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&baks[0]).expect("bak"),
        "{ not json"
    );
}

#[test]
fn test_devicedb_tcp_host_round_trips_through_ui_model() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manager = Arc::new(DeviceManager::new(dir.path().join("devices.json")));
    let profile = DeviceProfile::default();
    let id = profile.id.clone();
    manager.save_profile(profile.clone()).expect("save");

    let controller = DeviceUiController::new(Arc::clone(&manager));
    let mut ui_model: DeviceProfileUiModel = profile.into();
    assert_eq!(ui_model.tcp_host, "192.168.1.100");
    ui_model.connection_type = "TCP/IP".to_string();
    ui_model.tcp_host = "  cnc.local ".to_string();
    ui_model.tcp_port = "8080".to_string();
    controller
        .update_profile_from_ui(ui_model.clone())
        .expect("update");

    let reloaded = DeviceManager::new(dir.path().join("devices.json"));
    reloaded.load().expect("reload");
    let p = reloaded.get_profile(&id).expect("profile");
    assert_eq!(p.tcp_host, "cnc.local");
    assert_eq!(p.tcp_port, 8080);

    ui_model.tcp_host = String::new();
    assert!(
        controller.update_profile_from_ui(ui_model).is_err(),
        "an empty host is rejected for TCP profiles"
    );
}

#[test]
fn test_devicedb_set_active_rolls_back_when_save_fails() {
    // Review of #87: a failed save left the new profile active in memory.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("devices.json");
    let manager = DeviceManager::new(path.clone());
    let first = DeviceProfile {
        id: "first".to_string(),
        ..DeviceProfile::default()
    };
    let second = DeviceProfile {
        id: "second".to_string(),
        ..DeviceProfile::default()
    };
    manager.save_profile(first).expect("save first");
    manager.save_profile(second).expect("save second");
    manager.set_active_profile("first").expect("activate first");

    // Make the next save fail: the profiles file becomes a directory.
    std::fs::remove_file(&path).expect("remove file");
    std::fs::create_dir(&path).expect("block path");

    assert!(manager.set_active_profile("second").is_err());
    assert_eq!(
        manager.get_active_profile().map(|p| p.id),
        Some("first".to_string())
    );
}
