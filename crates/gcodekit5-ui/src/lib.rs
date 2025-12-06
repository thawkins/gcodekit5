//! # GCodeKit4 UI
//!
//! GTK-based user interface for GCodeKit4.

pub mod gtk_app;
pub mod ui;

// Re-export settings for convenience if needed
pub use gcodekit5_settings::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, SettingsManager, UiSettings,
};

pub use gcodekit5_gcodeeditor::{
    EditorState, TextBuffer, TextChange, TextLine, UndoManager,
    Viewport,
};
