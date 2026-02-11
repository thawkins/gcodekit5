//! Global device status shared across the application

use gcodekit5_communication::firmware::grbl::status_parser::{
    BufferRxState, FeedSpindleState, MachinePosition, WorkCoordinateOffset, WorkPosition,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};

/// Global GRBL device status
#[derive(Debug, Clone)]
pub struct GrblDeviceStatus {
    /// Machine state (Idle, Run, Jog, Hold, Alarm, etc.)
    pub state: String,
    /// Machine position (absolute coordinates)
    pub machine_position: Option<MachinePosition>,
    /// Work position (relative to work coordinate system)
    pub work_position: Option<WorkPosition>,
    /// Work coordinate offset
    pub work_coordinate_offset: Option<WorkCoordinateOffset>,
    /// Buffer state
    pub buffer_state: Option<BufferRxState>,
    /// Feed and spindle state
    pub feed_spindle_state: Option<FeedSpindleState>,
    /// Connection status
    pub is_connected: bool,
    /// Port name
    pub port_name: Option<String>,
    /// Firmware type (e.g., "GRBL", "TinyG")
    pub firmware_type: Option<String>,
    /// Firmware version (e.g., "1.1h")
    pub firmware_version: Option<String>,
    /// Device name (e.g., "CNC 3018 Pro")
    pub device_name: Option<String>,

    /// Last known GRBL settings (from `$$`), keyed by `$n` (u16 to support grblHAL extended settings up to $680)
    pub grbl_settings: HashMap<u16, String>,

    /// Last commanded feed rate (F value)
    pub commanded_feed_rate: Option<f32>,
    /// Last commanded spindle speed (S value)
    pub commanded_spindle_speed: Option<f32>,
}

impl Default for GrblDeviceStatus {
    fn default() -> Self {
        Self {
            state: "DISCONNECTED".to_string(),
            machine_position: None,
            work_position: None,
            work_coordinate_offset: None,
            buffer_state: None,
            feed_spindle_state: None,
            is_connected: false,
            port_name: None,
            firmware_type: None,
            firmware_version: None,
            device_name: None,
            grbl_settings: HashMap::new(),
            commanded_feed_rate: None,
            commanded_spindle_speed: None,
        }
    }
}

/// Global device status instance
pub static DEVICE_STATUS: Lazy<Arc<RwLock<GrblDeviceStatus>>> =
    Lazy::new(|| Arc::new(RwLock::new(GrblDeviceStatus::default())));

/// Number of axes on the active device (default 3).
static ACTIVE_NUM_AXES: AtomicU8 = AtomicU8::new(3);

/// Returns the number of axes configured on the active device (defaults to 3).
pub fn get_active_num_axes() -> u8 {
    ACTIVE_NUM_AXES.load(Ordering::Relaxed)
}

/// Sets the number of axes for the active device.
pub fn set_active_num_axes(n: u8) {
    ACTIVE_NUM_AXES.store(n, Ordering::Relaxed);
}

/// Update the machine state
pub fn update_state(state: String) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.state = state;
    }
}

/// Update machine position
pub fn update_machine_position(position: MachinePosition) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.machine_position = Some(position);
    }
}

/// Update work position
pub fn update_work_position(position: WorkPosition) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.work_position = Some(position);
    }
}

/// Update work coordinate offset
pub fn update_work_coordinate_offset(offset: WorkCoordinateOffset) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.work_coordinate_offset = Some(offset);
    }
}

/// Update buffer state
pub fn update_buffer_state(buffer: BufferRxState) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.buffer_state = Some(buffer);
    }
}

/// Update feed and spindle state
pub fn update_feed_spindle_state(feed_spindle: FeedSpindleState) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.feed_spindle_state = Some(feed_spindle);
    }
}

/// Update connection status
pub fn update_connection_status(connected: bool, port: Option<String>) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.is_connected = connected;
        status.port_name = port;
        if !connected {
            // Reset all status on disconnect
            *status = GrblDeviceStatus {
                is_connected: false,
                ..Default::default()
            };
        }
    }
}

/// Update firmware information
pub fn update_firmware_info(firmware_type: String, version: String, device_name: Option<String>) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.firmware_type = Some(firmware_type);
        status.firmware_version = Some(version);
        status.device_name = device_name;
    }
}

pub fn update_grbl_setting(number: u16, value: String) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.grbl_settings.insert(number, value);
    }
}

pub fn update_commanded_feed_rate(feed_rate: f32) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.commanded_feed_rate = Some(feed_rate);
    }
}

pub fn update_commanded_spindle_speed(spindle_speed: f32) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        status.commanded_spindle_speed = Some(spindle_speed);
    }
}

pub fn update_grbl_settings_bulk(settings: &[(u16, String)]) {
    if let Ok(mut status) = DEVICE_STATUS.write() {
        for (n, v) in settings {
            status.grbl_settings.insert(*n, v.clone());
        }
    }
}

pub fn get_grbl_setting(number: u16) -> Option<String> {
    DEVICE_STATUS
        .read()
        .ok()
        .and_then(|s| s.grbl_settings.get(&number).cloned())
}

fn parse_numeric_prefix(s: &str) -> Option<f64> {
    let s = s.trim();
    let mut end = 0usize;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
            end = i + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        None
    } else {
        s[..end].parse::<f64>().ok()
    }
}

pub fn get_grbl_setting_numeric(number: u16) -> Option<f64> {
    get_grbl_setting(number).and_then(|v| parse_numeric_prefix(&v))
}

/// Get a snapshot of the current device status
pub fn get_status() -> GrblDeviceStatus {
    DEVICE_STATUS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}
