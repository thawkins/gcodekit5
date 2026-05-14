//! Device database for managing CNC controller devices
//!
//! Tracks device information, firmware versions, capabilities, and connection details.

use super::capabilities_db::{CapabilitiesDatabase, FirmwareCapabilities};
use super::firmware_version::{FirmwareType, SemanticVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Spindle type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpindleType {
    /// ER collet spindle
    Collet,
    /// Chuck spindle
    Chuck,
    /// Belt drive spindle
    BeltDrive,
    /// Direct drive spindle
    DirectDrive,
    /// Laser spindle
    Laser,
    /// Other spindle type
    Other,
}

impl std::fmt::Display for SpindleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collet => write!(f, "Collet"),
            Self::Chuck => write!(f, "Chuck"),
            Self::BeltDrive => write!(f, "Belt Drive"),
            Self::DirectDrive => write!(f, "Direct Drive"),
            Self::Laser => write!(f, "Laser"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// Device health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceHealth {
    /// Device is healthy
    Healthy,
    /// Device has warnings
    Warning,
    /// Device has errors
    Error,
    /// Device health unknown
    Unknown,
}

/// Work area dimensions
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorkArea {
    /// X axis maximum (mm)
    pub x_max: f64,
    /// Y axis maximum (mm)
    pub y_max: f64,
    /// Z axis maximum (mm)
    pub z_max: f64,
    /// Z axis minimum (mm)
    pub z_min: Option<f64>,
}

impl WorkArea {
    /// Create new work area
    pub fn new(x_max: f64, y_max: f64, z_max: f64) -> Self {
        Self {
            x_max,
            y_max,
            z_max,
            z_min: None,
        }
    }
}

/// Device type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier
    pub id: String,

    /// User-friendly device name
    pub name: String,

    /// Firmware type
    pub firmware_type: FirmwareType,

    /// Detected firmware version
    pub firmware_version: Option<SemanticVersion>,

    /// Last detected firmware version
    pub last_detected_version: Option<SemanticVersion>,

    /// Serial number
    pub serial_number: Option<String>,

    /// MAC address (for network devices)
    pub mac_address: Option<String>,

    /// Number of axes
    pub axes: u8,

    /// Spindle type
    pub spindle_type: SpindleType,

    /// Maximum spindle RPM
    pub max_rpm: u32,

    /// Work area dimensions
    pub work_area: Option<WorkArea>,

    /// Connection port (serial port or IP)
    pub connection_port: String,

    /// Auto-connect on startup
    pub auto_connect: bool,

    /// Is this the primary device
    pub is_primary: bool,

    /// Is this device marked as favorite
    pub is_favorite: bool,

    /// Device group/collection name
    pub group: Option<String>,

    /// Last connection time
    pub last_connected: Option<SystemTime>,

    /// Total connection count
    pub connection_count: u32,

    /// Total operating time in minutes
    pub total_runtime_minutes: u64,

    /// Device health status
    pub device_health: DeviceHealth,

    /// User notes
    pub notes: String,

    /// Custom settings
    pub custom_settings: HashMap<String, String>,

    /// Creation time
    pub created_at: SystemTime,

    /// Last update time
    pub updated_at: SystemTime,
}

impl Device {
    /// Create a new device
    pub fn new(id: String, name: String, firmware_type: FirmwareType) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            name,
            firmware_type,
            firmware_version: None,
            last_detected_version: None,
            serial_number: None,
            mac_address: None,
            axes: 3,
            spindle_type: SpindleType::Collet,
            max_rpm: 24000,
            work_area: None,
            connection_port: String::new(),
            auto_connect: false,
            is_primary: false,
            is_favorite: false,
            group: None,
            last_connected: None,
            connection_count: 0,
            total_runtime_minutes: 0,
            device_health: DeviceHealth::Unknown,
            notes: String::new(),
            custom_settings: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Mark device as just connected
    pub fn record_connection(&mut self) {
        self.last_connected = Some(SystemTime::now());
        self.connection_count += 1;
        self.updated_at = SystemTime::now();
    }

    /// Add runtime minutes
    pub fn add_runtime(&mut self, minutes: u64) {
        self.total_runtime_minutes += minutes;
        self.updated_at = SystemTime::now();
    }

