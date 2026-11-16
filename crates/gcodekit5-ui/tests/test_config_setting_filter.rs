//! Tests for the Device Config settings filter shared by the list and row activation.

use gcodekit5_ui::ui::gtk::config_settings::ConfigSettingRow;

fn row(number: u16, name: &str, category: &str) -> ConfigSettingRow {
    ConfigSettingRow {
        number,
        name: name.to_string(),
        value: "0".to_string(),
        unit: String::new(),
        description: String::new(),
        category: category.to_string(),
        read_only: false,
    }
}

#[test]
fn test_config_setting_filter_matches_eeprom_id() {
    let eeprom = row(1100, "Max feedrate", "Repetier EEPROM");
    assert!(eeprom.matches_filter("epr:100", "All"));
    assert!(!eeprom.matches_filter("$1100", "All"));
}

#[test]
fn test_config_setting_filter_matches_grbl_id_and_category() {
    let grbl = row(110, "X max rate", "Max Rate");
    assert!(grbl.matches_filter("$110", "All"));
    assert!(grbl.matches_filter("x max", "Max Rate"));
    assert!(!grbl.matches_filter("", "Homing"));
    assert!(grbl.matches_filter("", "All"));
}
