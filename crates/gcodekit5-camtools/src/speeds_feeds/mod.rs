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
//!     SpeedsFeedsCalculator, CalculationInput, OperationType,
//!     SpeedsFeedsDataContext
//! };
//!
//! // Create data context with device
//! let context = SpeedsFeedsDataContext::new(Some(device_profile));
//!
//! // Build calculation input
//! let input = CalculationInput {
//!     material: selected_material,
//!     tool: selected_tool,
//!     device: device_profile,
//!     operation: OperationType::Slotting,
//!     depth_of_cut: 3.0,
//!     width_of_cut: None,
//!     tool_stick_out: 25.0,
//!     coolant_enabled: true,
//!     conservative: false,
//! };
//!
//! // Calculate
//! let result = SpeedsFeedsCalculator::calculate(&input);
//! println!("RPM: {}, Feed: {}", result.rpm, result.feed_rate);
//! ```

// Module declarations
mod calculator;
mod service;

// Re-export calculator types
pub use calculator::{
    calculate_speeds_feeds, CalculationInput, CalculationResult, GeometryAdjustments,
    MaterialSpeedFactors, OperationType, SpeedsFeedsCalculator,
};

// Re-export service types
pub use service::{
    DeviceLimits, DeviceRecommendations, FeedsSpeedsLookupTable, SpeedsFeedsDataContext,
    SpeedsFeedsDeviceService, SpeedsFeedsMaterialService, SpeedsFeedsToolService,
};
