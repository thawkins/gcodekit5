//! Settings Controller
//!
//! Handles interaction between the UI and the Settings Model.
//! Provides data transformation for UI consumption and handles user actions.

use crate::manager::SettingsManager;
use crate::persistence::SettingsPersistence;
use crate::view_model::{SettingValue, SettingsCategory, SettingsDialog};
use std::cell::RefCell;
use std::rc::Rc;

/// UI-friendly representation of a setting
#[derive(Debug, Clone)]
pub struct SettingUiModel {
    pub id: String,
    pub name: String,
    pub value: String,
    pub value_type: String,
    pub category: String,
    pub description: String,
    pub options: Vec<String>,
    pub current_index: i32,
}

/// Controller for settings logic
pub struct SettingsController {
    pub dialog: Rc<RefCell<SettingsDialog>>,
    pub persistence: Rc<RefCell<SettingsPersistence>>,
    #[allow(clippy::type_complexity)]
    listeners: Rc<RefCell<Vec<Box<dyn Fn(&str, &str)>>>>,
}

impl SettingsController {
    /// Create new settings controller
    pub fn new(
        dialog: Rc<RefCell<SettingsDialog>>,
        persistence: Rc<RefCell<SettingsPersistence>>,
    ) -> Self {
        Self {
            dialog,
            persistence,
            listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Register a callback to be notified when a setting changes
    pub fn on_setting_changed<F>(&self, callback: F)
    where
        F: Fn(&str, &str) + 'static,
    {
        self.listeners.borrow_mut().push(Box::new(callback));
    }

    /// Get settings formatted for UI display, optionally filtered by category
    pub fn get_settings_for_ui(
        &self,
        category_filter: Option<SettingsCategory>,
    ) -> Vec<SettingUiModel> {
        let dialog = self.dialog.borrow();
        let mut items = Vec::new();

        let settings = if let Some(cat) = category_filter {
            dialog.get_settings_for_category(&cat)
        } else {
            dialog.settings.values().collect()
        };

        for setting in settings {
            let value_type = match &setting.value {
                SettingValue::Boolean(_) => "Boolean",
                SettingValue::Integer(_) => "Integer",
                SettingValue::Float(_) => "Float",
                SettingValue::Path(_) => "Path",
                SettingValue::Enum(_, _) => "Enum",
                _ => "String",
            };

            let category = format!("{}", setting.category);

            let options: Vec<String> = if let SettingValue::Enum(_, opts) = &setting.value {
                opts.clone()
            } else {
                Vec::new()
            };

            let current_index = if let SettingValue::Enum(val, opts) = &setting.value {
                opts.iter().position(|o| o == val).unwrap_or(0) as i32
            } else {
                0
            };

            items.push(SettingUiModel {
                id: setting.id.clone(),
                name: setting.name.clone(),
                value: setting.value.as_str(),
                value_type: value_type.to_string(),
                category,
                description: setting.description.clone().unwrap_or_default(),
                options,
                current_index,
            });
        }

        // Sort by name for consistent order
        items.sort_by(|a, b| a.name.cmp(&b.name));

        items
    }

    /// Update a setting value from string input
    pub fn update_setting(&self, id: &str, value: &str) {
        let mut dialog = self.dialog.borrow_mut();

        // Determine the new value based on the existing setting's type
        let new_value_opt = if let Some(setting) = dialog.get_setting(id) {
            match &setting.value {
                SettingValue::String(_) => Some(SettingValue::String(value.to_string())),
                SettingValue::Integer(_) => Some(SettingValue::Integer(value.parse().unwrap_or(0))),
                SettingValue::Float(_) => Some(SettingValue::Float(value.parse().unwrap_or(0.0))),
                SettingValue::Boolean(_) => Some(SettingValue::Boolean(value == "true")),
                SettingValue::Path(_) => Some(SettingValue::Path(value.to_string())),
                SettingValue::Enum(_, options) => {
                    if options.contains(&value.to_string()) {
                        Some(SettingValue::Enum(value.to_string(), options.clone()))
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        if let Some(val) = new_value_opt {
            dialog.update_setting(id, val);

            // Notify listeners
            let listeners = self.listeners.borrow();
            for listener in listeners.iter() {
                listener(id, value);
            }
        }
    }

    /// Discard unsaved changes and restore dialog values from the persisted config.
    pub fn discard_changes(&self) {
        let before: std::collections::HashMap<String, String> = {
            let dialog = self.dialog.borrow();
            dialog
                .settings
                .iter()
                .map(|(id, s)| (id.clone(), s.value.as_str()))
                .collect()
        };

        {
            let persistence = self.persistence.borrow();
            let mut dialog = self.dialog.borrow_mut();
            persistence.populate_dialog(&mut dialog);
            dialog.has_unsaved_changes = false;
        }

        let after: std::collections::HashMap<String, String> = {
            let dialog = self.dialog.borrow();
            dialog
                .settings
                .iter()
                .map(|(id, s)| (id.clone(), s.value.as_str()))
                .collect()
        };

        let listeners = self.listeners.borrow();
        for (id, new_val) in after.iter() {
            if before.get(id) != Some(new_val) {
                for listener in listeners.iter() {
                    listener(id, new_val);
                }
            }
        }
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let mut dialog = self.dialog.borrow_mut();
        let mut persistence = self.persistence.borrow_mut();

        persistence
            .load_from_dialog(&dialog)
            .map_err(|e| e.to_string())?;

        let config_path = SettingsManager::config_file_path().map_err(|e| e.to_string())?;
        SettingsManager::ensure_config_dir().map_err(|e| e.to_string())?;

        persistence
            .save_to_file(&config_path)
            .map_err(|e| e.to_string())?;

        dialog.has_unsaved_changes = false;

        Ok(())
    }

    /// Restore default settings
    pub fn restore_defaults(&self) {
        let mut dialog = self.dialog.borrow_mut();
        dialog.reset_all_to_defaults();
    }

    /// Convert string category to enum
    pub fn get_category_from_string(cat: &str) -> SettingsCategory {
        match cat {
            "controller" => SettingsCategory::Controller,
            "general" => SettingsCategory::General,
            "ui" => SettingsCategory::UserInterface,
            "file" => SettingsCategory::FileProcessing,
            "shortcuts" => SettingsCategory::KeyboardShortcuts,
            "advanced" => SettingsCategory::Advanced,
            _ => SettingsCategory::Controller,
        }
    }
}
