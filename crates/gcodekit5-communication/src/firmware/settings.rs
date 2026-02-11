//! Firmware settings framework
//!
//! Provides traits and implementations for managing firmware-specific settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A firmware setting parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareSetting {
    /// Setting ID or code
    pub id: String,
    /// Current value
    pub value: String,
    /// Setting description
    pub description: String,
    /// Setting type
    pub setting_type: SettingType,
    /// Minimum value (for numeric settings)
    pub min: Option<f64>,
    /// Maximum value (for numeric settings)
    pub max: Option<f64>,
}

/// Setting data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingType {
    /// Numeric value
    Numeric,
    /// String value
    String,
    /// Boolean flag
    Boolean,
    /// Selection from options
    Enum,
}

/// Trait for firmware settings management
pub trait FirmwareSettingsTrait: Send + Sync {
    /// Get a setting by ID
    fn get_setting(&self, id: &str) -> Option<FirmwareSetting>;

    /// Get all settings
    fn get_all_settings(&self) -> Vec<FirmwareSetting>;

    /// Set a setting value
    fn set_setting(&mut self, id: &str, value: String) -> anyhow::Result<()>;

    /// Validate a setting value
    fn validate_setting(&self, id: &str, value: &str) -> anyhow::Result<()>;

    /// Get settings as a map
    fn settings_map(&self) -> HashMap<String, String> {
        self.get_all_settings()
            .iter()
            .map(|s| (s.id.clone(), s.value.clone()))
            .collect()
    }

    /// Load settings from file
    fn load_from_file(&mut self, path: &str) -> anyhow::Result<()>;

    /// Save settings to file
    fn save_to_file(&self, path: &str) -> anyhow::Result<()>;
}

/// Default implementation of firmware settings
#[derive(Debug, Clone)]
pub struct DefaultFirmwareSettings {
    settings: HashMap<String, FirmwareSetting>,
}

impl DefaultFirmwareSettings {
    /// Create a new settings instance
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    /// Add a setting
    pub fn add_setting(&mut self, setting: FirmwareSetting) {
        self.settings.insert(setting.id.clone(), setting);
    }
}

impl Default for DefaultFirmwareSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl FirmwareSettingsTrait for DefaultFirmwareSettings {
    fn get_setting(&self, id: &str) -> Option<FirmwareSetting> {
        self.settings.get(id).cloned()
    }

    fn get_all_settings(&self) -> Vec<FirmwareSetting> {
        self.settings.values().cloned().collect()
    }

    fn set_setting(&mut self, id: &str, value: String) -> anyhow::Result<()> {
        self.validate_setting(id, &value)?;
        if let Some(setting) = self.settings.get_mut(id) {
            setting.value = value;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Setting not found: {}", id))
        }
    }

    fn validate_setting(&self, id: &str, value: &str) -> anyhow::Result<()> {
        if let Some(setting) = self.settings.get(id) {
            match setting.setting_type {
                SettingType::Numeric => {
                    let num: f64 = value.parse()?;
                    if let Some(min) = setting.min {
                        if num < min {
                            return Err(anyhow::anyhow!("Value {} is below minimum {}", num, min));
                        }
                    }
                    if let Some(max) = setting.max {
                        if num > max {
                            return Err(anyhow::anyhow!("Value {} is above maximum {}", num, max));
                        }
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        } else {
            Err(anyhow::anyhow!("Setting not found: {}", id))
        }
    }

    fn load_from_file(&mut self, path: &str) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let settings: Vec<FirmwareSetting> = serde_json::from_str(&content)?;
        for setting in settings {
            self.settings.insert(setting.id.clone(), setting);
        }
        Ok(())
    }

    fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let settings: Vec<&FirmwareSetting> = self.settings.values().collect();
        let content = serde_json::to_string_pretty(&settings)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
