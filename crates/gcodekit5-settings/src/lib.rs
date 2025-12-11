//! GCodeKit4 Settings Crate
//!
//! Handles application configuration, settings persistence, and UI view models.

pub mod config;
pub mod controller;
pub mod manager;
pub mod persistence;
pub mod view_model;

pub use config::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, Theme, UiSettings,
};
pub use controller::{SettingUiModel, SettingsController};
pub use manager::SettingsManager;
pub use persistence::SettingsPersistence;
pub use view_model::{KeyboardShortcut, Setting, SettingValue, SettingsCategory, SettingsDialog};
