//! # GCodeKit5 Visualizer
//!
//! G-code visualization, parsing, and processing for GCodeKit5.
//! Includes parser, preprocessors, stream readers, and visualizer components.

pub mod gcode;
pub mod helpers;
pub mod visualizer;

pub use gcode::{
    stream::{FileStreamReader, GcodeStreamReader, PausableStream, StringStreamReader},
    CommandId, CommandLengthProcessor, CommandListener, CommandListenerHandle,
    CommandNumberGenerator, CommandProcessor, CommandResponse, CommandState, CommentProcessor,
    DecimalProcessor, EmptyLineRemoverProcessor, GcodeCommand, GcodeParser, GcodeState, ModalState,
    ProcessorConfig, ProcessorHandle, ProcessorPipeline, ProcessorRegistry, WhitespaceProcessor,
};
pub use visualizer::{
    generate_surface_mesh, Camera, Camera3D, GCodeCommand, Point3D, Renderer, Scene,
    StockSimulator3D, ToolpathSegment, ToolpathSegmentType, Visualizer, VisualizerControls,
    VoxelGrid,
};
