//! # Device Profile Templates
//!
//! Provides preset device profile templates for common CNC machine configurations,
//! including 3-axis, 4-axis, 5-axis, and 6-axis machines.

use crate::model::{AxisLimits, ControllerType, DeviceProfile, DeviceType};

/// Create a default 3-axis mill template
pub fn template_3axis_mill() -> DeviceProfile {
    DeviceProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: "3-Axis Mill".to_string(),
        description: "Standard 3-axis CNC milling machine".to_string(),
        device_type: DeviceType::CncMill,
        controller_type: ControllerType::Grbl,
        x_axis: AxisLimits {
            min: 0.0,
            max: 200.0,
            enabled: true,
        },
        y_axis: AxisLimits {
            min: 0.0,
            max: 200.0,
            enabled: true,
        },
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
        has_coolant: true,
        max_feed_rate: 1000.0,
        max_s_value: 1000.0,
        max_spindle_speed_rpm: 12000,
        cnc_spindle_watts: 500.0,
        laser_watts: 0.0,
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

/// Create a 4-axis mill template with rotary table (A axis)
pub fn template_4axis_mill() -> DeviceProfile {
    DeviceProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: "4-Axis Mill".to_string(),
        description: "4-axis CNC mill with rotary table (A axis)".to_string(),
        device_type: DeviceType::CncMill,
        controller_type: ControllerType::Grbl,
        x_axis: AxisLimits {
            min: 0.0,
            max: 300.0,
            enabled: true,
        },
        y_axis: AxisLimits {
            min: 0.0,
            max: 200.0,
            enabled: true,
        },
        z_axis: AxisLimits {
            min: 0.0,
            max: 150.0,
            enabled: true,
        },
        a_axis: AxisLimits {
            min: 0.0,
            max: 360.0,
            enabled: true,
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
        num_axes: 4,
        has_spindle: true,
        has_laser: false,
        has_coolant: true,
        max_feed_rate: 1500.0,
        max_s_value: 1000.0,
        max_spindle_speed_rpm: 18000,
        cnc_spindle_watts: 800.0,
        laser_watts: 0.0,
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

/// Create a 5-axis mill template with trunnion table (A and B axes)
pub fn template_5axis_mill() -> DeviceProfile {
    DeviceProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: "5-Axis Mill (Trunnion)".to_string(),
        description: "5-axis CNC mill with trunnion table (A tilt, B rotate)".to_string(),
        device_type: DeviceType::CncMill,
        controller_type: ControllerType::GrblHal,
        x_axis: AxisLimits {
            min: 0.0,
            max: 400.0,
            enabled: true,
        },
        y_axis: AxisLimits {
            min: 0.0,
            max: 300.0,
            enabled: true,
        },
        z_axis: AxisLimits {
            min: 0.0,
            max: 200.0,
            enabled: true,
        },
        a_axis: AxisLimits {
            min: -110.0,
            max: 110.0,
            enabled: true,
        },
        b_axis: AxisLimits {
            min: 0.0,
            max: 360.0,
            enabled: true,
        },
        c_axis: AxisLimits {
            min: 0.0,
            max: 360.0,
            enabled: false,
        },
        num_axes: 5,
        has_spindle: true,
        has_laser: false,
        has_coolant: true,
        max_feed_rate: 2000.0,
        max_s_value: 1000.0,
        max_spindle_speed_rpm: 24000,
        cnc_spindle_watts: 1500.0,
        laser_watts: 0.0,
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

/// Create a 6-axis mill template with full rotary table (A, B, and C axes)
pub fn template_6axis_mill() -> DeviceProfile {
    DeviceProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: "6-Axis Mill".to_string(),
        description: "6-axis CNC mill with full rotary capability".to_string(),
        device_type: DeviceType::CncMill,
        controller_type: ControllerType::GrblHal,
        x_axis: AxisLimits {
            min: 0.0,
            max: 500.0,
            enabled: true,
        },
        y_axis: AxisLimits {
            min: 0.0,
            max: 400.0,
            enabled: true,
        },
        z_axis: AxisLimits {
            min: 0.0,
            max: 300.0,
            enabled: true,
        },
        a_axis: AxisLimits {
            min: -110.0,
            max: 110.0,
            enabled: true,
        },
        b_axis: AxisLimits {
            min: -90.0,
            max: 90.0,
            enabled: true,
        },
        c_axis: AxisLimits {
            min: 0.0,
            max: 360.0,
            enabled: true,
        },
        num_axes: 6,
        has_spindle: true,
        has_laser: false,
        has_coolant: true,
        max_feed_rate: 3000.0,
        max_s_value: 1000.0,
        max_spindle_speed_rpm: 24000,
        cnc_spindle_watts: 2200.0,
        laser_watts: 0.0,
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

/// Get all available templates
pub fn get_all_templates() -> Vec<DeviceProfile> {
    vec![
        template_3axis_mill(),
        template_4axis_mill(),
        template_5axis_mill(),
        template_6axis_mill(),
    ]
}

/// Get templates filtered by axis count
pub fn get_templates_by_axis_count(axis_count: u8) -> Vec<DeviceProfile> {
    get_all_templates()
        .into_iter()
        .filter(|t| t.axis_count() == axis_count)
        .collect()
}
