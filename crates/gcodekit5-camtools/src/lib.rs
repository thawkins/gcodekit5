//! # GCodeKit4 CAM Tools
//!
//! This crate provides specialized CAM (Computer-Aided Manufacturing) tools and
//! processing operations for generating G-Code for various CNC applications.
//!
//! ## CAM Tools Included
//!
//! - **Advanced Features**: Cutting feeds, threading, speeds, and advanced machining operations
//! - **Jigsaw Puzzle Maker**: Generate G-Code for cutting interlocking puzzle pieces
//! - **Tabbed Box Maker**: Create finger-jointed box designs with customizable parameters
//! - **Drill Press**: Specialized drilling cycles including peck drilling and helical interpolation
//! - **Laser Engraver**: Specialized processing for laser cutting and engraving
//! - **Vector Engraver**: Vector path cutting with advanced contour and fill options
//! - **Arc Expander**: Arc interpolation and expansion
//!
//! ## Supporting Infrastructure
//!
//! - **Core Infrastructure**: Application state, configuration, logging, and telemetry
//! - **Optimizer**: G-Code optimization and minimization
//! - **Validator**: G-Code validation and safety checks
//! - **Comment Processor**: G-Code comment handling
//! - **Statistics**: G-Code statistics and analysis
//!
//! ## UI Components
//!
//! - **Advanced Features Panel**: UI panel for controlling CAM tools

pub mod advanced_features;
pub mod arc_expander;
pub mod comment_processor;
pub mod core_infrastructure;
pub mod drill_press;
pub mod gerber;
pub mod hatch_generator;
mod hatch_test;
pub mod jigsaw_puzzle;
pub mod laser_engraver;
pub mod optimizer;
pub mod speeds_feeds;
pub mod spoilboard_grid;
pub mod spoilboard_surfacing;
pub mod stats;
pub mod tabbed_box;
pub mod validator;
pub mod vector_engraver;

// UI module
pub mod advanced_features_panel;

// Re-export commonly used items
pub use advanced_features::{
    CommandHistory, ProbingSystem, SimulationMode, SoftLimits, ToolLibrary, WorkCoordinateManager,
};
pub use arc_expander::ArcExpander;
pub use comment_processor::CommentProcessor;
pub use core_infrastructure::{AppConfig, ApplicationState, Logger, TelemetryData};
pub use drill_press::{DrillPressGenerator, DrillPressParameters};
pub use gerber::{GerberConverter, GerberLayerType, GerberParameters};
pub use jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
pub use laser_engraver::{
    BitmapImageEngraver, EngravingParameters, HalftoneMethod, ImageTransformations, RotationAngle,
    ScanDirection,
};
pub use optimizer::GCodeOptimizer;
pub use speeds_feeds::{CalculationResult, SpeedsFeedsCalculator};
pub use spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
pub use spoilboard_surfacing::{SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters};
pub use stats::StatsCalculator;
pub use tabbed_box::{
    BoxParameters, BoxType, FingerJointSettings, FingerStyle, KeyDividerType, TabbedBoxMaker,
};
pub use validator::GCodeValidator;
pub use vector_engraver::{VectorEngraver, VectorEngravingParameters};
