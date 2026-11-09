//! Smoothieware controller capabilities and feature detection

/// Smoothieware capabilities configuration
#[derive(Debug, Clone)]
pub struct SmoothiewareCapabilities {
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
    /// Supports Ethernet connectivity
    pub supports_ethernet: bool,
    /// Supports SD card
    pub supports_sd_card: bool,
}

impl Default for SmoothiewareCapabilities {
    fn default() -> Self {
        Self {
            max_feed_rate: 30000.0,
            max_rapid_rate: 2000.0,
            max_spindle_speed: 255,
            axes: 5,
            supports_probing: true,
            supports_tool_change: true,
            supports_auto_home: true,
            supports_ethernet: true,
            supports_sd_card: true,
        }
    }
}

impl SmoothiewareCapabilities {
    /// Create capabilities with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an axis is supported
    pub fn supports_axis(&self, axis: char) -> bool {
        match axis.to_ascii_uppercase() {
            'X'..='Z' => true,
            'A' | 'B' => self.axes >= 4,
            'C' => self.axes >= 5,
            _ => false,
        }
    }
}
