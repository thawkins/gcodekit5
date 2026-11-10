//! # GCodeKit5 Designer
//!
//! Shape design and CAM layout for the Designer tab: shapes on a canvas,
//! CAM operations on them, and conversion to G-code.
//!
//! ## Core Components
//!
//! - **Shapes and canvas**: rectangles, circles, lines, polygons, text,
//!   gears, sprockets and paths on a `Canvas`, with a `Viewport` for zoom
//!   and pan
//! - **CAM operations**: profile and pocket toolpaths (`ToolpathGenerator`,
//!   `PocketGenerator`), offset, fillet and chamfer (`ops`), and arrays
//! - **G-code**: `ToolpathToGcode` turns toolpaths into a program
//! - **State**: `DesignerState` holds the design, selection and undo/redo
//! - **Import/export**: DXF, SVG and STL import, and design serialization
//! - **Spatial indexing**: efficient geometry queries
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
#![deny(unsafe_code)]

pub mod arrays;
pub mod canvas;
pub mod commands;
pub mod dxf_parser;
pub mod font_manager;
pub mod gcode_gen;
pub mod import;
pub mod model;
pub mod model3d;
pub mod ops;
pub mod parametric_shapes;
pub mod pocket_operations;
pub mod selection_manager;
pub mod serialization;
pub mod shape_store;
pub mod shapes;
pub mod spatial_index;
pub mod spatial_manager;
pub mod stock_removal;
pub mod tool_library;
pub mod toolpath;
pub mod viewport;

// Integration modules
pub mod designer_state;
pub mod engraving;
pub mod image_importer;

// Re-export all public types from submodules
pub use arrays::{
    ArrayGenerator, ArrayOperation, ArrayType, CircularArrayParams, GridArrayParams,
    LinearArrayParams,
};
pub use canvas::{Canvas, CanvasPoint, DrawingMode};
pub use commands::DesignerCommand;
pub use dxf_parser::{DxfEntity, DxfFile, DxfHeader, DxfParser};
pub use gcode_gen::ToolpathToGcode;
pub use import::{DxfImporter, FileFormat, ImportedDesign, StlImporter, SvgImporter};
pub use model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignRectangle as Rectangle, DesignText as TextShape, Point, Shape, ShapeType,
};
pub use model3d::{Mesh3D, Model3DFormat, Model3DImporter, ProjectionParams, Triangle3D};
pub use pocket_operations::{Island, PocketGenerator, PocketOperation};
pub use spatial_index::{Bounds, SpatialIndex, SpatialIndexStats};
pub use stock_removal::StockMaterial;
pub use tool_library::{CoolantType, MaterialProfile, Tool, ToolLibrary, ToolType};
pub use toolpath::{Toolpath, ToolpathGenerator, ToolpathSegment, ToolpathSegmentType};
pub use viewport::Viewport;

// State and integration
pub use designer_state::DesignerState;
