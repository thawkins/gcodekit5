//! # GCodeKit4 Visualizer
//!
//! G-code visualization, parsing, and processing for GCodeKit4.
//! Includes parser, preprocessors, stream readers, and visualizer components.

pub mod gcode;
pub mod helpers;
pub mod utils;
pub mod visualizer;

pub use visualizer::{
    generate_surface_mesh, render_g1_to_path, render_g2_to_path, render_g3_to_path,
    render_g4_to_path, render_grid_to_path, render_intensity_overlay, render_origin_to_path,
    render_rapid_moves_to_path, render_toolpath_to_path, Camera, Camera3D, GCodeCommand, Point3D,
    Renderer, Scene, StockSimulator3D, ToolpathSegment, ToolpathSegmentType, Visualizer,
    VisualizerControls, VoxelGrid,
};

pub use gcode::{
    stream::{FileStreamReader, GcodeStreamReader, PausableStream, StringStreamReader},
    CommandId, CommandLengthProcessor, CommandListener, CommandListenerHandle,
    CommandNumberGenerator, CommandProcessor, CommandResponse, CommandState, CommentProcessor,
    DecimalProcessor, EmptyLineRemoverProcessor, GcodeCommand, GcodeParser, GcodeState, ModalState,
    ProcessorConfig, ProcessorHandle, ProcessorPipeline, ProcessorRegistry, WhitespaceProcessor,
};

pub use utils::{
    AdvancedProber, Alarm, AlarmManager, AlarmType, AutoConnectConfig, BackupEntry, BackupManager,
    BasicProber, Bookmark, BookmarkManager, CommandHistory, CustomAction, CustomMacro, DataLogger,
    DropEvent, DropFileType, DropIndicatorState, DropTarget, DropZone, ExportOptions,
    FeedRateStats, FileComparison, FileEncoding, FileExporter, FileFormat, FileProcessingPipeline,
    FileReadStats, FileStatistics, FileValidation, GcodeFileReader, GcodeTemplate, HeightPoint,
    HistoryEntry, LogEntry, NetworkConfig, PendantButton, PendantConfig, PerformanceMetrics,
    ProbeMesh, ProbePoint, ProcessedFile, ProgramState, RecentFileEntry, RecentFilesManager,
    SimulationPosition, Simulator, SoftLimits, SpindleStats, Stepper, TemplateLibrary,
    TemplateVariable, ToolInfo, ToolLibrary, ToolOffset, ToolOffsetManager, ValidationIssue,
    ValidationResult, ValidationSeverity, WorkCoordinateSystem, WorkOffset,
};
