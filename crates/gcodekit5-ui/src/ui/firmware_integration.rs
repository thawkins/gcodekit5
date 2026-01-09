//! Firmware Settings Integration - Task 78 Phase 3
//!
//! Integrates device-specific firmware parameters into the Settings Dialog.
//! Bridges FirmwareSettingsPanel with SettingsDialog and Config persistence.
//! Provides firmware parameter management, validation, and backup/restore.

use crate::ui::firmware_settings_panel::{FirmwareParameter, FirmwareSettingsPanel, ParameterType};
use gcodekit5_settings::{Setting, SettingValue, SettingsCategory, SettingsDialog};
use gcodekit5_core::Result;
use std::collections::HashMap;
use tracing::warn;

/// Firmware settings integration
#[derive(Debug, Clone)]
pub struct FirmwareSettingsIntegration {
    /// Firmware settings panel
    pub firmware_panel: FirmwareSettingsPanel,
    /// Cached parameters for UI
    cached_parameters: HashMap<String, FirmwareParameter>,
    /// Whether firmware settings are loaded
    pub is_loaded: bool,
}

impl FirmwareSettingsIntegration {
    /// Create new firmware settings integration
    pub fn new(firmware_type: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            firmware_panel: FirmwareSettingsPanel::new(firmware_type, version),
            cached_parameters: HashMap::new(),
            is_loaded: false,
        }
    }

    /// Load GRBL firmware settings
    pub fn load_grbl_defaults(&mut self) -> Result<()> {
        self.firmware_panel = FirmwareSettingsPanel::new("GRBL", "1.1");

        // GRBL 1.1 Standard Settings
        let grbl_settings = vec![
            // Step Pulse Microseconds
            FirmwareParameter::new("$0", "Step Pulse Microseconds", "10")
                .with_type(ParameterType::Integer)
                .with_range(1.0, 127.0)
                .with_unit("μs")
                .with_description("Length of step pulse in microseconds"),
            // Stepping Idle Delay
            FirmwareParameter::new("$1", "Stepping Idle Delay", "25")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 254.0)
                .with_unit("ms")
                .with_description("Delay for motor idle detection"),
            // Step Port Invert
            FirmwareParameter::new("$2", "Step Port Invert", "0")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 255.0)
                .with_description("Step port polarity mask"),
            // Direction Port Invert
            FirmwareParameter::new("$3", "Direction Port Invert", "0")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 255.0)
                .with_description("Direction port polarity mask"),
            // Stepper Enable Invert
            FirmwareParameter::new("$4", "Stepper Enable Invert", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert stepper enable pin"),
            // Limit Pins Invert
            FirmwareParameter::new("$5", "Limit Pins Invert", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert limit pins"),
            // Probe Pin Invert
            FirmwareParameter::new("$6", "Probe Pin Invert", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert probe pin"),
            // Status Report Mask
            FirmwareParameter::new("$10", "Status Report Mask", "1")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 255.0)
                .with_description("Status report option mask"),
            // Junction Deviation
            FirmwareParameter::new("$11", "Junction Deviation", "0.01")
                .with_type(ParameterType::Float)
                .with_range(0.0, 1.0)
                .with_unit("mm")
                .with_description("Arc tolerance for corner deviation"),
            // Arc Tolerance
            FirmwareParameter::new("$12", "Arc Tolerance", "0.002")
                .with_type(ParameterType::Float)
                .with_range(0.0, 1.0)
                .with_unit("mm")
                .with_description("Maximum arc segment deviation"),
            // Report Inches
            FirmwareParameter::new("$13", "Report Inches", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Report position in inches (true) or mm (false)"),
            // Control Pin Invert
            FirmwareParameter::new("$14", "Control Pin Invert", "0")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 255.0)
                .with_description("Control pin invert mask"),
            // Control Pin Pull-Up
            FirmwareParameter::new("$15", "Control Pin Pull-Up", "0")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 255.0)
                .with_description("Control pin pull-up mask"),
            // Limit Pins Pull-Up
            FirmwareParameter::new("$16", "Limit Pins Pull-Up", "1")
                .with_type(ParameterType::Boolean)
                .with_description("Limit pins pull-up enable"),
            // Probe Pin Pull-Up
            FirmwareParameter::new("$17", "Probe Pin Pull-Up", "1")
                .with_type(ParameterType::Boolean)
                .with_description("Probe pin pull-up enable"),
            // Spindle PWM Frequency
            FirmwareParameter::new("$33", "Spindle PWM Frequency", "5000")
                .with_type(ParameterType::Integer)
                .with_range(0.0, 65535.0)
                .with_unit("Hz")
                .with_description("Spindle PWM frequency"),
            // Spindle PWM Off Value
            FirmwareParameter::new("$34", "Spindle PWM Off Value", "0")
                .with_type(ParameterType::Float)
                .with_range(0.0, 100.0)
                .with_unit("%")
                .with_description("Spindle off PWM output percentage"),
            // Spindle PWM Min Value
            FirmwareParameter::new("$35", "Spindle PWM Min Value", "0")
                .with_type(ParameterType::Float)
                .with_range(0.0, 100.0)
                .with_unit("%")
                .with_description("Spindle minimum PWM output percentage"),
            // Spindle PWM Max Value
            FirmwareParameter::new("$36", "Spindle PWM Max Value", "100")
                .with_type(ParameterType::Float)
                .with_range(0.0, 100.0)
                .with_unit("%")
                .with_description("Spindle maximum PWM output percentage"),
            // Spindle Enable Invert
            FirmwareParameter::new("$37", "Spindle Enable Invert", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert spindle enable pin"),
            // Spindle Direction Invert
            FirmwareParameter::new("$38", "Spindle Direction Invert", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert spindle direction pin"),
            // X Steps/mm
            FirmwareParameter::new("$100", "X Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("steps/mm")
                .with_description("X-axis steps per millimeter"),
            // Y Steps/mm
            FirmwareParameter::new("$101", "Y Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("steps/mm")
                .with_description("Y-axis steps per millimeter"),
            // Z Steps/mm
            FirmwareParameter::new("$102", "Z Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("steps/mm")
                .with_description("Z-axis steps per millimeter"),
            // X Max Rate
            FirmwareParameter::new("$110", "X Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("mm/min")
                .with_description("X-axis maximum rate"),
            // Y Max Rate
            FirmwareParameter::new("$111", "Y Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("mm/min")
                .with_description("Y-axis maximum rate"),
            // Z Max Rate
            FirmwareParameter::new("$112", "Z Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 30000.0)
                .with_unit("mm/min")
                .with_description("Z-axis maximum rate"),
            // X Acceleration
            FirmwareParameter::new("$120", "X Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 1000.0)
                .with_unit("mm/s²")
                .with_description("X-axis acceleration"),
            // Y Acceleration
            FirmwareParameter::new("$121", "Y Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 1000.0)
                .with_unit("mm/s²")
                .with_description("Y-axis acceleration"),
            // Z Acceleration
            FirmwareParameter::new("$122", "Z Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 1000.0)
                .with_unit("mm/s²")
                .with_description("Z-axis acceleration"),
            // X Max Travel
            FirmwareParameter::new("$130", "X Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 10000.0)
                .with_unit("mm")
                .with_description("X-axis maximum travel distance"),
            // Y Max Travel
            FirmwareParameter::new("$131", "Y Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 10000.0)
                .with_unit("mm")
                .with_description("Y-axis maximum travel distance"),
            // Z Max Travel
            FirmwareParameter::new("$132", "Z Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_range(1.0, 10000.0)
                .with_unit("mm")
                .with_description("Z-axis maximum travel distance"),
        ];

        for param in grbl_settings {
            self.firmware_panel.add_parameter(param);
        }

        self.is_loaded = true;
        Ok(())
    }

    /// Load grblHAL firmware settings (compatible with GRBL but with extensions)
    pub fn load_grblhal_defaults(&mut self) -> Result<()> {
        // grblHAL is GRBL-compatible, so start with GRBL settings
        self.load_grbl_defaults()?;
        
        // Update firmware type
        self.firmware_panel.firmware_type = "grblHAL".to_string();
        self.firmware_panel.firmware_version = "1.1f".to_string();

        // grblHAL has all GRBL settings writable
        Ok(())
    }

    /// Load FluidNC firmware settings (read-only)
    pub fn load_fluidnc_defaults(&mut self) -> Result<()> {
        self.firmware_panel = FirmwareSettingsPanel::new("FluidNC", "3.0");

        // FluidNC settings are read-only (configured via YAML files)
        // We still display them but mark as read-only
        let fluidnc_settings = vec![
            // Note: All FluidNC parameters are read-only
            FirmwareParameter::new("$0", "Step Pulse Microseconds", "4")
                .with_type(ParameterType::Integer)
                .with_unit("μs")
                .with_description("Length of step pulse in microseconds")
                .read_only(),
            FirmwareParameter::new("$1", "Stepper Idle Lock Time", "250")
                .with_type(ParameterType::Integer)
                .with_unit("ms")
                .with_description("Delay for motor idle detection")
                .read_only(),
            FirmwareParameter::new("$2", "Step Port Invert", "0")
                .with_type(ParameterType::Integer)
                .with_description("Step port polarity mask")
                .read_only(),
            FirmwareParameter::new("$3", "Direction Port Invert", "0")
                .with_type(ParameterType::Integer)
                .with_description("Direction port polarity mask")
                .read_only(),
            FirmwareParameter::new("$4", "Invert Step Enable Pin", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert stepper enable pin")
                .read_only(),
            FirmwareParameter::new("$5", "Invert Limit Pins", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert limit pins")
                .read_only(),
            FirmwareParameter::new("$6", "Invert Probe Pin", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Invert probe pin")
                .read_only(),
            FirmwareParameter::new("$10", "Status Report Options", "1")
                .with_type(ParameterType::Integer)
                .with_description("Status report option mask")
                .read_only(),
            FirmwareParameter::new("$11", "Junction Deviation", "0.01")
                .with_type(ParameterType::Float)
                .with_unit("mm")
                .with_description("Arc tolerance for corner deviation")
                .read_only(),
            FirmwareParameter::new("$12", "Arc Tolerance", "0.002")
                .with_type(ParameterType::Float)
                .with_unit("mm")
                .with_description("Maximum arc segment deviation")
                .read_only(),
            FirmwareParameter::new("$13", "Report in Inches", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Report position in inches (true) or mm (false)")
                .read_only(),
            FirmwareParameter::new("$20", "Soft Limits Enable", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Enable soft limits")
                .read_only(),
            FirmwareParameter::new("$21", "Hard Limits Enable", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Enable hard limits")
                .read_only(),
            FirmwareParameter::new("$22", "Homing Cycle Enable", "0")
                .with_type(ParameterType::Boolean)
                .with_description("Enable homing cycle")
                .read_only(),
            FirmwareParameter::new("$100", "X-axis Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_unit("steps/mm")
                .with_description("X-axis steps per millimeter")
                .read_only(),
            FirmwareParameter::new("$101", "Y-axis Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_unit("steps/mm")
                .with_description("Y-axis steps per millimeter")
                .read_only(),
            FirmwareParameter::new("$102", "Z-axis Steps per mm", "250.0")
                .with_type(ParameterType::Float)
                .with_unit("steps/mm")
                .with_description("Z-axis steps per millimeter")
                .read_only(),
            FirmwareParameter::new("$110", "X-axis Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/min")
                .with_description("X-axis maximum rate")
                .read_only(),
            FirmwareParameter::new("$111", "Y-axis Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/min")
                .with_description("Y-axis maximum rate")
                .read_only(),
            FirmwareParameter::new("$112", "Z-axis Max Rate", "1000.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/min")
                .with_description("Z-axis maximum rate")
                .read_only(),
            FirmwareParameter::new("$120", "X-axis Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/s²")
                .with_description("X-axis acceleration")
                .read_only(),
            FirmwareParameter::new("$121", "Y-axis Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/s²")
                .with_description("Y-axis acceleration")
                .read_only(),
            FirmwareParameter::new("$122", "Z-axis Acceleration", "10.0")
                .with_type(ParameterType::Float)
                .with_unit("mm/s²")
                .with_description("Z-axis acceleration")
                .read_only(),
            FirmwareParameter::new("$130", "X-axis Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_unit("mm")
                .with_description("X-axis maximum travel distance")
                .read_only(),
            FirmwareParameter::new("$131", "Y-axis Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_unit("mm")
                .with_description("Y-axis maximum travel distance")
                .read_only(),
            FirmwareParameter::new("$132", "Z-axis Max Travel", "200.0")
                .with_type(ParameterType::Float)
                .with_unit("mm")
                .with_description("Z-axis maximum travel distance")
                .read_only(),
        ];

        for param in fluidnc_settings {
            self.firmware_panel.add_parameter(param);
        }

        self.is_loaded = true;
        Ok(())
    }

    /// Populate SettingsDialog with firmware parameters
    pub fn populate_dialog(&mut self, dialog: &mut SettingsDialog) {
        if !self.is_loaded {
            warn!("Firmware settings not loaded, skipping dialog population");
            return;
        }

        let params = self.firmware_panel.list_parameters();

        for param in params {
            let description = param
                .description
                .clone()
                .unwrap_or_else(|| "Firmware parameter".to_string());

            let value_str = param.value.clone();

            // Create setting based on parameter type
            let setting_value = match param.param_type {
                ParameterType::Integer => SettingValue::String(value_str),
                ParameterType::Float => SettingValue::String(value_str),
                ParameterType::Boolean => {
                    let bool_val =
                        matches!(value_str.to_lowercase().as_str(), "1" | "true" | "yes");
                    SettingValue::Boolean(bool_val)
                }
                ParameterType::String => SettingValue::String(value_str),
            };

            let display_name = format!("{} ({})", param.name, param.code);
            let full_description = if let Some(unit) = &param.unit {
                format!("{} [{}]", description, unit)
            } else {
                description
            };

            let setting = Setting::new(format!("fw_{}", param.code), display_name, setting_value)
                .with_description(full_description)
                .with_category(SettingsCategory::Advanced);

            dialog.add_setting(setting);

            // Cache for later lookup
            self.cached_parameters
                .insert(param.code.clone(), param.clone());
        }
    }

    /// Update firmware parameters from dialog
    pub fn update_from_dialog(&mut self, dialog: &SettingsDialog) -> Result<()> {
        let params_to_update: Vec<(String, String)> = self
            .firmware_panel
            .list_parameters()
            .iter()
            .filter_map(|param| {
                let setting_id = format!("fw_{}", param.code);
                dialog
                    .get_setting(&setting_id)
                    .map(|setting| (param.code.clone(), setting.value.as_str().to_string()))
            })
            .collect();

        for (param_code, new_value) in params_to_update {
            match self
                .firmware_panel
                .set_parameter_value(&param_code, new_value.clone())
            {
                Ok(_) => {}
                Err(e) => {
                    warn!("Failed to update parameter {}: {}", param_code, e);
                }
            }
        }

        Ok(())
    }

    /// Get modified parameters
    pub fn get_modified_parameters(&self) -> Vec<&FirmwareParameter> {
        self.firmware_panel.get_modified_parameters()
    }

    /// Create backup of firmware parameters
    pub fn create_backup(&mut self) {
        self.firmware_panel.create_backup();
    }

    /// Restore firmware parameters from backup
    pub fn restore_from_backup(&mut self) -> Result<()> {
        self.firmware_panel
            .restore_backup()
            .map_err(gcodekit5_core::Error::other)
    }

    /// Reset all parameters to defaults
    pub fn reset_to_defaults(&mut self) {
        self.firmware_panel.reset_all_to_defaults();
    }

    /// Get firmware type and version
    pub fn firmware_info(&self) -> (String, String) {
        (
            self.firmware_panel.firmware_type.clone(),
            self.firmware_panel.firmware_version.clone(),
        )
    }

    /// Check if firmware settings are modified
    pub fn has_changes(&self) -> bool {
        !self.firmware_panel.get_modified_parameters().is_empty()
    }

    /// Get count of parameters
    pub fn parameter_count(&self) -> usize {
        self.firmware_panel.list_parameters().len()
    }
}

impl Default for FirmwareSettingsIntegration {
    fn default() -> Self {
        Self::new("Unknown", "Unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firmware_integration_new() {
        let integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        assert!(!integration.is_loaded);
    }

    #[test]
    fn test_load_grbl_defaults() {
        let mut integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        assert!(integration.load_grbl_defaults().is_ok());
        assert!(integration.is_loaded);
        assert!(integration.parameter_count() > 0);
    }

    #[test]
    fn test_populate_dialog() {
        let mut integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        let _ = integration.load_grbl_defaults();

        let mut dialog = SettingsDialog::new();
        integration.populate_dialog(&mut dialog);

        assert!(!dialog.settings.is_empty());
        assert!(dialog.get_setting("fw_$100").is_some());
    }

    #[test]
    fn test_modified_parameters() {
        let mut integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        let _ = integration.load_grbl_defaults();

        // Modify a parameter
        let _ = integration
            .firmware_panel
            .set_parameter_value("$100", "500.0");

        let modified = integration.get_modified_parameters();
        assert!(!modified.is_empty());
    }

    #[test]
    fn test_firmware_info() {
        let integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        let (fw_type, fw_version) = integration.firmware_info();
        assert_eq!(fw_type, "GRBL");
        assert_eq!(fw_version, "1.1");
    }

    #[test]
    fn test_reset_to_defaults() {
        let mut integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        let _ = integration.load_grbl_defaults();

        // Modify a parameter
        let _ = integration
            .firmware_panel
            .set_parameter_value("$100", "500.0");
        assert!(integration.has_changes());

        // Reset
        integration.reset_to_defaults();
        assert!(!integration.has_changes());
    }

    #[test]
    fn test_parameter_count() {
        let mut integration = FirmwareSettingsIntegration::new("GRBL", "1.1");
        let _ = integration.load_grbl_defaults();
        assert!(integration.parameter_count() > 30);
    }
}
