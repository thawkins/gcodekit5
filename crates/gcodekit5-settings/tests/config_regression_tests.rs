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
