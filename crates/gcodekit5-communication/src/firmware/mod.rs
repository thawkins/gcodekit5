//! Firmware implementations for various CNC controllers
//!
//! Supported controllers:
//! - GRBL: Open-source CNC control software
//! - TinyG: CNC control for 3D printers and engravers
//! - g2core: Next generation of TinyG
//! - Smoothieware: CNC control software
//! - FluidNC: Powerful open-source CNC control

pub mod capabilities;
pub mod capabilities_db;
pub mod capability_manager;
pub mod connection_watch;
pub mod device_db;
pub mod device_status;
pub mod file_service;
pub mod firmware_detector;
pub mod firmware_version;
pub mod fluidnc;
pub mod g2core;
pub mod grbl;
pub mod grblhal;
pub mod override_manager;
pub mod settings;
pub mod smoothieware;
pub mod tinyg;

pub use capabilities::{CapabilitiesTrait, Capability, DefaultCapabilities};
pub use capability_manager::{CapabilityManager, CapabilityState};
pub use connection_watch::{ConnectionWatchConfig, ConnectionWatchState, ConnectionWatcher};
pub use file_service::{FileInfo, FileServiceTrait, NoOpFileService, StorageInfo};
pub use firmware_detector::{FirmwareDetectionResult, FirmwareDetector};
pub use fluidnc::{FluidNCCapabilities, FluidNCController, FluidNCVersion};
pub use g2core::{G2CoreCapabilities, G2CoreController, G2CoreVersion as G2CoreVer};
pub use grbl::GrblCapabilities;
pub use grblhal::{GrblHalCapabilities, GrblHalVersion};
pub use override_manager::{
    DefaultOverrideManager, OverrideManagerTrait, OverrideState, RapidOverrideLevel,
};
pub use settings::{DefaultFirmwareSettings, FirmwareSetting, FirmwareSettingsTrait, SettingType};
pub use smoothieware::{SmoothiewareCapabilities, SmoothiewareController, SmoothiewareVersion};
pub use tinyg::{TinyGCapabilities, TinyGController, TinyGVersion as TinyGVer};

/// Supported CNC controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControllerType {
    /// GRBL (default, most common)
    #[default]
    Grbl,
    /// grblHAL (enhanced GRBL with additional features)
    GrblHal,
    /// TinyG
    TinyG,
    /// g2core (TinyG variant)
    G2Core,
    /// Smoothieware
    Smoothieware,
    /// FluidNC
    FluidNC,
    /// Unknown/generic
    Unknown,
}

impl std::fmt::Display for ControllerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grbl => write!(f, "GRBL"),
            Self::GrblHal => write!(f, "grblHAL"),
            Self::TinyG => write!(f, "TinyG"),
            Self::G2Core => write!(f, "g2core"),
            Self::Smoothieware => write!(f, "Smoothieware"),
            Self::FluidNC => write!(f, "FluidNC"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Capabilities of a specific firmware/controller
#[derive(Debug, Clone)]
pub struct FirmwareCapabilities {
    /// Controller type
    pub controller_type: ControllerType,
    /// Maximum axes supported
    pub max_axes: u8,
    /// Maximum feed rate (units per minute)
    pub max_feed_rate: f64,
    /// Maximum rapid rate (units per minute)
    pub max_rapid_rate: f64,
    /// Maximum spindle speed (RPM)
    pub max_spindle_speed: u32,
    /// Supports probing
    pub supports_probing: bool,
    /// Supports tool change
    pub supports_tool_change: bool,
    /// Supports automatic home detection
    pub supports_auto_home: bool,
    /// Buffer size for commands
    pub buffer_size: usize,
}

impl FirmwareCapabilities {
    /// Create capabilities for GRBL
    pub fn grbl() -> Self {
        Self {
            controller_type: ControllerType::Grbl,
            max_axes: 5,
            max_feed_rate: 24000.0,
            max_rapid_rate: 1000.0,
            max_spindle_speed: 255,
            supports_probing: true,
            supports_tool_change: false,
            supports_auto_home: true,
            buffer_size: 128,
        }
    }

    /// Create capabilities for grblHAL
    pub fn grblhal() -> Self {
        Self {
            controller_type: ControllerType::GrblHal,
            max_axes: 6,
            max_feed_rate: 24000.0,
            max_rapid_rate: 3000.0,
            max_spindle_speed: 30000,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            buffer_size: 256,
        }
    }

    /// Create capabilities for TinyG
    pub fn tinyg() -> Self {
        Self {
            controller_type: ControllerType::TinyG,
            max_axes: 4,
            max_feed_rate: 10000.0,
            max_rapid_rate: 3000.0,
            max_spindle_speed: 255,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            buffer_size: 64,
        }
    }

    /// Create capabilities for g2core
    pub fn g2core() -> Self {
        Self {
            controller_type: ControllerType::G2Core,
            max_axes: 6,
            max_feed_rate: 10000.0,
            max_rapid_rate: 3000.0,
            max_spindle_speed: 255,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            buffer_size: 256,
        }
    }

    /// Create capabilities for Smoothieware
    pub fn smoothieware() -> Self {
        Self {
            controller_type: ControllerType::Smoothieware,
            max_axes: 5,
            max_feed_rate: 30000.0,
            max_rapid_rate: 2000.0,
            max_spindle_speed: 255,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            buffer_size: 128,
        }
    }

    /// Create capabilities for FluidNC
    pub fn fluidnc() -> Self {
        Self {
            controller_type: ControllerType::FluidNC,
            max_axes: 6,
            max_feed_rate: 50000.0,
            max_rapid_rate: 5000.0,
            max_spindle_speed: 10000,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            buffer_size: 512,
        }
    }
}
