//! # Repetier Settings Panel
//!
//! UI panel definitions and default values for Repetier firmware EEPROM settings.

use gcodekit5_communication::firmware::grbl::settings::Setting;
use super::ConfigSettingsView;

impl ConfigSettingsView {
    pub(crate) fn load_default_repetier_settings(&self) {
        let mut manager = self.settings_manager.borrow_mut();
        manager.clear();

        // Load standard Repetier EEPROM definitions (offset by +1000)
        for (number, name, description, read_only, unit) in Self::repetier_settings_definitions() {
            manager.set_setting(Setting {
                number,
                name: name.to_string(),
                value: "0".to_string(),
                numeric_value: Some(0.0),
                description: description.to_string(),
                range: None,
                read_only,
                unit: if unit.is_empty() {
                    None
                } else {
                    Some(unit.to_string())
                },
            });
        }
    }

    pub(crate) fn repetier_settings_definitions(
    ) -> Vec<(u16, &'static str, &'static str, bool, &'static str)> {
        vec![
            // Standard Repetier EEPROM Positions (1000 + EEPROM_POS)
            (1075, "Baudrate", "Serial communication baudrate", false, "bps"),
            (1129, "Filament Printed", "Total filament printed", true, "m"),
            (1125, "Printer Active", "Total printer active time", true, "s"),
            (1079, "Max. Inactive Time", "Max inactive time before shutdown", false, "ms"),
            (1083, "Stop Stepper After", "Stop stepper after inactivity", false, "ms"),
            (1003, "X Steps/mm", "X-axis steps per mm", false, "steps/mm"),
            (1007, "Y Steps/mm", "Y-axis steps per mm", false, "steps/mm"),
            (1011, "Z Steps/mm", "Z-axis steps per mm", false, "steps/mm"),
            (1015, "X Max Feedrate", "X-axis maximum feedrate", false, "mm/s"),
            (1019, "Y Max Feedrate", "Y-axis maximum feedrate", false, "mm/s"),
            (1023, "Z Max Feedrate", "Z-axis maximum feedrate", false, "mm/s"),
            (1027, "X Homing Feedrate", "X-axis homing feedrate", false, "mm/min"),
            (1031, "Y Homing Feedrate", "Y-axis homing feedrate", false, "mm/min"),
            (1035, "Z Homing Feedrate", "Z-axis homing feedrate", false, "mm/min"),
            (1039, "Max Jerk", "Maximum XY jerk", false, "mm/s"),
            (1047, "Max Z-Jerk", "Maximum Z jerk", false, "mm/s"),
            (1133, "X Min Pos", "X-axis minimum position", false, "mm"),
            (1137, "Y Min Pos", "Y-axis minimum position", false, "mm"),
            (1141, "Z Min Pos", "Z-axis minimum position", false, "mm"),
            (1145, "X Max Length", "X-axis maximum print length", false, "mm"),
            (1149, "Y Max Length", "Y-axis maximum print length", false, "mm"),
            (1153, "Z Max Length", "Z-axis maximum print length", false, "mm"),
            (1051, "X Acceleration", "X-axis acceleration", false, "mm/s²"),
            (1055, "Y Acceleration", "Y-axis acceleration", false, "mm/s²"),
            (1059, "Z Acceleration", "Z-axis acceleration", false, "mm/s²"),
            (1063, "X Travel Accel", "X-axis travel acceleration", false, "mm/s²"),
            (1067, "Y Travel Accel", "Y-axis travel acceleration", false, "mm/s²"),
            (1071, "Z Travel Accel", "Z-axis travel acceleration", false, "mm/s²"),
            (1106, "Bed Heat Manager", "Heated bed temperature manager", false, "mode"),
            (1107, "Bed PID Drive Max", "Heated bed PID max drive", false, ""),
            (1124, "Bed PID Drive Min", "Heated bed PID min drive", false, ""),
            (1108, "Bed PID P-Gain", "Heated bed PID proportional gain", false, ""),
            (1112, "Bed PID I-Gain", "Heated bed PID integral gain", false, ""),
            (1116, "Bed PID D-Gain", "Heated bed PID derivative gain", false, ""),
            (1120, "Bed PID Max Value", "Heated bed PID max PWM value", false, ""),
            (1200, "Extr.1 Steps/mm", "Extruder 1 steps per mm", false, "steps/mm"),
            (1204, "Extr.1 Max Feedrate", "Extruder 1 maximum feedrate", false, "mm/s"),
            (1208, "Extr.1 Start Feedrate", "Extruder 1 start feedrate", false, "mm/s"),
            (1212, "Extr.1 Acceleration", "Extruder 1 acceleration", false, "mm/s²"),
            (1216, "Extr.1 Heat Manager", "Extruder 1 temperature manager", false, "mode"),
            (1217, "Extr.1 PID Drive Max", "Extruder 1 PID max drive", false, ""),
            (1245, "Extr.1 PID Drive Min", "Extruder 1 PID min drive", false, ""),
            (1218, "Extr.1 PID P-Gain", "Extruder 1 PID proportional gain", false, ""),
            (1222, "Extr.1 PID I-Gain", "Extruder 1 PID integral gain", false, ""),
            (1226, "Extr.1 PID D-Gain", "Extruder 1 PID derivative gain", false, ""),
            (1230, "Extr.1 PID Max Value", "Extruder 1 PID max PWM value", false, ""),
            (1231, "Extr.1 X-Offset", "Extruder 1 X offset", false, "steps"),
            (1235, "Extr.1 Y-Offset", "Extruder 1 Y offset", false, "steps"),
            (1239, "Extr.1 Temp. Stabilize", "Extruder 1 temp stabilize time", false, "s"),
        ]
    }
}
