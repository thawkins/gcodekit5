//! Regression tests for settings persistence: tolerant loading of partial
//! config files and real factory-default restoration.

use gcodekit5_core::shared;
use gcodekit5_settings::{
    Config, SettingValue, SettingsController, SettingsDialog, SettingsPersistence,
};

#[test]
fn test_config_load_partial_json_uses_defaults() {
    // An older config missing whole sections and individual fields must still load.
    let json = r#"{ "connection": { "port": "/dev/ttyACM0" }, "machine": { "x_limit": 400.0 } }"#;
    let config: Config = serde_json::from_str(json).expect("partial config should load");
    assert_eq!(config.connection.port, "/dev/ttyACM0");
    assert_eq!(config.connection.baud_rate, 115200);
    assert_eq!(config.machine.x_limit, 400.0);
    assert_eq!(config.machine.y_limit, 200.0);
    config.validate().expect("defaults should validate");
}

#[test]
fn test_settings_restore_defaults_uses_factory_values() {
    let mut persistence = SettingsPersistence::new();
    persistence.config_mut().machine.x_limit = 555.0;
    let mut dialog = SettingsDialog::new();
    persistence.populate_dialog(&mut dialog);

    let controller = SettingsController::new(shared(dialog), shared(persistence));
    controller.restore_defaults();

    let dialog = controller.dialog.borrow();
    match &dialog.get_setting("x_limit").expect("x_limit").value {
        SettingValue::Float(v) => assert_eq!(*v, 200.0),
        other => panic!("unexpected value {:?}", other),
    }
}

#[test]
fn test_settings_invalid_numeric_input_is_ignored() {
    let persistence = SettingsPersistence::new();
    let mut dialog = SettingsDialog::new();
    persistence.populate_dialog(&mut dialog);
    let controller = SettingsController::new(shared(dialog), shared(persistence));

    controller.update_setting("window_width", "abc");
    let dialog = controller.dialog.borrow();
    match &dialog
        .get_setting("window_width")
        .expect("window_width")
        .value
    {
        SettingValue::Integer(v) => assert_eq!(*v, 1400),
        other => panic!("unexpected value {:?}", other),
    }
}

#[test]
fn test_settings_listener_can_read_dialog_during_update() {
    let persistence = SettingsPersistence::new();
    let mut dialog = SettingsDialog::new();
    persistence.populate_dialog(&mut dialog);
    let controller = std::rc::Rc::new(SettingsController::new(shared(dialog), shared(persistence)));

    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    {
        let dialog = controller.dialog.clone();
        let seen = seen.clone();
        controller.on_setting_changed(move |id, _value| {
            // Reading the dialog from inside a listener used to panic with
            // "already mutably borrowed".
            let dialog = dialog.borrow();
            let current = dialog
                .get_setting(id)
                .expect("setting exists")
                .value
                .as_str();
            seen.borrow_mut().push(current);
        });
    }
    {
        // A listener that registers another listener must not panic either.
        let ctrl = std::rc::Rc::downgrade(&controller);
        controller.on_setting_changed(move |_, _| {
            if let Some(ctrl) = ctrl.upgrade() {
                ctrl.on_setting_changed(|_, _| {});
            }
        });
    }

    controller.update_setting("window_width", "1600");
    controller.restore_defaults();
    controller.discard_changes();

    let seen = seen.borrow();
    assert_eq!(seen.first().map(String::as_str), Some("1600"));
    assert!(
        seen.len() >= 2,
        "restore/discard should also notify: {seen:?}"
    );
}

#[test]
fn test_config_merge_keeps_ui_settings_with_system_theme() {
    let mut base = Config::default();
    let mut other = Config::default();
    other.ui.theme = gcodekit5_settings::Theme::System;
    other.ui.window_width = 1920;
    other.ui.font_size = 16;
    other.ui.language = "es".to_string();

    base.merge(&other);

    assert_eq!(base.ui.window_width, 1920);
    assert_eq!(base.ui.font_size, 16);
    assert_eq!(base.ui.language, "es");
}

#[test]
fn test_config_merge_ignores_unpopulated_ui_section() {
    let mut base = Config::default();
    base.ui.window_width = 1024;
    let mut other = Config::default();
    other.ui.window_width = 0;
    other.ui.window_height = 0;

    base.merge(&other);

    assert_eq!(base.ui.window_width, 1024);
}

#[test]
fn test_unreadable_config_is_backed_up_before_it_is_replaced() {
    // Audit A-01: a config.json that failed to load was silently replaced by
    // defaults on the next automatic save.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{ not json").expect("write");

    let (mut persistence, error) = SettingsPersistence::load_or_default(&path);
    assert!(error.is_some(), "load error must be reported");

    persistence.persist_to(&path).expect("save");
    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("config.json.bak-"))
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
    let backup = std::fs::read_to_string(dir.path().join(&backups[0])).expect("backup");
    assert_eq!(backup, "{ not json");
    assert!(SettingsPersistence::load_from_file(&path).is_ok());

    // Later saves do not back up again.
    persistence.persist_to(&path).expect("second save");
    let count = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|e| e.file_name().to_string_lossy().contains(".bak-"))
        })
        .count();
    assert_eq!(count, 1);
}

#[test]
fn test_missing_config_loads_defaults_without_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_, error) = SettingsPersistence::load_or_default(&dir.path().join("config.json"));
    assert!(error.is_none());
}

#[test]
fn test_preferences_save_keeps_folder_remembered_by_file_dialogs() {
    // Audit E2-05: saving Preferences restored the folder the dialog was
    // filled with, undoing the folder a file dialog remembered since.
    let mut persistence = SettingsPersistence::new();
    persistence.config_mut().file_processing.output_directory = "/start".into();
    let mut dialog = SettingsDialog::new();
    persistence.populate_dialog(&mut dialog);
    persistence.config_mut().file_processing.output_directory = "/picked".into();
    persistence.load_from_dialog(&dialog).expect("load");
    assert_eq!(
        persistence.config().file_processing.output_directory,
        std::path::PathBuf::from("/picked")
    );
}
