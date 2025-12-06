//! Global device status shared across the application

use gcodekit5_communication::firmware::grbl::status_parser::{
    MachinePosition, WorkPosition, WorkCoordinateOffset, BufferRxState, FeedSpindleState,
};
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

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
        }
    }
}

/// Global device status instance
pub static DEVICE_STATUS: Lazy<Arc<RwLock<GrblDeviceStatus>>> = 
    Lazy::new(|| Arc::new(RwLock::new(GrblDeviceStatus::default())));

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

/// Get a snapshot of the current device status
pub fn get_status() -> GrblDeviceStatus {
    DEVICE_STATUS.read().unwrap().clone()
}
