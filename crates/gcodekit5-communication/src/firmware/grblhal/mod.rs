//! grblHAL Firmware Support
//!
//! This module provides support for grblHAL (enhanced GRBL with additional features).
//! grblHAL is a high-performance fork of GRBL with support for more advanced features
//! including network connectivity, additional axes, and enhanced plugin support.

pub mod capabilities;

pub use capabilities::GrblHalCapabilities;

/// grblHAL version information
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GrblHalVersion {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Patch version
    pub patch: u32,
    /// Build identifier
    pub build: Option<String>,
}

impl std::fmt::Display for GrblHalVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref build) = self.build {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, build)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}
