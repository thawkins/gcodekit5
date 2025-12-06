//! # GCodeKit4
//!
//! A Rust-based Universal G-Code Sender for CNC machines with support for:
//! - GRBL, TinyG, g2core, Smoothieware, FluidNC controllers
//! - Serial (USB), TCP/IP, and WebSocket connectivity
//! - 14 G-Code preprocessors for advanced toolpath processing
//! - Real-time 3D visualization and interactive UI
//!
//! ## Architecture
//!
//! GCodeKit4 is organized as a workspace with multiple crates:
//!
//! 1. **gcodekit5-core** - Core types, traits, state management, events
//! 2. **gcodekit5-parser** - G-code parsing, preprocessing, utilities
//! 3. **gcodekit5-communication** - Serial, TCP, WebSocket, firmware protocols
//! 4. **gcodekit5-ui** - Slint UI, visualizer, settings, editor
//! 5. **gcodekit5** - Main binary that integrates all crates
//!
//! ## Features
//!
//! - **Multi-Controller Support**: GRBL, TinyG, g2core, Smoothieware, FluidNC
//! - **Connection Protocols**: Serial/USB, TCP/IP, WebSocket
//! - **G-Code Processing**: Full parser with 14 preprocessors (arc expansion, mesh leveling, etc.)
//! - **Real-time Control**: Jogging, homing, probing, work coordinate systems
//! - **Advanced Features**: Macros, simulation mode, performance monitoring
//! - **Professional UI**: Syntax-highlighted editor, 3D visualization, console
//! - **Cross-Platform**: Linux, Windows, macOS support

#![allow(dead_code)]

// pub mod platform;

// Re-export modules for main.rs
pub use gcodekit5_communication::firmware;
pub use gcodekit5_core::data;
pub use gcodekit5_designer as designer;
pub use gcodekit5_ui::ui;
pub use gcodekit5_visualizer::visualizer;

pub use gcodekit5_core::{
    CNCPoint, CommunicatorState, ConnectionError, ControllerError, ControllerEvent,
    ControllerListener, ControllerListenerHandle, ControllerState, ControllerStatus,
    ControllerTrait, Error, EventDispatcher, FirmwareError, GcodeError, MachineStatus,
    MachineStatusSnapshot, Message, MessageDispatcher, MessageLevel, OverrideState,
    PartialPosition, Position, Result, SimpleController, Units,
};

pub use gcodekit5_visualizer::{
    AdvancedProber, Alarm, AlarmManager, AlarmType, AutoConnectConfig, BackupEntry, BackupManager,
    BasicProber, Bookmark, BookmarkManager, CommandHistory, CommandId, CommandLengthProcessor,
    CommandListener, CommandListenerHandle, CommandNumberGenerator, CommandProcessor,
    CommandResponse, CommandState, CommentProcessor, CustomAction, CustomMacro, DataLogger,
    DecimalProcessor, DropEvent, DropFileType, DropIndicatorState, DropTarget, DropZone,
    EmptyLineRemoverProcessor, ExportOptions, FeedRateStats, FileComparison, FileEncoding,
    FileExporter, FileFormat, FileProcessingPipeline, FileReadStats, FileStatistics,
    FileStreamReader, FileValidation, GcodeCommand, GcodeFileReader, GcodeParser, GcodeState,
    GcodeStreamReader, GcodeTemplate, HeightPoint, HistoryEntry, LogEntry, ModalState,
    NetworkConfig, PausableStream, PendantButton, PendantConfig, PerformanceMetrics, ProbeMesh,
    ProbePoint, ProcessedFile, ProcessorConfig, ProcessorHandle, ProcessorPipeline,
    ProcessorRegistry, ProgramState, RecentFileEntry, RecentFilesManager, SimulationPosition,
    Simulator, SoftLimits, SpindleStats, Stepper, StringStreamReader, TemplateLibrary,
    TemplateVariable, ToolInfo, ToolLibrary, ToolOffset, ToolOffsetManager, ValidationIssue,
    ValidationResult, ValidationSeverity, WhitespaceProcessor, WorkCoordinateSystem, WorkOffset,
};

pub use gcodekit5_designer::{
    Canvas, CanvasPoint, Circle, DesignerState, DrawingMode, Line, Point, Rectangle, Shape,
    ShapeType, Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType, ToolpathToGcode,
};

pub use gcodekit5_camtools::{
    BoxParameters, BoxType, FingerJointSettings, FingerStyle, JigsawPuzzleMaker, PuzzleParameters,
    TabbedBoxMaker, KeyDividerType, SpeedsFeedsCalculator, CalculationResult,
    SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters,
    SpoilboardGridGenerator, SpoilboardGridParameters,
};

pub use gcodekit5_communication::{
    list_ports, CapabilityManager, CapabilityState, Communicator, CommunicatorEvent,
    CommunicatorListener, CommunicatorListenerHandle, ConnectionDriver, ConnectionParams,
    ControllerType, FirmwareDetector, NoOpCommunicator, SerialCommunicator, SerialParity,
    SerialPortInfo, TcpCommunicator, TcpConnectionInfo,
};

pub use gcodekit5_ui::{
    Config, ConnectionSettings, ConnectionType, FileProcessingSettings, FirmwareSettings,
    MachineSettings, SettingsManager, UiSettings,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build date (set at compile time)
pub const BUILD_DATE: &str = env!("BUILD_DATE");

/// Initialize logging with the default configuration
///
/// Sets up structured logging with:
/// - Console output with pretty formatting
/// - RUST_LOG environment variable support
/// - UTF timestamps
pub fn init_logging() -> anyhow::Result<()> {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into());

    // In Windows GUI mode (release builds), log to a file instead of stdout
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use std::fs::OpenOptions;
        
        let log_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        
        let log_file = log_dir.join("gcodekit5.log");
        
        // Try to open log file, but if it fails, just disable logging rather than crash
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
        {
            Ok(file) => {
                let fmt_layer = fmt::layer()
                    .with_writer(file)
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_line_number(true)
                    .pretty();

                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt_layer)
                    .init();
            }
            Err(_) => {
                // If file creation fails, just use a no-op subscriber
                tracing_subscriber::registry()
                    .with(env_filter)
                    .init();
            }
        }
    }

    // In debug mode or non-Windows, log to stdout as before
    #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
    {
        let fmt_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_target(true)
            .with_level(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_line_number(true)
            .pretty();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}
