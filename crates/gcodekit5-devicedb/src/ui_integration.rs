use crate::manager::DeviceManager;
use crate::model::{ControllerType, DeviceProfile, DeviceType};
use anyhow::Context;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DeviceProfileUiModel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub device_type: String,
    pub controller_type: String,
    pub x_min: String,
    pub x_max: String,
    pub y_min: String,
    pub y_max: String,
    pub z_min: String,
    pub z_max: String,
    pub has_spindle: bool,
    pub has_laser: bool,
    pub has_coolant: bool,
    pub max_feed_rate: String,
    pub max_s_value: String,
    pub max_spindle_speed_rpm: String,
    pub cnc_spindle_watts: String,
    pub laser_watts: String,
    pub connection_type: String,
    pub baud_rate: String,
    pub port: String,
    pub tcp_port: String,
    pub timeout_ms: String,
    pub auto_reconnect: bool,
    pub is_active: bool,
}

impl From<DeviceProfile> for DeviceProfileUiModel {
    fn from(p: DeviceProfile) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            device_type: p.device_type.to_string(),
            controller_type: p.controller_type.to_string(),
            x_min: format!("{:.2}", p.x_axis.min),
            x_max: format!("{:.2}", p.x_axis.max),
            y_min: format!("{:.2}", p.y_axis.min),
            y_max: format!("{:.2}", p.y_axis.max),
            z_min: format!("{:.2}", p.z_axis.min),
            z_max: format!("{:.2}", p.z_axis.max),
            has_spindle: p.has_spindle,
            has_laser: p.has_laser,
            has_coolant: p.has_coolant,
            max_feed_rate: format!("{:.0}", p.max_feed_rate),
            max_s_value: format!("{:.0}", p.max_s_value),
            max_spindle_speed_rpm: p.max_spindle_speed_rpm.to_string(),
            cnc_spindle_watts: format!("{:.0}", p.cnc_spindle_watts),
            laser_watts: format!("{:.0}", p.laser_watts),
            connection_type: p.connection_type,
            baud_rate: p.baud_rate.to_string(),
            port: p.port,
            tcp_port: p.tcp_port.to_string(),
            timeout_ms: p.timeout_ms.to_string(),
            auto_reconnect: p.auto_reconnect,
            is_active: false, // Set separately
        }
    }
}

pub struct DeviceUiController {
    manager: Arc<DeviceManager>,
}

impl DeviceUiController {
    pub fn new(manager: Arc<DeviceManager>) -> Self {
        Self { manager }
    }

    pub fn get_ui_profiles(&self) -> Vec<DeviceProfileUiModel> {
        let profiles = self.manager.get_all_profiles();
        let active_profile = self.manager.get_active_profile();
        let active_id = active_profile.map(|p| p.id).unwrap_or_default();

        let mut ui_profiles: Vec<DeviceProfileUiModel> = profiles
            .into_iter()
            .map(|p| {
                let mut ui_model: DeviceProfileUiModel = p.into();
                ui_model.is_active = ui_model.id == active_id;
                ui_model
            })
            .collect();

        ui_profiles.sort_by(|a, b| a.name.cmp(&b.name));
        ui_profiles
    }

