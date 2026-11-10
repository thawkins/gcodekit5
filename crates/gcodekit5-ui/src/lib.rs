//! # GCodeKit5 UI
//!
//! GTK-based user interface for GCodeKit5.

pub mod device_status;
pub mod glib_log_filter;
pub mod gtk_app;
pub mod i18n;
pub mod types;
pub mod ui;

// Re-export settings for convenience if needed
pub use gcodekit5_settings::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, MachineSettings,
    SettingsManager, UiSettings,
};

pub use crate::types::{
    BitmapEngravingParams, JigsawPuzzleParams, TabbedBoxParams, VectorEngravingParams,
};
