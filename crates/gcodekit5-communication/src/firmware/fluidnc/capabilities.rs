//! FluidNC controller capabilities and feature detection

/// FluidNC capabilities configuration
#[derive(Debug, Clone)]
pub struct FluidNCCapabilities {
    /// Maximum feed rate (units per minute)
    pub max_feed_rate: f64,
    /// Maximum rapid rate (units per minute)
    pub max_rapid_rate: f64,
    /// Maximum spindle speed (RPM)
    pub max_spindle_speed: u32,
    /// Number of axes supported
    pub axes: u8,
    /// Supports probing
    pub supports_probing: bool,
    /// Supports tool change
    pub supports_tool_change: bool,
    /// Supports auto-homing
    pub supports_auto_home: bool,
    /// Supports WiFi connectivity
    pub supports_wifi: bool,
    /// Supports file system
    pub supports_filesystem: bool,
    /// Supports web interface
    pub supports_web_interface: bool,
    /// Settings are read-only (FluidNC uses YAML config files)
    pub settings_read_only: bool,
}

impl Default for FluidNCCapabilities {
    fn default() -> Self {
        Self {
            max_feed_rate: 50000.0,
            max_rapid_rate: 5000.0,
            max_spindle_speed: 10000,
            axes: 6,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            supports_wifi: true,
            supports_filesystem: true,
            supports_web_interface: true,
            settings_read_only: true, // FluidNC uses YAML config, $$ is read-only
        }
    }
}

impl FluidNCCapabilities {
    /// Create capabilities with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an axis is supported
    pub fn supports_axis(&self, axis: char) -> bool {
        match axis.to_ascii_uppercase() {
            'X' | 'Y' | 'Z' => true,
            'A' | 'B' => self.axes >= 4,
            'C' => self.axes >= 5,
            'U' => self.axes >= 6,
            _ => false,
        }
    }

    /// Check if WiFi is available
    pub fn has_wifi(&self) -> bool {
        self.supports_wifi
    }

    /// Check if file system is available
    pub fn has_filesystem(&self) -> bool {
        self.supports_filesystem
    }

    /// Check if settings are writable (false for FluidNC)
    pub fn are_settings_writable(&self) -> bool {
        !self.settings_read_only
    }
}
