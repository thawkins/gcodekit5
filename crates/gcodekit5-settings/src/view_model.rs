//! Settings Dialog Logic
//!
//! Application settings with categories for controller settings, UI preferences,
//! file processing options, and keyboard shortcuts configuration

use std::collections::HashMap;

/// Keyboard shortcut definition
#[derive(Debug, Clone)]
pub struct KeyboardShortcut {
    /// Action ID
    pub action_id: String,
    /// Action description
    pub description: String,
    /// Key combination (e.g., "Ctrl+S", "F5")
    pub keys: String,
    /// Whether this is customizable
    pub customizable: bool,
}

impl KeyboardShortcut {
    /// Create new keyboard shortcut
    pub fn new(
        action_id: impl Into<String>,
        desc: impl Into<String>,
        keys: impl Into<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            description: desc.into(),
            keys: keys.into(),
            customizable: true,
        }
    }

    /// Set as non-customizable (system)
    pub fn system(mut self) -> Self {
        self.customizable = false;
        self
    }
}

/// Settings category
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SettingsCategory {
    /// Controller connection settings
    Controller,
    /// General application settings
    General,
    /// UI appearance and behavior
    UserInterface,
    /// File processing options
    FileProcessing,
    /// Keyboard shortcuts
    KeyboardShortcuts,
    /// Machine and axis configuration
    Machine,
    /// Advanced options
    Advanced,
}

impl std::fmt::Display for SettingsCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Controller => write!(f, "Controller"),
            Self::General => write!(f, "General"),
            Self::UserInterface => write!(f, "User Interface"),
            Self::FileProcessing => write!(f, "File Processing"),
            Self::KeyboardShortcuts => write!(f, "Keyboard Shortcuts"),
            Self::Machine => write!(f, "Machine"),
            Self::Advanced => write!(f, "Advanced"),
        }
    }
}

/// Setting value type
#[derive(Debug, Clone)]
pub enum SettingValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i32),
    /// Floating point value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// File system path
    Path(String),
    /// Enumeration with options
    Enum(String, Vec<String>),
}

impl SettingValue {
    /// Get as string
    pub fn as_str(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) => f.to_string(),
            Self::Boolean(b) => b.to_string(),
            Self::Path(p) => p.clone(),
            Self::Enum(s, _) => s.clone(),
        }
    }
}

/// Single setting item
#[derive(Debug, Clone)]
pub struct Setting {
    /// Setting ID
    pub id: String,
    /// Setting display name
    pub name: String,
    /// Setting description
    pub description: Option<String>,
    /// Current value
    pub value: SettingValue,
    /// Default value
    pub default: SettingValue,
    /// Category
    pub category: SettingsCategory,
}

