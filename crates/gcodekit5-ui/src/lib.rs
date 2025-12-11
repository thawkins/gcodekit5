//! # GCodeKit4 UI
//!
//! GTK-based user interface for GCodeKit4.

pub mod gtk_app;
pub mod ui;
pub mod editor;
pub mod device_status;
pub mod helpers;
pub mod types;
pub mod i18n;

// Re-export settings for convenience if needed
pub use gcodekit5_settings::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, SettingsManager, UiSettings,
};

pub use gcodekit5_gcodeeditor::{
    EditorState, TextBuffer, TextChange, TextLine, UndoManager,
    Viewport,
};

pub use crate::types::{
    VectorEngravingParams, BitmapEngravingParams, TabbedBoxParams, JigsawPuzzleParams,
};

// Re-export EditorBridge so UI and examples can continue importing from gcodekit5_ui
// Note: gcodeeditor exports a non-UI EditorBridge backend as `EditorBridgeBackend`; UI exposes a separate Slint `EditorBridge`.
// Re-export the UI's Slint EditorBridge at the crate root so existing imports keep working.
pub use crate::editor::EditorBridge;
pub use crate::editor::SlintTextLine as TextLineUi;
