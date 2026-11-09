//! # Device Database
//!
//! Manages CNC device profiles including machine capabilities,
//! work area dimensions, firmware type, and connection parameters.
//! Provides CRUD operations and persistence for the device library.

pub mod error;
pub mod manager;
pub mod model;
pub mod templates;
pub mod traits;
pub mod ui_integration;

pub use error::{DeviceError, DeviceResult, ProfileError, ProfileResult};
pub use manager::DeviceManager;
pub use model::{AxisLimits, ControllerType, DeviceProfile, DeviceType};
pub use templates::{get_all_templates, get_templates_by_axis_count};
pub use traits::DeviceProfileProvider;
pub use ui_integration::{DeviceProfileUiModel, DeviceUiController};
