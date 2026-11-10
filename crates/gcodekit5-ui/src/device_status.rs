//! gcodekit5-ui/src/device_status.rs
//! Global device status shared across the application

use gcodekit5_communication::firmware::grbl::status_parser::{
    BufferRxState, FeedSpindleState, MachinePosition, WorkCoordinateOffset, WorkPosition,
};
use gcodekit5_core::{thread_safe_rw, ThreadSafeRw};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

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
pub static DEVICE_STATUS: Lazy<ThreadSafeRw<GrblDeviceStatus>> =
    Lazy::new(|| thread_safe_rw(GrblDeviceStatus::default()));

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

/// Whether the active device profile has Repetier pen mode on.
static ACTIVE_PEN_MODE: AtomicBool = AtomicBool::new(false);

/// Returns whether the active device uses Repetier pen mode.
pub fn get_active_pen_mode() -> bool {
    ACTIVE_PEN_MODE.load(Ordering::Relaxed)
}

/// Sets whether the active device uses Repetier pen mode.
pub fn set_active_pen_mode(enabled: bool) {
    ACTIVE_PEN_MODE.store(enabled, Ordering::Relaxed);
}

/// Controller type of the currently active device profile, as a debug string
/// (e.g., "Repetier", "Grbl", "Marlin"). None means no device is active.
///
/// We store it as a String to avoid adding a dependency on `gcodekit5-devicedb`
/// from this low-level module. Consumers can parse it if they need the enum.
static ACTIVE_CONTROLLER_TYPE: Lazy<ThreadSafeRw<Option<String>>> =
    Lazy::new(|| thread_safe_rw(None));

/// Returns the controller type of the active device (e.g., "Repetier", "Grbl").
pub fn get_active_controller_type() -> Option<String> {
    ACTIVE_CONTROLLER_TYPE.read().clone()
}

/// Sets the controller type of the active device.
pub fn set_active_controller_type(ct: Option<String>) {
    *ACTIVE_CONTROLLER_TYPE.write() = ct;
}

/// Update the machine state
pub fn update_state(state: String) {
    {
        let mut status = DEVICE_STATUS.write();
        status.state = state;
    }
}

/// Update machine position
pub fn update_machine_position(position: MachinePosition) {
    {
        let mut status = DEVICE_STATUS.write();
        status.machine_position = Some(position);
    }
}

/// Update work position
pub fn update_work_position(position: WorkPosition) {
    {
        let mut status = DEVICE_STATUS.write();
        status.work_position = Some(position);
    }
}

/// Update work coordinate offset
pub fn update_work_coordinate_offset(offset: WorkCoordinateOffset) {
    {
        let mut status = DEVICE_STATUS.write();
        status.work_coordinate_offset = Some(offset);
    }
}

/// Update buffer state
pub fn update_buffer_state(buffer: BufferRxState) {
    {
        let mut status = DEVICE_STATUS.write();
        status.buffer_state = Some(buffer);
    }
}

/// Update feed and spindle state
pub fn update_feed_spindle_state(feed_spindle: FeedSpindleState) {
    {
        let mut status = DEVICE_STATUS.write();
        status.feed_spindle_state = Some(feed_spindle);
    }
}

/// Update connection status
pub fn update_connection_status(connected: bool, port: Option<String>) {
    {
        let mut status = DEVICE_STATUS.write();
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
    {
        let mut status = DEVICE_STATUS.write();
        status.firmware_type = Some(firmware_type);
        status.firmware_version = Some(version);
        status.device_name = device_name;
    }
}

pub fn update_grbl_setting(number: u16, value: String) {
    {
        let mut status = DEVICE_STATUS.write();
        status.grbl_settings.insert(number, value);
    }
}

pub fn update_commanded_feed_rate(feed_rate: f32) {
    {
        let mut status = DEVICE_STATUS.write();
        status.commanded_feed_rate = Some(feed_rate);
    }
}

pub fn update_commanded_spindle_speed(spindle_speed: f32) {
    {
        let mut status = DEVICE_STATUS.write();
        status.commanded_spindle_speed = Some(spindle_speed);
    }
}

pub fn update_grbl_settings_bulk(settings: &[(u16, String)]) {
    {
        let mut status = DEVICE_STATUS.write();
        for (n, v) in settings {
            status.grbl_settings.insert(*n, v.clone());
        }
    }
}

pub fn get_grbl_setting(number: u16) -> Option<String> {
    DEVICE_STATUS.read().grbl_settings.get(&number).cloned()
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

/// Whether the controller reports positions in inches (`$13=1`).
pub fn reports_in_inches() -> bool {
    get_grbl_setting_numeric(13).is_some_and(|v| v >= 1.0)
}

/// Whether a job is streaming to the controller. Kept up to date by the
/// machine control panel.
static JOB_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Records whether a job is streaming to the controller.
pub fn set_job_active(active: bool) {
    JOB_ACTIVE.store(active, Ordering::Relaxed);
}

/// Whether a job is streaming to the controller.
pub fn is_job_active() -> bool {
    JOB_ACTIVE.load(Ordering::Relaxed)
}

/// Why a manual command (console line, jog) must not be sent now, or `None`.
///
/// While a job streams, the job's sender owns the controller's responses: an
/// extra line's `ok` would be credited to a job line and its `error:N` would
/// stop the job.
pub fn manual_command_blocked() -> Option<String> {
    is_job_active().then(|| crate::t!("a job is running"))
}

/// Why a `$` settings command (`$$`, `$n=v`) must not be sent now, or `None`.
///
/// Besides the job rule of [`manual_command_blocked`], GRBL rejects `$`
/// commands unless it is Idle (or in Alarm).
pub fn settings_command_blocked() -> Option<String> {
    if let Some(reason) = manual_command_blocked() {
        return Some(reason);
    }
    let state = get_status().state;
    let low = state.to_ascii_lowercase();
    if low.is_empty() || low.starts_with("idle") || low.starts_with("alarm") {
        None
    } else {
        Some(format!(
            "{} ({state})",
            crate::t!("the machine must be idle")
        ))
    }
}

/// Whether a GRBL state means the machine is not moving, so a soft reset
/// will not lose its position: a completed hold (`Hold:0`), or any state that
/// is not motion (`Idle`, `Alarm`, `Check`, `Door:0`, ...).
pub fn machine_is_stopped(state: &str) -> bool {
    let low = state.trim().to_ascii_lowercase();
    if let Some(sub) = low
        .strip_prefix("hold")
        .or_else(|| low.strip_prefix("door"))
    {
        return sub.is_empty() || sub == ":0";
    }
    !(low.starts_with("run") || low.starts_with("jog") || low.starts_with("home"))
}

/// Axes whose jog direction is reversed (Preferences > Machine > Invert).
static JOG_INVERTED: Lazy<ThreadSafeRw<Vec<char>>> = Lazy::new(|| thread_safe_rw(Vec::new()));

/// Sets whether jogging `axis` (X, Y, Z, A, B or C) runs in reverse.
pub fn set_jog_inverted(axis: char, inverted: bool) {
    let mut axes = JOG_INVERTED.write();
    axes.retain(|a| *a != axis);
    if inverted {
        axes.push(axis);
    }
}

/// `-1.0` when jogging `axis` is inverted, otherwise `1.0`.
pub fn jog_direction(axis: char) -> f32 {
    if JOG_INVERTED.read().contains(&axis) {
        -1.0
    } else {
        1.0
    }
}

/// Get a snapshot of the current device status
pub fn get_status() -> GrblDeviceStatus {
    DEVICE_STATUS.read().clone()
}
