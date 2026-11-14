//! # GCodeKit5 Visualizer
//!
//! G-code visualization, parsing, and processing for GCodeKit5.
//! Includes parser, preprocessors, stream readers, and visualizer components.
#![deny(unsafe_code)]

pub mod gcode;
pub mod helpers;
pub mod visualizer;

pub use gcode::{
    CommandId, CommandLengthProcessor, CommandListener, CommandListenerHandle,
    CommandNumberGenerator, CommandProcessor, CommandResponse, CommandState, CommentProcessor,
    DecimalProcessor, EmptyLineRemoverProcessor, GcodeCommand, GcodeParser, GcodeState, ModalState,
    ProcessorConfig, ProcessorHandle, ProcessorPipeline, ProcessorRegistry, WhitespaceProcessor,
    stream::{FileStreamReader, GcodeStreamReader, PausableStream, StringStreamReader},
};
pub use visualizer::{
    Camera, Camera3D, GCodeCommand, HeightMap, Point3D, Renderer, Scene, StockSimulator3D,
    ToolpathSegment, ToolpathSegmentType, Visualizer, VisualizerControls, generate_surface_mesh,
    toolpath_from_commands,
};
