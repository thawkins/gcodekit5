//! # GCodeKit5 CAM Tools
//!
//! This crate provides specialized CAM (Computer-Aided Manufacturing) tools and
//! processing operations for generating G-Code for various CNC applications.
//!
//! ## CAM Tools Included
//!
//! - **Jigsaw Puzzle Maker**: Generate G-Code for cutting interlocking puzzle pieces
//! - **Tabbed Box Maker**: Create finger-jointed box designs with customizable parameters
//! - **Drill Press**: Specialized drilling cycles including peck drilling and helical interpolation
//! - **Laser Engraver**: Specialized processing for laser cutting and engraving
//! - **Arc Expander**: Arc interpolation and expansion
//! - **Probe Routines**: Touch probe operations for work coordinate setup
//! - **WCS Service**: Work coordinate system offset management from probe results
//!
//! ## Supporting Infrastructure
//!
//! - **Optimizer**: G-Code optimization and minimization
//! - **Validator**: G-Code validation and safety checks
//! - **Comment Processor**: G-Code comment stripping and word tokenizing
//! - **Job Preamble**: Optional homing preamble shared by the laser generators
//! - **CAM Presets**: Unit-aware saved parameter files for the bitmap and probe tools
//! - **Statistics**: G-Code statistics and analysis

pub mod arc_expander;
pub mod cam_presets;
pub mod comment_processor;
pub mod drill_press;
pub mod gerber;
pub mod hatch_generator;
pub mod jigsaw_puzzle;
pub mod job_preamble;
pub mod laser_engraver;
pub mod optimizer;
pub mod probe_routines;
pub mod speeds_feeds;
pub mod spoilboard_grid;
pub mod spoilboard_surfacing;
pub mod stats;
pub mod tabbed_box;
pub mod validator;
pub mod wcs_service;

// UI module

// Re-export commonly used items
pub use arc_expander::ArcExpander;
pub use comment_processor::CommentProcessor;
pub use drill_press::{DrillPressGenerator, DrillPressParameters};
pub use gerber::{GerberConverter, GerberLayerType, GerberParameters};
pub use jigsaw_puzzle::{JigsawPuzzleMaker, PuzzleParameters};
pub use laser_engraver::{
    BitmapImageEngraver, EngravingParameters, HalftoneMethod, ImageTransformations, RotationAngle,
    ScanDirection,
};
pub use optimizer::GCodeOptimizer;
pub use probe_routines::{
    ProbeCycle, ProbeOptions, ProbeRoutineEngine, ProbeRoutineOutput, defaults as probe_defaults,
};
pub use speeds_feeds::{
    CalculationInput, CalculationResult, DeviceLimits, DeviceRecommendations,
    FeedsSpeedsLookupTable, GeometryAdjustments, MaterialSpeedFactors, OperationType,
    SpeedsFeedsCalculator, SpeedsFeedsDataContext, SpeedsFeedsDeviceService,
    SpeedsFeedsMaterialService, SpeedsFeedsToolService, calculate_speeds_feeds,
};
pub use spoilboard_grid::{SpoilboardGridGenerator, SpoilboardGridParameters};
pub use spoilboard_surfacing::{SpoilboardSurfacingGenerator, SpoilboardSurfacingParameters};
pub use stats::StatsCalculator;
pub use tabbed_box::{
    BoxParameters, BoxType, FingerJointSettings, FingerStyle, KeyDividerType, TabbedBoxMaker,
};
pub use validator::GCodeValidator;
pub use wcs_service::{PersistentProbeResults, WcsUpdateConfig, WcsUpdateResult, WcsUpdateService};
