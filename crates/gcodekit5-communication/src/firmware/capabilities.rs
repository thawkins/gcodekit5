//! Controller capabilities system
//!
//! Provides a unified interface for querying and managing controller capabilities.

use std::collections::HashMap;

/// Capability flags for controllers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Supports probing
    Probing,
    /// Supports tool change
    ToolChange,
    /// Supports automatic homing
    AutoHome,
    /// Supports spindle control
    Spindle,
    /// Supports variable spindle speed
    VariableSpindle,
    /// Supports coolant
    Coolant,
    /// Supports Wi-Fi connectivity
    WiFi,
    /// Supports file system
    FileSystem,
    /// Supports work coordinate systems (G54-G59)
    WorkCoordinateSystems,
    /// Supports tool length offset
    ToolLengthOffset,
    /// Supports arc motion
    ArcMotion,
    /// Supports override commands
    Overrides,
    /// Supports status reports
    StatusReports,
    /// Supports E-stop
    EStop,
    /// Settings are writable (false for FluidNC which has read-only settings)
    SettingsWritable,
    /// Supports network/WiFi connectivity
    NetworkConnectivity,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Probing => write!(f, "Probing"),
            Self::ToolChange => write!(f, "Tool Change"),
            Self::AutoHome => write!(f, "Auto Home"),
            Self::Spindle => write!(f, "Spindle"),
            Self::VariableSpindle => write!(f, "Variable Spindle"),
            Self::Coolant => write!(f, "Coolant"),
            Self::WiFi => write!(f, "WiFi"),
            Self::FileSystem => write!(f, "File System"),
            Self::WorkCoordinateSystems => write!(f, "Work Coordinate Systems"),
            Self::ToolLengthOffset => write!(f, "Tool Length Offset"),
            Self::ArcMotion => write!(f, "Arc Motion"),
            Self::Overrides => write!(f, "Overrides"),
            Self::StatusReports => write!(f, "Status Reports"),
            Self::EStop => write!(f, "E-Stop"),
            Self::SettingsWritable => write!(f, "Settings Writable"),
            Self::NetworkConnectivity => write!(f, "Network Connectivity"),
        }
    }
}

/// Trait for querying controller capabilities
pub trait CapabilitiesTrait: Send + Sync {
    /// Check if a capability is supported
    fn has_capability(&self, capability: Capability) -> bool;

    /// Get all supported capabilities
    fn get_capabilities(&self) -> Vec<Capability>;

    /// Get maximum number of axes
    fn get_max_axes(&self) -> u8;

    /// Get maximum feed rate
    fn get_max_feed_rate(&self) -> f64;

    /// Get maximum rapid rate
    fn get_max_rapid_rate(&self) -> f64;

    /// Get maximum spindle speed
    fn get_max_spindle_speed(&self) -> u32;

    /// Get buffer size
    fn get_buffer_size(&self) -> usize;

    /// Check if axis is supported
    fn supports_axis(&self, axis: char) -> bool {
        let axis_num = match axis.to_ascii_uppercase() {
            'X' => 0,
            'Y' => 1,
            'Z' => 2,
            'A' => 3,
            'B' => 4,
            'C' => 5,
            'U' => 6,
            'V' => 7,
            'W' => 8,
            _ => return false,
        };
        axis_num < self.get_max_axes() as usize
    }
}

/// Default implementation of capabilities
#[derive(Debug, Clone)]
pub struct DefaultCapabilities {
    capabilities: HashMap<Capability, bool>,
    max_axes: u8,
    max_feed_rate: f64,
    max_rapid_rate: f64,
    max_spindle_speed: u32,
    buffer_size: usize,
}

impl Default for DefaultCapabilities {
    fn default() -> Self {
        let mut capabilities = HashMap::new();
        capabilities.insert(Capability::Probing, true);
        capabilities.insert(Capability::ToolChange, false);
        capabilities.insert(Capability::AutoHome, true);
        capabilities.insert(Capability::Spindle, true);
        capabilities.insert(Capability::VariableSpindle, true);
        capabilities.insert(Capability::Coolant, false);
        capabilities.insert(Capability::WiFi, false);
        capabilities.insert(Capability::FileSystem, false);
        capabilities.insert(Capability::WorkCoordinateSystems, true);
        capabilities.insert(Capability::ToolLengthOffset, false);
        capabilities.insert(Capability::ArcMotion, true);
        capabilities.insert(Capability::Overrides, true);
        capabilities.insert(Capability::StatusReports, true);
        capabilities.insert(Capability::EStop, true);
        capabilities.insert(Capability::SettingsWritable, true);
        capabilities.insert(Capability::NetworkConnectivity, false);

        Self {
            capabilities,
            max_axes: 3,
            max_feed_rate: 10000.0,
            max_rapid_rate: 1000.0,
            max_spindle_speed: 255,
            buffer_size: 128,
        }
    }
}

impl DefaultCapabilities {
    /// Create a new capabilities instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a capability
    pub fn set_capability(&mut self, capability: Capability, enabled: bool) {
        self.capabilities.insert(capability, enabled);
    }

    /// Set maximum axes
    pub fn set_max_axes(&mut self, max_axes: u8) {
        self.max_axes = max_axes;
    }

    /// Set maximum feed rate
    pub fn set_max_feed_rate(&mut self, rate: f64) {
        self.max_feed_rate = rate;
    }

    /// Set maximum rapid rate
    pub fn set_max_rapid_rate(&mut self, rate: f64) {
        self.max_rapid_rate = rate;
    }

    /// Set maximum spindle speed
    pub fn set_max_spindle_speed(&mut self, speed: u32) {
        self.max_spindle_speed = speed;
    }

    /// Set buffer size
    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = size;
    }
}

impl CapabilitiesTrait for DefaultCapabilities {
    fn has_capability(&self, capability: Capability) -> bool {
        *self.capabilities.get(&capability).unwrap_or(&false)
    }

    fn get_capabilities(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|(_, &enabled)| enabled)
            .map(|(&cap, _)| cap)
            .collect()
    }

    fn get_max_axes(&self) -> u8 {
        self.max_axes
    }

    fn get_max_feed_rate(&self) -> f64 {
        self.max_feed_rate
    }

    fn get_max_rapid_rate(&self) -> f64 {
        self.max_rapid_rate
    }

    fn get_max_spindle_speed(&self) -> u32 {
        self.max_spindle_speed
    }

    fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }
}