impl Setting {
    /// Create new setting
    pub fn new(id: impl Into<String>, name: impl Into<String>, value: SettingValue) -> Self {
        let default = value.clone();
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            value,
            default,
            category: SettingsCategory::Advanced,
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set category
    pub fn with_category(mut self, category: SettingsCategory) -> Self {
        self.category = category;
        self
    }

    /// Reset to default
    pub fn reset_to_default(&mut self) {
        self.value = self.default.clone();
    }

    /// Check if changed
    pub fn is_changed(&self) -> bool {
        match (&self.value, &self.default) {
            (SettingValue::String(a), SettingValue::String(b)) => a != b,
            (SettingValue::Integer(a), SettingValue::Integer(b)) => a != b,
            (SettingValue::Float(a), SettingValue::Float(b)) => (a - b).abs() > f64::EPSILON,
            (SettingValue::Boolean(a), SettingValue::Boolean(b)) => a != b,
            (SettingValue::Path(a), SettingValue::Path(b)) => a != b,
            (SettingValue::Enum(a, _), SettingValue::Enum(b, _)) => a != b,
            _ => false,
        }
    }
}

/// Settings and Preferences manager
#[derive(Debug, Clone)]
pub struct SettingsDialog {
    /// All settings
    pub settings: HashMap<String, Setting>,
    /// Keyboard shortcuts
    pub shortcuts: HashMap<String, KeyboardShortcut>,
    /// Current selected category
    pub selected_category: SettingsCategory,
    /// Unsaved changes flag
    pub has_unsaved_changes: bool,
}

impl SettingsDialog {
    /// Create new settings dialog
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
            shortcuts: HashMap::new(),
            selected_category: SettingsCategory::General,
            has_unsaved_changes: false,
        }
    }

    /// Add setting
    pub fn add_setting(&mut self, setting: Setting) {
        self.settings.insert(setting.id.clone(), setting);
    }

    /// Get setting by ID
    pub fn get_setting(&self, id: &str) -> Option<&Setting> {
        self.settings.get(id)
    }

    /// Get mutable setting
    pub fn get_setting_mut(&mut self, id: &str) -> Option<&mut Setting> {
        self.settings.get_mut(id)
    }

    /// Update setting value
    pub fn update_setting(&mut self, id: &str, value: SettingValue) -> bool {
        if let Some(setting) = self.settings.get_mut(id) {
            if !matches!(setting.value, SettingValue::Enum(_, ref options) if !options.contains(&value.as_str()))
            {
                setting.value = value;
                self.has_unsaved_changes = true;
                return true;
            }
        }
        false
    }

    /// Add keyboard shortcut
    pub fn add_shortcut(&mut self, shortcut: KeyboardShortcut) {
        self.shortcuts.insert(shortcut.action_id.clone(), shortcut);
    }

    /// Update keyboard shortcut
    pub fn update_shortcut(&mut self, action_id: &str, keys: impl Into<String>) -> bool {
        if let Some(shortcut) = self.shortcuts.get_mut(action_id) {
            if shortcut.customizable {
                shortcut.keys = keys.into();
                self.has_unsaved_changes = true;
                return true;
            }
        }
        false
    }

    /// Get settings for category
    pub fn get_settings_for_category(&self, category: &SettingsCategory) -> Vec<&Setting> {
        self.settings
            .values()
            .filter(|s| &s.category == category)
            .collect()
    }

    /// List all categories with settings
    pub fn get_categories(&self) -> Vec<SettingsCategory> {
        let mut categories = std::collections::HashSet::new();
        for setting in self.settings.values() {
            categories.insert(setting.category.clone());
        }
        let mut sorted: Vec<_> = categories.into_iter().collect();
        sorted.sort_by_key(|c| format!("{}", c));
        sorted
    }

    /// Reset all settings to defaults
    pub fn reset_all_to_defaults(&mut self) {
        for setting in self.settings.values_mut() {
            setting.reset_to_default();
        }
        self.has_unsaved_changes = true;
    }

    /// Check if any settings changed
    pub fn has_changes(&self) -> bool {
        self.settings.values().any(|s| s.is_changed())
    }

    /// Export settings to JSON
    pub fn export_settings(&self) -> Result<String, serde_json::Error> {
        let settings_map: HashMap<String, String> = self
            .settings
            .iter()
            .map(|(k, v)| (k.clone(), v.value.as_str()))
            .collect();
        serde_json::to_string_pretty(&settings_map)
    }

    /// Import settings from JSON
    pub fn import_settings(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let settings_map: HashMap<String, String> = serde_json::from_str(json)?;
        for (key, value) in settings_map {
            if let Some(setting) = self.settings.get_mut(&key) {
                let new_value = match &setting.value {
                    SettingValue::String(_) => SettingValue::String(value),
                    SettingValue::Integer(_) => SettingValue::Integer(value.parse().unwrap_or(0)),
                    SettingValue::Float(_) => SettingValue::Float(value.parse().unwrap_or(0.0)),
                    SettingValue::Boolean(_) => {
                        SettingValue::Boolean(value.parse().unwrap_or(false))
                    }
                    SettingValue::Path(_) => SettingValue::Path(value),
                    SettingValue::Enum(_, ref options) => {
                        if options.contains(&value) {
                            SettingValue::Enum(value, options.clone())
                        } else {
                            continue;
                        }
                    }
                };
                setting.value = new_value;
            }
        }
        self.has_unsaved_changes = true;
        Ok(())
    }

    /// Discard changes and select category
    pub fn select_category(&mut self, category: SettingsCategory) {
        self.selected_category = category;
    }
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}