    /// Get human-readable device description
    pub fn description(&self) -> String {
        format!(
            "{} - {} ({} axes)",
            self.name, self.firmware_type, self.axes
        )
    }
}

/// Device database
pub struct DeviceDatabase {
    /// Devices by ID
    devices: HashMap<String, Device>,
    /// Next device ID counter
    next_id: u32,
}

impl DeviceDatabase {
    /// Create new device database
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a device to the database
    pub fn add_device(&mut self, device: Device) {
        let device_id = device.id.clone();
        self.devices.insert(device_id, device.clone());
        // Update ID counter if needed
        if let Ok(id_num) = device.id.split('_').nth(1).unwrap_or("0").parse::<u32>() {
            self.next_id = self.next_id.max(id_num + 1);
        }
    }

    /// Create and add a new device
    pub fn create_device(&mut self, name: String, firmware_type: FirmwareType) -> String {
        let id = format!("device_{:03}", self.next_id);
        self.next_id += 1;

        let mut device = Device::new(id.clone(), name, firmware_type);
        device.is_primary = self.devices.is_empty(); // First device is primary
        self.add_device(device);

        id
    }

    /// Get device by ID
    pub fn get_device(&self, id: &str) -> Option<&Device> {
        self.devices.get(id)
    }

    /// Get mutable device by ID
    pub fn get_device_mut(&mut self, id: &str) -> Option<&mut Device> {
        self.devices.get_mut(id)
    }

    /// Get device by name
    pub fn find_device_by_name(&self, name: &str) -> Option<&Device> {
        self.devices.values().find(|d| d.name == name)
    }

    /// Get device by serial number
    pub fn find_device_by_serial(&self, serial: &str) -> Option<&Device> {
        self.devices
            .values()
            .find(|d| d.serial_number.as_ref().is_some_and(|s| s == serial))
    }

    /// Get primary device
    pub fn get_primary_device(&self) -> Option<&Device> {
        self.devices.values().find(|d| d.is_primary)
    }

    /// Get all devices
    pub fn all_devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    /// Get devices in group
    pub fn devices_in_group(&self, group: &str) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|d| d.group.as_ref().is_some_and(|g| g == group))
            .collect()
    }

    /// Get favorite devices
    pub fn favorite_devices(&self) -> Vec<&Device> {
        self.devices.values().filter(|d| d.is_favorite).collect()
    }

    /// Get recently used devices (sorted by last_connected)
    pub fn recent_devices(&self, count: usize) -> Vec<&Device> {
        let mut devices: Vec<_> = self
            .devices
            .values()
            .filter(|d| d.last_connected.is_some())
            .collect();
        devices.sort_by_key(|b| std::cmp::Reverse(b.last_connected));
        devices.into_iter().take(count).collect()
    }

    /// Set primary device
    pub fn set_primary_device(&mut self, id: &str) -> Result<(), String> {
        // Clear current primary
        for device in self.devices.values_mut() {
            device.is_primary = false;
        }

        // Set new primary
        if let Some(device) = self.devices.get_mut(id) {
            device.is_primary = true;
            device.updated_at = SystemTime::now();
            Ok(())
        } else {
            Err(format!("Device {} not found", id))
        }
    }

    /// Delete device
    pub fn delete_device(&mut self, id: &str) -> Result<(), String> {
        if self.devices.remove(id).is_some() {
            // If deleted device was primary, set first remaining as primary
            if self.devices.is_empty() {
                return Ok(());
            }
            if !self.devices.values().any(|d| d.is_primary) {
                if let Some(device) = self.devices.values_mut().next() {
                    device.is_primary = true;
                }
            }
            Ok(())
        } else {
            Err(format!("Device {} not found", id))
        }
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Get capabilities for a device
    pub fn get_device_capabilities(
        &self,
        id: &str,
        capabilities_db: &CapabilitiesDatabase,
    ) -> Option<FirmwareCapabilities> {
        let device = self.get_device(id)?;
        if let Some(version) = device.firmware_version {
            capabilities_db.get_capabilities(device.firmware_type, &version)
        } else {
            None
        }
    }
}

impl Default for DeviceDatabase {
    fn default() -> Self {
        Self::new()
    }
}
