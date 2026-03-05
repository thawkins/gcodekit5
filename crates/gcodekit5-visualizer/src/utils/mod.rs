//! Utility functions and helpers

pub mod advanced;
pub mod export;
pub mod file_io;
pub mod phase6_extended;
pub mod phase7;
pub mod processing;

pub use advanced::{
    AdvancedProber, BackupEntry, BackupManager, BasicProber, FileComparison, GcodeTemplate,
    ProbePoint, TemplateLibrary, TemplateVariable, ValidationIssue, ValidationResult,
    ValidationSeverity,
};
pub use export::{
    DropEvent, DropFileType, DropIndicatorState, DropTarget, DropZone, ExportOptions, FileExporter,
    FileFormat,
};
pub use file_io::{
    FileEncoding, FileReadStats, FileValidation, GcodeFileReader, RecentFileEntry,
    RecentFilesManager,
};
pub use phase6_extended::{
    Alarm, AlarmManager, AlarmType, AutoConnectConfig, Bookmark, BookmarkManager, CommandHistory,
    CustomAction, CustomMacro, DataLogger, HeightPoint, HistoryEntry, LogEntry, NetworkConfig,
    PendantButton, PendantConfig, PerformanceMetrics, ProbeMesh, ProgramState, SimulationPosition,
    Simulator, SoftLimits, Stepper, ToolInfo, ToolLibrary, ToolOffset, ToolOffsetManager,
    WorkCoordinateSystem, WorkOffset,
};
pub use phase7::{
    BufferDiagnostics, CalibrationResult, CalibrationStep, CalibrationStepType, CalibrationWizard,
    CommunicationDiagnostics, DiagnosticReport, EmergencyStopManager, EmergencyStopState,
    ExportFormat, FeedHoldManager, FormatExporter, MotionInterlock, PerformanceProfiler, Plugin,
    PluginConfig, PluginError, PluginMetadata, PluginRegistry, PostProcessor, SafetyError,
    SafetyFeaturesManager,
};
pub use processing::{
    BoundingBox, FeedRateStats, FileProcessingPipeline, FileStatistics, ProcessedFile, SpindleStats,
};

/// Format a float to a reasonable number of decimal places
pub fn format_float(value: f64, precision: usize) -> String {
    format!("{:.prec$}", value, prec = precision)
}

/// Convert degrees to radians
pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

/// Convert radians to degrees
pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * 180.0 / std::f64::consts::PI
}
