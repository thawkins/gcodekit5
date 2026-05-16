//! # Device Data Model
//!
//! Defines the data structures for CNC device profiles, including
//! machine dimensions, firmware configuration, and connection settings.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum DeviceType {
    #[default]
    CncMill,
    CncLathe,
    LaserCutter,
    ThreeDPrinter,
    Plotter,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CncMill => write!(f, "CNC Mill"),
            Self::CncLathe => write!(f, "CNC Lathe"),
            Self::LaserCutter => write!(f, "Laser Cutter"),
            Self::ThreeDPrinter => write!(f, "3D Printer"),
            Self::Plotter => write!(f, "Plotter"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ControllerType {
    #[default]
    Grbl,
    GrblHal,
    TinyG,
    G2Core,
    Smoothieware,
    FluidNC,
    Marlin,
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
            Self::Marlin => write!(f, "Marlin"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisLimits {
    pub min: f64,
    pub max: f64,
    pub enabled: bool,
}

impl Default for AxisLimits {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 200.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub device_type: DeviceType,
    pub controller_type: ControllerType,

    // Workspace Limits
    pub x_axis: AxisLimits,
    pub y_axis: AxisLimits,
    pub z_axis: AxisLimits,
    pub a_axis: AxisLimits, // Rotary/Aux - around X axis
    pub b_axis: AxisLimits, // Rotary/Aux - around Y axis (5th axis)
    pub c_axis: AxisLimits, // Rotary/Aux - around Z axis (6th axis)

    // Capabilities
    pub num_axes: u8,
    pub has_spindle: bool,
    pub has_laser: bool,
    pub has_coolant: bool,
    pub max_feed_rate: f64,
    pub max_s_value: f64,
    pub max_spindle_speed_rpm: u32,

    // Power
    pub cnc_spindle_watts: f64,
    pub laser_watts: f64,

    // Connection Settings
    pub connection_type: String,
    pub baud_rate: u32,
    pub port: String,
    pub tcp_host: String,
    pub tcp_port: u16,
    pub timeout_ms: u64,
    pub auto_reconnect: bool,

    /// Last known GRBL settings (from `$$`) for this profile (u16 to support grblHAL extended settings up to $680).
    #[serde(default)]
    pub grbl_settings: std::collections::HashMap<u16, String>,
}

impl DeviceProfile {
    /// Returns the number of enabled axes based on axis configuration
    pub fn axis_count(&self) -> u8 {
        let mut count = 3; // X, Y, Z are always present
        if self.a_axis.enabled {
            count += 1;
        }
        if self.b_axis.enabled {
            count += 1;
        }
        if self.c_axis.enabled {
            count += 1;
        }
        count
    }

    /// Returns true if the device has 4 or more axes (includes A axis)
    pub fn has_a_axis(&self) -> bool {
        self.a_axis.enabled
    }

    /// Returns true if the device has 5 or more axes (includes B axis)
    pub fn has_b_axis(&self) -> bool {
        self.b_axis.enabled
    }

    /// Returns true if the device has 6 axes (includes C axis)
    pub fn has_c_axis(&self) -> bool {
        self.c_axis.enabled
    }
}

impl Default for DeviceProfile {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "New Device".to_string(),
            description: "".to_string(),
            device_type: DeviceType::default(),
            controller_type: ControllerType::default(),
            x_axis: AxisLimits::default(),
            y_axis: AxisLimits::default(),
            z_axis: AxisLimits {
                min: 0.0,
                max: 100.0,
                enabled: true,
            },
            a_axis: AxisLimits {
                min: 0.0,
                max: 360.0,
                enabled: false,
            },
            b_axis: AxisLimits {
                min: 0.0,
                max: 360.0,
                enabled: false,
            },
            c_axis: AxisLimits {
                min: 0.0,
                max: 360.0,
                enabled: false,
            },
            num_axes: 3,
            has_spindle: true,
            has_laser: false,
            has_coolant: false,
            max_feed_rate: 1000.0,
            max_s_value: 1000.0,
            max_spindle_speed_rpm: 12000,
            cnc_spindle_watts: 500.0,
            laser_watts: 5.0,
            connection_type: "Serial".to_string(),
            baud_rate: 115200,
            port: "Auto".to_string(),
            tcp_host: "192.168.1.100".to_string(),
            tcp_port: 23,
            timeout_ms: 5000,
            auto_reconnect: false,
            grbl_settings: std::collections::HashMap::new(),
        }
    }
}
