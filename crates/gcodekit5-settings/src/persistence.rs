//! Settings Persistence
//!
//! Handles loading and saving application settings from/to configuration files.
//! Bridges SettingsDialog (UI) with Config (persistence layer).
//! Provides validation, migration, and synchronization of settings.

use crate::config::Config;
use crate::view_model::{
    KeyboardShortcut, Setting, SettingValue, SettingsCategory, SettingsDialog,
};
use gcodekit5_core::Result;
use std::path::Path;

/// Settings persistence layer
#[derive(Debug, Clone)]
pub struct SettingsPersistence {
    config: Config,
}

impl SettingsPersistence {
    /// Create new persistence layer with default config
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    /// Load settings from file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let config = Config::load_from_file(path)?;
        Ok(Self { config })
    }

    /// Save settings to file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        self.config.save_to_file(path)?;
        Ok(())
    }

    /// Populate SettingsDialog from config
    pub fn populate_dialog(&self, dialog: &mut SettingsDialog) {
        // Connection Settings - Moved to DeviceDB
        // self.add_connection_settings(dialog);

        // General Settings
        self.add_general_settings(dialog);

        // UI Settings
        self.add_ui_settings(dialog);

        // File Processing Settings
        self.add_file_processing_settings(dialog);

        // Keyboard Shortcuts (from config if available)
        self.add_keyboard_shortcuts(dialog);
    }

    /// Load settings from dialog into config
    pub fn load_from_dialog(&mut self, dialog: &SettingsDialog) -> Result<()> {
        // Update connection settings - Moved to DeviceDB
        // self.update_connection_settings(dialog)?;

        // Update General settings
        self.update_general_settings(dialog)?;

        // Update UI settings
        self.update_ui_settings(dialog)?;

        // Update file processing settings
        self.update_file_processing_settings(dialog)?;

        // Validate updated config
        self.config.validate()?;

        Ok(())
    }

    /// Get reference to config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable reference to config
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Validate settings
    pub fn validate(&self) -> Result<()> {
        self.config.validate()
    }

    /// Add General settings to dialog
    fn add_general_settings(&self, dialog: &mut SettingsDialog) {
        let file = &self.config.file_processing;
        let ui = &self.config.ui;

        // Measurement System
        let systems = vec!["Metric".to_string(), "Imperial".to_string()];
        dialog.add_setting(
            Setting::new(
                "measurement_system",
                "Measurement System",
                SettingValue::Enum(ui.measurement_system.to_string(), systems),
            )
            .with_description("Units for display and input (Metric/mm or Imperial/inch)")
            .with_category(SettingsCategory::General),
        );

        // Feed Rate Units
        let feed_units = vec![
            "mm/min".to_string(),
            "mm/sec".to_string(),
            "in/min".to_string(),
            "in/sec".to_string(),
        ];
        dialog.add_setting(
            Setting::new(
                "feed_rate_units",
                "Feed Rate Units",
                SettingValue::Enum(ui.feed_rate_units.to_string(), feed_units),
            )
            .with_description("Units for feed rate display and input")
            .with_category(SettingsCategory::General),
        );

        // Default Directory
        dialog.add_setting(
            Setting::new(
                "default_directory",
                "Default Directory",
                SettingValue::Path(file.output_directory.to_string_lossy().to_string()),
            )
            .with_description("Default directory for file operations")
            .with_category(SettingsCategory::General),
        );
    }

    /// Add UI settings to dialog
    fn add_ui_settings(&self, dialog: &mut SettingsDialog) {
        let ui = &self.config.ui;

        // Theme
        let themes = vec!["System".to_string(), "Light".to_string(), "Dark".to_string()];
        dialog.add_setting(
            Setting::new(
                "theme",
                "Theme",
                SettingValue::Enum(ui.theme.to_string(), themes),
            )
            .with_description("Application color theme (Light, Dark, or System default)")
            .with_category(SettingsCategory::UserInterface),
        );

        // Language
        let languages = vec![
            "System".to_string(),
            "English".to_string(),
            "French".to_string(),
            "German".to_string(),
            "Spanish".to_string(),
            "Portuguese".to_string(),
            "Italian".to_string(),
        ];
        let current_language = match ui.language.as_str() {
            "en" => "English",
            "fr" => "French",
            "de" => "German",
            "es" => "Spanish",
            "pt" => "Portuguese",
            "it" => "Italian",
            _ => "System",
        };
        dialog.add_setting(
            Setting::new(
                "language",
                "Language",
                SettingValue::Enum(current_language.to_string(), languages),
            )
            .with_description("User interface language (requires restart)")
            .with_category(SettingsCategory::UserInterface),
        );

        // Startup Tab
        let startup_tabs = vec![
            "Machine".to_string(),
            "Console".to_string(),
            "Editor".to_string(),
            "Visualizer".to_string(),
            "CamTools".to_string(),
            "Designer".to_string(),
            "DeviceInfo".to_string(),
            "Config".to_string(),
            "Devices".to_string(),
            "Tools".to_string(),
            "Materials".to_string(),
        ];
        dialog.add_setting(
            Setting::new(
                "startup_tab",
                "Startup Tab",
                SettingValue::Enum(ui.startup_tab.to_string(), startup_tabs),
            )
            .with_description("Tab to show when application starts")
            .with_category(SettingsCategory::UserInterface),
        );

        // Window Width
        dialog.add_setting(
            Setting::new(
                "window_width",
                "Window Width",
                SettingValue::Integer(ui.window_width as i32),
            )
            .with_description("Default window width in pixels")
            .with_category(SettingsCategory::UserInterface),
        );

        // Window Height
        dialog.add_setting(
            Setting::new(
                "window_height",
                "Window Height",
                SettingValue::Integer(ui.window_height as i32),
            )
            .with_description("Default window height in pixels")
            .with_category(SettingsCategory::UserInterface),
        );

        // Show Toolbar
        dialog.add_setting(
            Setting::new(
                "show_toolbar",
                "Show Toolbar",
                SettingValue::Boolean(ui.panel_visibility.get("toolbar").copied().unwrap_or(true)),
            )
            .with_description("Show the main toolbar")
            .with_category(SettingsCategory::UserInterface),
        );

        // Show Status Bar
        dialog.add_setting(
            Setting::new(
                "show_status_bar",
                "Show Status Bar",
                SettingValue::Boolean(
                    ui.panel_visibility
                        .get("status_bar")
                        .copied()
                        .unwrap_or(true),
                ),
            )
            .with_description("Show the status bar at the bottom")
            .with_category(SettingsCategory::UserInterface),
        );

        // Show Menu Shortcuts
        dialog.add_setting(
            Setting::new(
                "show_menu_shortcuts",
                "Show Menu Shortcuts",
                SettingValue::Boolean(ui.show_menu_shortcuts),
            )
            .with_description("Display keyboard shortcuts in menu items")
            .with_category(SettingsCategory::UserInterface),
        );
    }

    /// Add file processing settings to dialog
    fn add_file_processing_settings(&self, dialog: &mut SettingsDialog) {
        let file = &self.config.file_processing;

        // Preserve Comments (inverted logic: preserve = not remove)
        dialog.add_setting(
            Setting::new(
                "preserve_comments",
                "Preserve Comments",
                SettingValue::Boolean(file.preserve_comments),
            )
            .with_description("Keep G-code comments during file processing")
            .with_category(SettingsCategory::FileProcessing),
        );

        // Arc Segment Length
        dialog.add_setting(
            Setting::new(
                "arc_segment_length",
                "Arc Segment Length (mm)",
                SettingValue::String(file.arc_segment_length.to_string()),
            )
            .with_description("Length of arc segments for arc-to-line expansion")
            .with_category(SettingsCategory::FileProcessing),
        );

        // Max Line Length
        dialog.add_setting(
            Setting::new(
                "max_line_length",
                "Max Line Length",
                SettingValue::Integer(file.max_line_length as i32),
            )
            .with_description("Maximum characters per line in output files")
            .with_category(SettingsCategory::FileProcessing),
        );
    }

    /// Add keyboard shortcuts to dialog
    fn add_keyboard_shortcuts(&self, dialog: &mut SettingsDialog) {
        // Define default keyboard shortcuts
        let shortcuts = vec![
            KeyboardShortcut::new("file_open", "Open File", "Ctrl+O"),
            KeyboardShortcut::new("file_save", "Save File", "Ctrl+S"),
            KeyboardShortcut::new("file_exit", "Exit Application", "Ctrl+Q"),
            KeyboardShortcut::new("edit_undo", "Undo", "Ctrl+Z"),
            KeyboardShortcut::new("edit_redo", "Redo", "Ctrl+Y"),
            KeyboardShortcut::new("edit_cut", "Cut", "Ctrl+X"),
            KeyboardShortcut::new("edit_copy", "Copy", "Ctrl+C"),
            KeyboardShortcut::new("edit_paste", "Paste", "Ctrl+V"),
            KeyboardShortcut::new("edit_preferences", "Preferences", "Ctrl+,"),
            KeyboardShortcut::new("machine_connect", "Connect", "Alt+C"),
            KeyboardShortcut::new("machine_disconnect", "Disconnect", "Alt+D"),
            KeyboardShortcut::new("machine_home", "Home Machine", "Ctrl+H"),
            KeyboardShortcut::new("machine_reset", "Reset", "F5"),
        ];

        for shortcut in shortcuts {
            dialog.add_shortcut(shortcut);
        }
    }

    /// Update General settings in config from dialog
    fn update_general_settings(&mut self, dialog: &SettingsDialog) -> Result<()> {
        if let Some(setting) = dialog.get_setting("measurement_system") {
            let system_str = setting.value.as_str();
            self.config.ui.measurement_system = match system_str.as_str() {
                "Imperial" => crate::config::MeasurementSystem::Imperial,
                _ => crate::config::MeasurementSystem::Metric,
            };
        }

        if let Some(setting) = dialog.get_setting("feed_rate_units") {
            let units_str = setting.value.as_str();
            self.config.ui.feed_rate_units = match units_str.as_str() {
                "mm/sec" => crate::config::FeedRateUnits::MmPerSec,
                "in/min" => crate::config::FeedRateUnits::InPerMin,
                "in/sec" => crate::config::FeedRateUnits::InPerSec,
                _ => crate::config::FeedRateUnits::MmPerMin,
            };
        }

        if let Some(setting) = dialog.get_setting("default_directory") {
            let path_str = setting.value.as_str();
            if !path_str.is_empty() {
                self.config.file_processing.output_directory = std::path::PathBuf::from(path_str);
            }
        }
        Ok(())
    }

    /// Update UI settings in config from dialog
    fn update_ui_settings(&mut self, dialog: &SettingsDialog) -> Result<()> {
        if let Some(setting) = dialog.get_setting("theme") {
            let theme_str = setting.value.as_str();
            self.config.ui.theme = match theme_str.as_str() {
                "Light" => crate::config::Theme::Light,
                "Dark" => crate::config::Theme::Dark,
                _ => crate::config::Theme::System,
            };
        }

        if let Some(setting) = dialog.get_setting("language") {
            let lang_str = setting.value.as_str();
            self.config.ui.language = match lang_str.as_str() {
                "English" => "en".to_string(),
                "French" => "fr".to_string(),
                "German" => "de".to_string(),
                "Spanish" => "es".to_string(),
                "Portuguese" => "pt".to_string(),
                "Italian" => "it".to_string(),
                _ => "system".to_string(),
            };
        }

        if let Some(setting) = dialog.get_setting("startup_tab") {
            let tab_str = setting.value.as_str();
            self.config.ui.startup_tab = match tab_str.as_str() {
                "Machine" => crate::config::StartupTab::Machine,
                "Console" => crate::config::StartupTab::Console,
                "Editor" => crate::config::StartupTab::Editor,
                "Visualizer" => crate::config::StartupTab::Visualizer,
                "CamTools" => crate::config::StartupTab::CamTools,
                "Designer" => crate::config::StartupTab::Designer,
                "DeviceInfo" => crate::config::StartupTab::DeviceInfo,
                "Config" => crate::config::StartupTab::Config,
                "Devices" => crate::config::StartupTab::Devices,
                "Tools" => crate::config::StartupTab::Tools,
                "Materials" => crate::config::StartupTab::Materials,
                _ => crate::config::StartupTab::Machine,
            };
        }

        if let Some(setting) = dialog.get_setting("window_width") {
            if let Ok(width) = setting.value.as_str().parse::<u32>() {
                self.config.ui.window_width = width;
            }
        }

        if let Some(setting) = dialog.get_setting("window_height") {
            if let Ok(height) = setting.value.as_str().parse::<u32>() {
                self.config.ui.window_height = height;
            }
        }

        if let Some(setting) = dialog.get_setting("show_toolbar") {
            if let Ok(value) = setting.value.as_str().parse::<bool>() {
                self.config
                    .ui
                    .panel_visibility
                    .insert("toolbar".to_string(), value);
            }
        }

        if let Some(setting) = dialog.get_setting("show_status_bar") {
            if let Ok(value) = setting.value.as_str().parse::<bool>() {
                self.config
                    .ui
                    .panel_visibility
                    .insert("status_bar".to_string(), value);
            }
        }

        if let Some(setting) = dialog.get_setting("show_menu_shortcuts") {
            if let Ok(value) = setting.value.as_str().parse::<bool>() {
                self.config.ui.show_menu_shortcuts = value;
            }
        }

        Ok(())
    }

    /// Update file processing settings in config from dialog
    fn update_file_processing_settings(&mut self, dialog: &SettingsDialog) -> Result<()> {
        if let Some(setting) = dialog.get_setting("preserve_comments") {
            if let Ok(value) = setting.value.as_str().parse::<bool>() {
                self.config.file_processing.preserve_comments = value;
            }
        }

        if let Some(setting) = dialog.get_setting("arc_segment_length") {
            if let Ok(value) = setting.value.as_str().parse::<f64>() {
                self.config.file_processing.arc_segment_length = value;
            }
        }

        if let Some(setting) = dialog.get_setting("max_line_length") {
            if let Ok(value) = setting.value.as_str().parse::<u32>() {
                self.config.file_processing.max_line_length = value;
            }
        }

        Ok(())
    }
}

impl Default for SettingsPersistence {
    fn default() -> Self {
        Self::new()
    }
}

