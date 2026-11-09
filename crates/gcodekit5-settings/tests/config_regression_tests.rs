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
