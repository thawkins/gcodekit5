//! Settings Controller
//!
//! Handles interaction between the UI and the Settings Model.
//! Provides data transformation for UI consumption and handles user actions.

use crate::persistence::SettingsPersistence;
use crate::view_model::{SettingValue, SettingsCategory, SettingsDialog};
use gcodekit5_core::{shared, Shared, SharedVec};
use std::rc::Rc;

/// Callback invoked with `(setting_id, new_value)` after a setting changes.
type SettingListener = Rc<dyn Fn(&str, &str)>;

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
    pub dialog: Shared<SettingsDialog>,
    pub persistence: Shared<SettingsPersistence>,
    listeners: SharedVec<SettingListener>,
}

impl SettingsController {
    /// Create new settings controller
    pub fn new(dialog: Shared<SettingsDialog>, persistence: Shared<SettingsPersistence>) -> Self {
        Self {
            dialog,
            persistence,
            listeners: shared(Vec::new()),
        }
    }

    /// Register a callback to be notified when a setting changes
    ///
    /// Listeners are always invoked with no borrow of the dialog, persistence or
    /// listener list held, so they may freely read settings or register further
    /// listeners.
    pub fn on_setting_changed<F>(&self, callback: F)
    where
        F: Fn(&str, &str) + 'static,
    {
        self.listeners.borrow_mut().push(Rc::new(callback));
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
        let applied = {
            let mut dialog = self.dialog.borrow_mut();

            // Determine the new value based on the existing setting's type
            let new_value_opt = if let Some(setting) = dialog.get_setting(id) {
                match &setting.value {
                    SettingValue::String(_) => Some(SettingValue::String(value.to_string())),
                    // Unparseable numeric input is rejected rather than silently stored as 0
                    // (which would e.g. zero the window size and make save() fail validation).
                    SettingValue::Integer(_) => value.parse().ok().map(SettingValue::Integer),
                    SettingValue::Float(_) => value.parse().ok().map(SettingValue::Float),
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

            match new_value_opt {
                Some(val) => {
                    dialog.update_setting(id, val);
                    true
                }
                None => false,
            }
            // dialog borrow released here, before any listener runs
        };

        if applied {
            self.notify_listeners(&[(id.to_string(), value.to_string())]);
        }
    }

    /// Invoke every registered listener for each `(id, value)` change.
    ///
    /// Takes a snapshot of the listener list first so no `RefCell` borrow is held
    /// while user callbacks run (a listener may read the dialog or add listeners).
    fn notify_listeners(&self, changes: &[(String, String)]) {
        if changes.is_empty() {
            return;
        }
        let listeners: Vec<SettingListener> = self.listeners.borrow().iter().cloned().collect();
        for (id, value) in changes {
            for listener in &listeners {
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

        let changed: Vec<(String, String)> = after
            .into_iter()
            .filter(|(id, new_val)| before.get(id) != Some(new_val))
            .collect();
        self.notify_listeners(&changed);
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let mut dialog = self.dialog.borrow_mut();
        let mut persistence = self.persistence.borrow_mut();

        persistence
            .load_from_dialog(&dialog)
            .map_err(|e| e.to_string())?;

        persistence.persist().map_err(|e| e.to_string())?;

        dialog.has_unsaved_changes = false;

        Ok(())
    }

    /// Restore default settings
    pub fn restore_defaults(&self) {
        // Setting::default holds the value the dialog was populated with (i.e. the
        // currently saved config), so reset_all_to_defaults() would only discard edits.
        // Build the dialog from a default Config to get the real factory defaults.
        let mut factory = SettingsDialog::new();
        SettingsPersistence::new().populate_dialog(&mut factory);

        let mut changed: Vec<(String, String)> = Vec::new();
        {
            let mut dialog = self.dialog.borrow_mut();
            for (id, default_setting) in factory.settings {
                if let Some(setting) = dialog.get_setting_mut(&id) {
                    let new_str = default_setting.value.as_str();
                    if setting.value.as_str() != new_str {
                        changed.push((id.clone(), new_str));
                    }
                    setting.value = default_setting.value;
                }
            }
            dialog.has_unsaved_changes = true;
        }

        self.notify_listeners(&changed);
    }
}
