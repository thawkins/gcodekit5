use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    CncMill,
    CncLathe,
    LaserCutter,
    ThreeDPrinter,
    Plotter,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::CncMill
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ControllerType {
    Grbl,
    TinyG,
    G2Core,
    Smoothieware,
    FluidNC,
    Marlin,
}

impl Default for ControllerType {
    fn default() -> Self {
        Self::Grbl
    }
}

impl std::fmt::Display for ControllerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grbl => write!(f, "GRBL"),
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
    pub a_axis: AxisLimits, // Rotary/Aux

    // Capabilities
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
    pub tcp_port: u16,
    pub timeout_ms: u64,
    pub auto_reconnect: bool,

    /// Last known GRBL settings (from `$$`) for this profile.
    #[serde(default)]
    pub grbl_settings: std::collections::HashMap<u8, String>,
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
            tcp_port: 23,
            timeout_ms: 5000,
            auto_reconnect: false,
            grbl_settings: std::collections::HashMap::new(),
        }
    }
}
