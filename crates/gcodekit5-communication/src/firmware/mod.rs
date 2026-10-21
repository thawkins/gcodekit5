//! gcodekit5-communication/src/firmware/mod.rs
//! Firmware implementations for various CNC controllers
//!
//! Supported controllers:
//! - GRBL: Open-source CNC control software
//! - Repetier: 3D printer firmware with its own line processor and flow-control sender
//! Supported via GRBL
//! - FluidNC: Powerful open-source CNC control
//! - grblHAL: Enhanced GRBL with additional features
//! Planned but not supported
//! - TinyG: CNC control for 3D printers and engravers
//! - g2core: Next generation of TinyG
//! - Smoothieware: CNC control software
//! - Marlin: 3D printer firmware

pub mod capabilities;
pub mod connection_watch;
pub mod device_status;
pub mod file_service;
pub mod fluidnc;
pub mod g2core;
pub mod grbl;
pub mod grblhal;
pub mod override_manager;
pub mod protocol_registry;
pub mod repetier;
pub mod sender;
pub mod settings;
pub mod smoothieware;
pub mod tinyg;

pub use capabilities::{CapabilitiesTrait, Capability, DefaultCapabilities};
pub use connection_watch::{ConnectionWatchConfig, ConnectionWatchState, ConnectionWatcher};
pub use file_service::{FileInfo, FileServiceTrait, NoOpFileService, StorageInfo};
pub use fluidnc::{FluidNCCapabilities, FluidNCController, FluidNCVersion};
pub use g2core::{G2CoreCapabilities, G2CoreController, G2CoreVersion as G2CoreVer};
pub use grblhal::{GrblHalCapabilities, GrblHalVersion};
pub use override_manager::{
    DefaultOverrideManager, OverrideManagerTrait, OverrideState, RapidOverrideLevel,
};

pub use protocol_registry::{
    protocol_profile, ProtocolProfile, SenderKind, FALLBACK_PROFILE, PROTOCOLS,
};

pub use settings::{DefaultFirmwareSettings, FirmwareSetting, FirmwareSettingsTrait, SettingType};

pub use smoothieware::{SmoothiewareCapabilities, SmoothiewareController, SmoothiewareVersion};
pub use tinyg::{TinyGCapabilities, TinyGController, TinyGVersion as TinyGVer};
pub use sender::Sender;

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
    /// Repetier (3D printer firmware)
    Repetier,
    /// Marlin (3D printer firmware)
    Marlin,
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
            Self::Repetier => write!(f, "Repetier"),
            Self::Marlin => write!(f, "Marlin"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Protocolo de control, usado tanto para la configuración del usuario
/// (desde el Device Manager) como para la autodetección (al leer bytes
/// del dispositivo).
///
/// Es la **fuente de verdad** del modo operativo del `SerialCommunicator`
/// (cuando se usa como configuración) y el sujeto del chequeo de
/// divergencia (cuando se compara config vs detección).
///
/// ## Añadir un protocolo nuevo
///
/// 1. Descomentar la variante aquí.
/// 2. Añadir el match en `as_str()` y `from_str()`.
/// 3. Añadir una entrada en `PROTOCOLS` (en `protocol_registry.rs`).
/// 4. Implementar sender y processor si aplica.
///
/// El resto del código sigue funcionando sin cambios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControllerProtocol {
    Grbl,
    Repetier,

    // ─── Future protocols (uncomment when implementing) ───
    // Marlin,
    // FluidNC,
    // TinyG,
    // G2Core,
    // Smoothieware,
    // GrblHal,

    /// Unknown or not yet detected.
    #[default]
    Unknown,
}

impl ControllerProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Grbl => "GRBL",
            Self::Repetier => "Repetier",
            // Self::Marlin => "Marlin",
            // Self::FluidNC => "FluidNC",
            // Self::TinyG => "TinyG",
            // Self::G2Core => "g2core",
            // Self::Smoothieware => "Smoothieware",
            // Self::GrblHal => "grblHAL",
            Self::Unknown => "Unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "GRBL" | "Grbl" => Self::Grbl,
            "Repetier" => Self::Repetier,
            // "Marlin" => Self::Marlin,
            // "FluidNC" => Self::FluidNC,
            // "TinyG" => Self::TinyG,
            // "g2core" | "G2Core" => Self::G2Core,
            // "Smoothieware" => Self::Smoothieware,
            // "grblHAL" | "GrblHal" => Self::GrblHal,
            _ => Self::Unknown,
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

impl std::fmt::Display for ControllerProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
