//! Speeds and Feeds Calculator Module
//!
//! Provides comprehensive feeds and speeds calculation with database integration:
//! - **Calculator**: Advanced calculation engine with formulas and adjustments
//! - **Services**: Database integration layers for Tools, Materials, and Devices
//!
//! ## Usage Example
//!
//! ```rust
//! use gcodekit5_camtools::speeds_feeds::{
//!     CalculationInput, CalculationResult, OperationType, SpeedsFeedsCalculator,
//!     SpeedsFeedsDataContext,
//! };
//! use gcodekit5_core::data::materials::Material;
//! use gcodekit5_core::data::tools::Tool;
//! use gcodekit5_devicedb::model::DeviceProfile;
//!
//! fn slotting_speeds(
//!     selected_material: Material,
//!     selected_tool: Tool,
//!     device_profile: DeviceProfile,
//! ) -> CalculationResult {
//!     // Data context giving access to the tool, material and device databases
//!     let _context = SpeedsFeedsDataContext::new(Some(device_profile.clone()));
//!
//!     // Build calculation input
//!     let input = CalculationInput {
//!         material: selected_material,
//!         tool: selected_tool,
//!         device: device_profile,
//!         operation: OperationType::Slotting,
//!         depth_of_cut: 3.0,
//!         width_of_cut: None,
//!         tool_stick_out: 25.0,
//!         coolant_enabled: true,
//!         conservative: false,
//!     };
//!
//!     // Calculate RPM, feed rate and chip load
//!     SpeedsFeedsCalculator::calculate(&input)
//! }
//! ```

// Module declarations
mod calculator;
mod service;

// Re-export calculator types
pub use calculator::{
    CalculationInput, CalculationResult, GeometryAdjustments, MaterialSpeedFactors, OperationType,
    SpeedsFeedsCalculator, calculate_speeds_feeds,
};

// Re-export service types
pub use service::{
    DeviceLimits, DeviceRecommendations, FeedsSpeedsLookupTable, SpeedsFeedsDataContext,
    SpeedsFeedsDeviceService, SpeedsFeedsMaterialService, SpeedsFeedsToolService,
};
