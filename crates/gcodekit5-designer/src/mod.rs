//! # Designer Module
//!
//! Provides a 2D CAD/CAM design tool for creating CNC projects and generating G-code.
//!
//! The designer module includes:
//! - Shape drawing and manipulation
//! - Canvas with zoom/pan controls with proper coordinate mapping
//! - Viewport management for pixel-to-world coordinate conversion
//! - Toolpath generation from design shapes
//! - CAM operations: pocket milling, drilling patterns, multi-pass depth control
//! - Tool library management
//! - Toolpath simulation and visualization
//! - File import support (SVG, DXF)
//! - Array operations (linear, circular, grid pattern copies)
//! - V-carving toolpath generation for V-bit tools
//! - Adaptive clearing for optimized material removal
//! - DXF file parsing and entity extraction
//! - G-code export to the G-Code Editor
//!
//! This module is organized into sub-modules:
//! - `shapes` - Geometric shape definitions and operations
//! - `canvas` - Canvas state and drawing operations
//! - `viewport` - Coordinate transformation and viewport management
//! - `toolpath` - Toolpath generation from shapes
//! - `tool_library` - Tool definitions and management
//! - `pocket_operations` - Pocket milling with island detection
//! - `drilling_patterns` - Drilling pattern generation
//! - `multipass` - Multi-pass depth control and ramping
//! - `toolpath_simulation` - Toolpath preview and analysis
//! - `import` - SVG and DXF file import
//! - `arrays` - Linear, circular, and grid array operations
//! - `vcarve` - V-carving toolpath generation for V-bit tools
//! - `adaptive` - Adaptive clearing strategy for load optimization
//! - `dxf_parser` - DXF file parsing and entity extraction
//! - `parametric` - Parametric design system for templates
//! - `gcode_gen` - G-code generation from toolpaths
//! - `templates` - Design template management system for saving and organizing designs
//! - `history` - Undo/redo functionality for all design operations

pub mod adaptive;
pub mod arrays;
pub mod canvas;
pub mod drilling_patterns;
pub mod dxf_parser;
pub mod gcode_gen;
pub mod history;
pub mod import;
pub mod multipass;
pub mod parametric;
pub mod pocket_operations;
pub mod render_optimizer;
pub mod renderer;
pub mod serialization;
pub mod shapes;
pub mod spatial_index;
pub mod svg_renderer;
pub mod templates;
pub mod tool_library;
pub mod toolpath;
pub mod toolpath_simulation;
pub mod vcarve;
pub mod viewport;

pub use adaptive::{
    AdaptiveAlgorithm, AdaptiveClearing, DynamicStepover, LoadMonitor, MaterialProperties,
    MaterialType,
};
pub use arrays::{
    ArrayGenerator, ArrayOperation, ArrayType, CircularArrayParams, GridArrayParams,
    LinearArrayParams,
};
pub use canvas::{Canvas, CanvasPoint, DrawingMode};
pub use drilling_patterns::{
    DrillOperation, DrillingPattern, DrillingPatternGenerator, PatternType,
};
pub use dxf_parser::{
    DxfArc, DxfCircle, DxfEntity, DxfEntityType, DxfFile, DxfHeader, DxfLine, DxfParser,
    DxfPolyline, DxfText, DxfUnit,
};
pub use gcode_gen::ToolpathToGcode;
pub use history::{ActionType, HistoryAction, HistoryTransaction, UndoRedoManager};
pub use import::{DxfImporter, FileFormat, ImportedDesign, SvgImporter};
pub use multipass::{DepthStrategy, MultiPassConfig, MultiPassToolpathGenerator};
pub use parametric::{
    Parameter, ParameterConstraint, ParameterSet, ParameterType, ParametricGenerator,
    ParametricTemplate, TemplateLibrary,
};
pub use pocket_operations::{Island, PocketGenerator, PocketOperation};
pub use render_optimizer::{RenderOptimizer, RenderStats};
pub use shapes::{
    Circle, Ellipse, Line, Point, Polygon, Rectangle, RoundRectangle, Shape, ShapeType,
};
pub use spatial_index::{Bounds, SpatialIndex, SpatialIndexStats};
pub use templates::{
    DesignTemplate, DesignTemplateLibrary, TemplateCategory, TemplateManager, TemplatePersistence,
};
pub use tool_library::{CoolantType, MaterialProfile, Tool, ToolLibrary, ToolType};
pub use toolpath::{Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType};
pub use toolpath_simulation::{SimulationState, ToolPosition, ToolpathAnalyzer, ToolpathSimulator};
pub use vcarve::{VBitTool, VCarveGenerator, VCarveParams, VCarveSegment};
pub use viewport::Viewport;
