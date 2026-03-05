//! # Device Database Traits
//!
//! Trait definitions for device database operations, enabling
//! different storage backends and testability via trait objects.

use crate::model::DeviceProfile;

pub trait DeviceProfileProvider: Send + Sync {
    fn get_active_profile(&self) -> Option<DeviceProfile>;
    fn get_profile(&self, id: &str) -> Option<DeviceProfile>;
}
