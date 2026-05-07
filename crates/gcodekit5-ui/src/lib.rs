//! # GCodeKit4 UI
//!
//! GTK-based user interface for GCodeKit4.

pub mod device_status;
pub mod editor;
pub mod gtk_app;
pub mod helpers;
pub mod i18n;
// pub mod platform; // Moved to ui::gtk::platform
pub mod types;
pub mod ui;

// Re-export settings for convenience if needed
pub use gcodekit5_settings::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, SettingsManager, UiSettings,
};

pub use gcodekit5_gcodeeditor::{
    EditorBridgeBackend, EditorError, EditorResult, TextBuffer, TextChange, UndoManager, Viewport,
};

pub use crate::types::{
    BitmapEngravingParams, JigsawPuzzleParams, TabbedBoxParams, VectorEngravingParams,
};

// Re-export EditorBridge so UI and examples can continue importing from gcodekit5_ui
// Note: gcodeeditor exports a non-UI EditorBridge backend as `EditorBridgeBackend`; UI exposes a separate Slint `EditorBridge`.
// Re-export the UI's Slint EditorBridge at the crate root so existing imports keep working.
pub use crate::editor::EditorBridge;
pub use crate::editor::SlintTextLine as TextLineUi;
