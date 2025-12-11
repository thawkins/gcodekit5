//! # GCodeKit4 Designer
//!
//! This crate provides visual design and CAM layout tools for creating complex
//! toolpaths. It combines shape design, CAM operations, and visualization into
//! an integrated design environment.
//!
//! ## Core Components
//!
//! ### Design Elements
//! - **Shapes**: Rectangles, circles, polylines, text, and custom paths
//! - **Templates**: Pre-built designs (boxes, puzzles, engravings, etc.)
//! - **Canvas**: Drawing surface with coordinate systems and transformations
//! - **Viewport**: Camera control and zoom for navigation
//!
//! ### CAM Operations Integration
//! - **Pocket Operations**: Hollow out areas with tool compensation
//! - **Drilling Patterns**: Generate hole drilling sequences
//! - **Multipass**: Cut thick materials in multiple depths
//! - **Adaptive**: Optimize toolpath load for better cutting
//! - **V-Carving**: Advanced angle-based cutting
//! - **Arrays**: Create repetitive patterns
//! - **Parametric**: Generate designs from parameters
//!
//! ### Advanced Features
//! - **History/Undo-Redo**: Full operation history management
//! - **Spatial Indexing**: Efficient geometry queries
//! - **Toolpath Simulation**: Visualize cutting operations
//! - **Import/Export**: DXF, SVG, and design serialization
//! - **Rendering**: 2D visualization with optimization
//!
//! ## Architecture
//!
//! The designer operates in layers:
//!
//! ```text
//! Canvas (Drawing surface)
//!   ├── Shapes (Rectangles, circles, paths)
//!   ├── Viewport (Camera/zoom)
//!   └── Renderer (Visualization)
//!
//! Operations (Pocket, drilling, multipass, adaptive, etc.)
//!   └── Spatial Index (Efficient geometry storage)
//!
//! Toolpath (Final G-code path)
//!   └── Simulation (Preview)
//!
//! History (Undo/redo)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gcodekit5_designer::Canvas;
//!
//! // Create a new canvas
//! let mut canvas = Canvas::new(100.0, 100.0);
//!
//! // Add shapes
//! canvas.add_rectangle(10.0, 10.0, 30.0, 30.0);
//!
//! // Apply operations
//! let toolpath = canvas.generate_toolpath()?;
//! ```

pub mod adaptive;
pub mod arrays;
pub mod canvas;
pub mod commands;
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
pub mod font_manager;
pub mod helpers;
pub mod shapes;
pub mod shape_store;
pub mod selection_manager;
pub mod spatial_manager;
pub mod spatial_index;
pub mod svg_renderer;
pub mod templates;
pub mod tool_library;
pub mod stock_removal;
pub mod toolpath;
pub mod toolpath_simulation;
pub mod vcarve;
pub mod viewport;

// Integration modules
pub mod designer_state;
pub mod designer_editor_integration;
pub mod designer_visualizer_integration;
pub mod gcode_converter;

// Re-export all public types from submodules
pub use adaptive::{AdaptiveClearing, AdaptiveAlgorithm, MaterialProperties, MaterialType, LoadMonitor, DynamicStepover};
pub use arrays::{ArrayType, ArrayGenerator, ArrayOperation, LinearArrayParams, CircularArrayParams, GridArrayParams};
pub use canvas::{Canvas, CanvasPoint, DrawingMode};
pub use commands::DesignerCommand;
pub use drilling_patterns::*;
pub use dxf_parser::{DxfParser, DxfFile, DxfEntity, DxfHeader};
pub use gcode_gen::ToolpathToGcode;
pub use history::{ActionType, HistoryAction, HistoryTransaction, UndoRedoManager};
pub use import::{DxfImporter, FileFormat, ImportedDesign, SvgImporter};
pub use multipass::{DepthStrategy, MultiPassConfig, MultiPassToolpathGenerator};
pub use parametric::ParametricGenerator;
pub use pocket_operations::{Island, PocketGenerator, PocketOperation};
pub use render_optimizer::{RenderOptimizer, RenderStats};
pub use shapes::{Point, Circle, Rectangle, Line, Ellipse, Shape, ShapeType};
pub use spatial_index::{Bounds, SpatialIndex, SpatialIndexStats};
pub use stock_removal::{HeightMap2D, SimulationResult, StockMaterial};
pub use templates::*;
pub use tool_library::{CoolantType, MaterialProfile, Tool, ToolLibrary, ToolType};
pub use toolpath::{Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType};
pub use toolpath_simulation::{SimulationState, ToolPosition, ToolpathAnalyzer, ToolpathSimulator};
pub use vcarve::VCarveGenerator;
pub use viewport::Viewport;

// State and integration
pub use designer_state::DesignerState;
pub use gcode_converter::{create_linear_segment, create_arc_segment, point_to_2d};
