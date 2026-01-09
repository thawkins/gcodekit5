//! grblHAL controller capabilities and feature detection

/// grblHAL capabilities configuration
/// grblHAL is GRBL-compatible but with enhanced features
#[derive(Debug, Clone)]
pub struct GrblHalCapabilities {
    /// Maximum feed rate (units per minute)
    pub max_feed_rate: f64,
    /// Maximum rapid rate (units per minute)
    pub max_rapid_rate: f64,
    /// Maximum spindle speed (RPM)
    pub max_spindle_speed: u32,
    /// Number of axes supported (grblHAL supports up to 6)
    pub axes: u8,
    /// Supports probing
    pub supports_probing: bool,
    /// Supports tool change
    pub supports_tool_change: bool,
    /// Supports auto-homing
    pub supports_auto_home: bool,
    /// Supports network/WiFi connectivity (telnet, websocket)
    pub supports_network: bool,
    /// Supports plugins
    pub supports_plugins: bool,
    /// Supports SD card/file system
    pub supports_filesystem: bool,
    /// Settings are writable via $nn=value
    pub settings_writable: bool,
    /// Supports work coordinate systems
    pub supports_wcs: bool,
}

impl Default for GrblHalCapabilities {
    fn default() -> Self {
        Self {
            max_feed_rate: 24000.0,
            max_rapid_rate: 3000.0,
            max_spindle_speed: 30000,
            axes: 6,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            supports_network: true,
            supports_plugins: true,
            supports_filesystem: true,
            settings_writable: true,
            supports_wcs: true,
        }
    }
}

impl GrblHalCapabilities {
    /// Create capabilities with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an axis is supported
    pub fn supports_axis(&self, axis: char) -> bool {
        match axis.to_ascii_uppercase() {
            'X' | 'Y' | 'Z' => true,
            'A' => self.axes >= 4,
            'B' => self.axes >= 5,
            'C' => self.axes >= 6,
            _ => false,
        }
    }

    /// Check if network connectivity is available
    pub fn has_network(&self) -> bool {
        self.supports_network
    }

    /// Check if file system is available
    pub fn has_filesystem(&self) -> bool {
        self.supports_filesystem
    }

    /// Check if settings are writable
    pub fn are_settings_writable(&self) -> bool {
        self.settings_writable
    }

    /// Check if plugins are supported
    pub fn has_plugins(&self) -> bool {
        self.supports_plugins
    }
}
