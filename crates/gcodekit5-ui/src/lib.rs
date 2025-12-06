//! # GCodeKit4 UI
//!
//! Slint-based user interface for GCodeKit4.
//! Provides UI panels, visualizer, settings, and editor components.

pub mod testing;
pub mod ui;
pub mod editor_bridge;
pub mod gtk_app;

pub use ui::{
    ConsoleEvent, ConsoleListener, DeviceConsoleManager, DeviceMessageType, FirmwareSettingsIntegration,
    GcodeEditor, GcodeLine, KeyboardShortcut, Setting, SettingUiModel, SettingValue,
    SettingsCategory, SettingsController, SettingsDialog, SettingsPersistence, Token, TokenType,
};

pub use gcodekit5_settings::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, SettingsManager, UiSettings,
};

pub use editor_bridge::{EditorBridge, SlintTextLine};

pub use gcodekit5_gcodeeditor::{
    EditorState, TextBuffer, TextChange, TextLine, UndoManager,
    Viewport,
};