    pub fn update_profile_from_ui(&self, ui_model: DeviceProfileUiModel) -> anyhow::Result<()> {
        let mut profile = self.manager.get_profile(&ui_model.id).unwrap_or_default();

        profile.id = ui_model.id;
        profile.name = ui_model.name;
        profile.description = ui_model.description;

        profile.device_type = match ui_model.device_type.as_str() {
            "CNC Mill" => DeviceType::CncMill,
            "CNC Lathe" => DeviceType::CncLathe,
            "Laser Cutter" => DeviceType::LaserCutter,
            "3D Printer" => DeviceType::ThreeDPrinter,
            "Plotter" => DeviceType::Plotter,
            _ => anyhow::bail!("Unknown device type: {}", ui_model.device_type),
        };

        profile.controller_type = match ui_model.controller_type.as_str() {
            "GRBL" => ControllerType::Grbl,
            "TinyG" => ControllerType::TinyG,
            "g2core" => ControllerType::G2Core,
            "Smoothieware" => ControllerType::Smoothieware,
            "FluidNC" => ControllerType::FluidNC,
            "Marlin" => ControllerType::Marlin,
            _ => anyhow::bail!("Unknown controller type: {}", ui_model.controller_type),
        };

        let mut x_min: f64 = ui_model
            .x_min
            .trim()
            .parse()
            .with_context(|| format!("X Min must be a number (got {})", ui_model.x_min))?;
        let mut x_max: f64 = ui_model
            .x_max
            .trim()
            .parse()
            .with_context(|| format!("X Max must be a number (got {})", ui_model.x_max))?;
        if x_min > x_max {
            std::mem::swap(&mut x_min, &mut x_max);
        }
        profile.x_axis.min = x_min;
        profile.x_axis.max = x_max;

        let mut y_min: f64 = ui_model
            .y_min
            .trim()
            .parse()
            .with_context(|| format!("Y Min must be a number (got {})", ui_model.y_min))?;
        let mut y_max: f64 = ui_model
            .y_max
            .trim()
            .parse()
            .with_context(|| format!("Y Max must be a number (got {})", ui_model.y_max))?;
        if y_min > y_max {
            std::mem::swap(&mut y_min, &mut y_max);
        }
        profile.y_axis.min = y_min;
        profile.y_axis.max = y_max;

        let mut z_min: f64 = ui_model
            .z_min
            .trim()
            .parse()
            .with_context(|| format!("Z Min must be a number (got {})", ui_model.z_min))?;
        let mut z_max: f64 = ui_model
            .z_max
            .trim()
            .parse()
            .with_context(|| format!("Z Max must be a number (got {})", ui_model.z_max))?;
        if z_min > z_max {
            std::mem::swap(&mut z_min, &mut z_max);
        }
        profile.z_axis.min = z_min;
        profile.z_axis.max = z_max;

        profile.has_spindle = ui_model.has_spindle;
        profile.has_laser = ui_model.has_laser;
        profile.has_coolant = ui_model.has_coolant;

        profile.max_feed_rate = ui_model
            .max_feed_rate
            .trim()
            .parse()
            .with_context(|| {
                format!(
                    "Max Feed Rate must be a number (got {})",
                    ui_model.max_feed_rate
                )
            })?;
        profile.max_s_value = ui_model
            .max_s_value
            .trim()
            .parse()
            .with_context(|| format!("Max S-Value must be a number (got {})", ui_model.max_s_value))?;

        profile.max_spindle_speed_rpm = ui_model
            .max_spindle_speed_rpm
            .trim()
            .parse()
            .with_context(|| {
                format!(
                    "Max Spindle Speed (RPM) must be an integer (got {})",
                    ui_model.max_spindle_speed_rpm
                )
            })?;

        profile.cnc_spindle_watts = ui_model
            .cnc_spindle_watts
            .trim()
            .parse()
            .with_context(|| {
                format!(
                    "Spindle watts must be a number (got {})",
                    ui_model.cnc_spindle_watts
                )
            })?;
        profile.laser_watts = ui_model
            .laser_watts
            .trim()
            .parse()
            .with_context(|| format!("Laser watts must be a number (got {})", ui_model.laser_watts))?;

        profile.connection_type = ui_model.connection_type;
        profile.baud_rate = ui_model
            .baud_rate
            .trim()
            .parse()
            .with_context(|| format!("Baud Rate must be an integer (got {})", ui_model.baud_rate))?;
        profile.port = ui_model.port;
        profile.tcp_port = ui_model
            .tcp_port
            .trim()
            .parse()
            .with_context(|| format!("TCP Port must be an integer (got {})", ui_model.tcp_port))?;
        profile.timeout_ms = ui_model
            .timeout_ms
            .trim()
            .parse()
            .with_context(|| format!("Timeout must be an integer (got {})", ui_model.timeout_ms))?;
        profile.auto_reconnect = ui_model.auto_reconnect;

        self.manager.save_profile(profile)
    }

    pub fn create_new_profile(&self) -> anyhow::Result<String> {
        let profile = DeviceProfile::default();
        let id = profile.id.clone();
        self.manager.save_profile(profile)?;
        Ok(id)
    }

    pub fn delete_profile(&self, id: &str) -> anyhow::Result<()> {
        self.manager.delete_profile(id)
    }

    pub fn set_active_profile(&self, id: &str) -> anyhow::Result<()> {
        self.manager.set_active_profile(id)
    }
}
