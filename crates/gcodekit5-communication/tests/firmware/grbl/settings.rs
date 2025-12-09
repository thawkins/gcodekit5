use gcodekit5_communication::firmware::grbl::settings::*;

#[test]
fn test_settings_manager_creation() {
    let manager = SettingsManager::new();
    assert_eq!(manager.count(), 0);
}

#[test]
fn test_add_setting() {
    let mut manager = SettingsManager::new();
    let setting = Setting {
        number: 110,
        name: "Baud Rate".to_string(),
        value: "115200".to_string(),
        numeric_value: Some(115200.0),
        description: "Serial communication speed".to_string(),
        range: Some((9600.0, 115200.0)),
        read_only: false,
    };

    manager.set_setting(setting);
    assert_eq!(manager.count(), 1);

    let retrieved = manager.get_setting(110);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Baud Rate");
}

#[test]
fn test_parse_setting_line_valid() {
    let result = SettingsManager::parse_setting_line("$110=115200.000");
    assert!(result.is_some());

    let (number, value) = result.unwrap();
    assert_eq!(number, 110);
    assert_eq!(value, "115200.000");
}

#[test]
fn test_parse_setting_line_no_dollar() {
    let result = SettingsManager::parse_setting_line("110=115200.000");
    assert!(result.is_none());
}

#[test]
fn test_parse_setting_line_invalid_format() {
    let result = SettingsManager::parse_setting_line("$110:115200.000");
    assert!(result.is_none());
}

#[test]
fn test_backup_and_restore() {
    let mut manager = SettingsManager::new();
    let setting = Setting {
        number: 110,
        name: "Baud Rate".to_string(),
        value: "115200".to_string(),
        numeric_value: Some(115200.0),
        description: "Serial communication speed".to_string(),
        range: Some((9600.0, 115200.0)),
        read_only: false,
    };

    manager.set_setting(setting);
    manager.reset_dirty();
    manager.backup();

    // Modify
    manager.clear();
    assert_eq!(manager.count(), 0);

    // Restore
    let result = manager.restore();
    assert!(result.is_ok());
    assert_eq!(manager.count(), 1);
}

#[test]
fn test_validate_setting_read_only() {
    let mut manager = SettingsManager::new();
    let setting = Setting {
        number: 110,
        name: "Baud Rate".to_string(),
        value: "115200".to_string(),
        numeric_value: Some(115200.0),
        description: "Serial communication speed".to_string(),
        range: Some((9600.0, 115200.0)),
        read_only: true,
    };

    manager.set_setting(setting);
    let result = manager.validate_setting(110, "57600");
    assert!(result.is_err());
}

#[test]
fn test_validate_setting_range() {
    let mut manager = SettingsManager::new();
    let setting = Setting {
        number: 110,
        name: "Baud Rate".to_string(),
        value: "115200".to_string(),
        numeric_value: Some(115200.0),
        description: "Serial communication speed".to_string(),
        range: Some((9600.0, 115200.0)),
        read_only: false,
    };

    manager.set_setting(setting);

    // Valid value
    assert!(manager.validate_setting(110, "57600").is_ok());

    // Out of range
    assert!(manager.validate_setting(110, "500000").is_err());
}

#[test]
fn test_get_sorted_settings() {
    let mut manager = SettingsManager::new();
    for i in [120u8, 110, 130].iter() {
        let setting = Setting {
            number: *i,
            name: format!("Setting {}", i),
            value: "test".to_string(),
            numeric_value: None,
            description: "Test setting".to_string(),
            range: None,
            read_only: false,
        };
        manager.set_setting(setting);
    }

    let sorted = manager.get_sorted_settings();
    assert_eq!(sorted.len(), 3);
    assert_eq!(sorted[0].number, 110);
    assert_eq!(sorted[1].number, 120);
    assert_eq!(sorted[2].number, 130);
}

#[test]
fn test_find_by_name() {
    let mut manager = SettingsManager::new();
    let setting1 = Setting {
        number: 110,
        name: "Step Pulse Duration".to_string(),
        value: "10".to_string(),
        numeric_value: Some(10.0),
        description: "Pulse duration".to_string(),
        range: None,
        read_only: false,
    };

    let setting2 = Setting {
        number: 111,
        name: "Idle Delay".to_string(),
        value: "25".to_string(),
        numeric_value: Some(25.0),
        description: "Idle delay".to_string(),
        range: None,
        read_only: false,
    };

    manager.set_setting(setting1);
    manager.set_setting(setting2);

    let results = manager.find_by_name("Pulse");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].number, 110);
}
