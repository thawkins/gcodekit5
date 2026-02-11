use crate::editor::EditorBridge;
use crate::ui::gtk::device_info::CapabilityItem;
use crate::ui::main_window::MainWindow;
use gcodekit5_communication::firmware::firmware_version::{FirmwareType, SemanticVersion};
use gcodekit5_communication::{list_ports, CapabilityManager};

/// Copy text to clipboard using arboard crate
pub fn copy_to_clipboard(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text.to_string()) {
            Ok(_) => {
                // Keep clipboard alive for a moment to ensure managers see it
                std::thread::sleep(std::time::Duration::from_millis(100));
                true
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Get list of available serial ports as friendly strings
pub fn get_available_ports() -> anyhow::Result<Vec<String>> {
    match list_ports() {
        Ok(ports) => {
            let port_names: Vec<String> = ports.iter().map(|p| p.port_name.clone()).collect();
            if port_names.is_empty() {
                Ok(vec!["No ports available".to_string()])
            } else {
                Ok(port_names)
            }
        }
        Err(_) => Ok(vec!["Error reading ports".to_string()]),
    }
}

// ConfigSetting type is internal to the app root; for UI we define a local compatible version
#[derive(Debug, Clone)]
pub struct ConfigSettingRow {
    pub number: i32,
    pub name: String,
    pub value: String,
    pub unit: String,
    pub description: String,
    pub category: String,
    pub read_only: bool,
}

/// Parse a GRBL setting line; adapted for UI view model use
pub fn parse_grbl_setting_line(line: &str) -> Option<ConfigSettingRow> {
    let line = line.trim();
    if !line.starts_with('$') {
        return None;
    }

    let line = &line[1..];
    let parts: Vec<&str> = line.split('=').collect();
    if parts.len() != 2 {
        return None;
    }

    let number = parts[0].parse::<i32>().ok()?;
    let value = parts[1].to_string();
    let (name, desc, unit, category) = get_grbl_setting_info(number);

    Some(ConfigSettingRow {
        number,
        name: name.to_string(),
        value: value.to_string(),
        unit: unit.to_string(),
        description: desc.to_string(),
        category: category.to_string(),
        read_only: false,
    })
}

pub fn get_grbl_setting_info(
    number: i32,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match number {
        // Standard GRBL Settings (0-132)
        0 => ("Step pulse time", "Step pulse duration", "μs", "System"),
        1 => ("Step idle delay", "Step idle delay", "ms", "System"),
        2 => ("Step pulse invert", "Step pulse invert mask", "", "System"),
        3 => (
            "Step direction invert",
            "Step direction invert mask",
            "",
            "System",
        ),
        4 => ("Invert step enable", "Invert step enable pin", "", "System"),
        5 => (
            "Invert limit pins",
            "Invert limit switch pins",
            "",
            "System",
        ),
        6 => ("Invert probe pin", "Invert probe pin", "", "System"),
        10 => ("Status report options", "Status report mask", "", "System"),
        11 => ("Junction deviation", "Junction deviation", "mm", "Motion"),
        12 => ("Arc tolerance", "Arc tolerance", "mm", "Motion"),
        13 => (
            "Report in inches",
            "Report inches instead of mm",
            "",
            "System",
        ),
        20 => ("Soft limits enable", "Enable soft limits", "", "Limits"),
        21 => ("Hard limits enable", "Enable hard limits", "", "Limits"),
        22 => ("Homing cycle enable", "Enable homing cycle", "", "Homing"),
        23 => (
            "Homing direction invert",
            "Homing direction invert mask",
            "",
            "Homing",
        ),
        24 => (
            "Homing locate feed rate",
            "Homing locate feed rate",
            "mm/min",
            "Homing",
        ),
        25 => (
            "Homing search seek rate",
            "Homing search seek rate",
            "mm/min",
            "Homing",
        ),
        26 => (
            "Homing switch debounce delay",
            "Homing switch debounce",
            "ms",
            "Homing",
        ),
        27 => (
            "Homing switch pull-off distance",
            "Homing switch pull-off",
            "mm",
            "Homing",
        ),
        28 => (
            "G73 retract distance",
            "G73 retract distance",
            "mm",
            "Motion",
        ),
        29 => ("Pulse delay", "Pulse delay", "μs", "System"),
        30 => (
            "Maximum spindle speed",
            "Maximum spindle speed",
            "RPM",
            "Spindle",
        ),
        31 => (
            "Minimum spindle speed",
            "Minimum spindle speed",
            "RPM",
            "Spindle",
        ),
        32 => ("Laser mode enable", "Enable laser mode", "", "Laser"),
        33 => (
            "Spindle PWM frequency",
            "Spindle PWM frequency",
            "Hz",
            "Spindle",
        ),
        34 => (
            "Spindle PWM off value",
            "Spindle PWM off value",
            "",
            "Spindle",
        ),
        35 => (
            "Spindle PWM min value",
            "Spindle PWM min value",
            "",
            "Spindle",
        ),
        36 => (
            "Spindle PWM max value",
            "Spindle PWM max value",
            "",
            "Spindle",
        ),
        37 => (
            "Stepper deenergize mask",
            "Stepper deenergize mask",
            "",
            "System",
        ),
        38 => (
            "Spindle encoder pulses per rev",
            "Spindle encoder PPR",
            "",
            "Spindle",
        ),
        39 => (
            "Enable legacy RT commands",
            "Enable legacy RT commands",
            "",
            "System",
        ),
        40 => ("Limit pins invert", "Limit pins invert mask", "", "Limits"),
        41 => ("Probe invert", "Probe invert", "", "Probe"),
        43 => ("Homing passes", "Homing passes", "", "Homing"),
        44 => ("Homing cycle", "Axes homing cycle", "", "Homing"),
        45 => ("Homing cycle 2", "Axes homing cycle 2", "", "Homing"),
        46 => ("Homing cycle 3", "Axes homing cycle 3", "", "Homing"),
        50 => ("Step jog speed", "Step jog speed", "mm/min", "Jogging"),
        51 => ("Slow jog speed", "Slow jog speed", "mm/min", "Jogging"),
        52 => ("Fast jog speed", "Fast jog speed", "mm/min", "Jogging"),
        53 => ("Step jog distance", "Step jog distance", "mm", "Jogging"),
        54 => ("Slow jog distance", "Slow jog distance", "mm", "Jogging"),
        55 => ("Fast jog distance", "Fast jog distance", "mm", "Jogging"),
        56 => ("Parking enable", "Enable parking", "", "Parking"),
        57 => ("Parking axis", "Parking axis", "", "Parking"),
        58 => (
            "Parking pullout increment",
            "Parking pullout increment",
            "",
            "Parking",
        ),
        59 => (
            "Parking pullout rate",
            "Parking pullout rate",
            "mm/min",
            "Parking",
        ),
        60 => ("Restore overrides", "Restore overrides", "", "System"),
        61 => ("Safety door enable", "Enable safety door", "", "Safety"),
        62 => ("Sleep enable", "Enable sleep mode", "", "System"),
        63 => ("Feed hold actions", "Feed hold actions", "", "System"),
        64 => (
            "Force init alarm",
            "Force initialization alarm",
            "",
            "System",
        ),
        65 => (
            "Probe allow feed override",
            "Probe allow feed override",
            "",
            "Probe",
        ),
        70 => ("Network services", "Network services", "", "Network"),
        71 => ("WiFi mode", "WiFi mode", "", "Network"),
        72 => ("Telnet port", "Telnet port", "", "Network"),
        73 => ("WebSocket port", "WebSocket port", "", "Network"),
        74 => ("HTTP port", "HTTP port", "", "Network"),
        75 => ("Bluetooth service name", "Bluetooth name", "", "Network"),
        100 => ("X steps/mm", "X-axis steps per mm", "steps/mm", "Motion"),
        101 => ("Y steps/mm", "Y-axis steps per mm", "steps/mm", "Motion"),
        102 => ("Z steps/mm", "Z-axis steps per mm", "steps/mm", "Motion"),
        103 => ("A steps/mm", "A-axis steps per mm", "steps/mm", "Motion"),
        104 => ("B steps/mm", "B-axis steps per mm", "steps/mm", "Motion"),
        105 => ("C steps/mm", "C-axis steps per mm", "steps/mm", "Motion"),
        110 => ("X max rate", "X-axis max rate", "mm/min", "Motion"),
        111 => ("Y max rate", "Y-axis max rate", "mm/min", "Motion"),
        112 => ("Z max rate", "Z-axis max rate", "mm/min", "Motion"),
        113 => ("A max rate", "A-axis max rate", "mm/min", "Motion"),
        114 => ("B max rate", "B-axis max rate", "mm/min", "Motion"),
        115 => ("C max rate", "C-axis max rate", "mm/min", "Motion"),
        120 => ("X acceleration", "X-axis acceleration", "mm/sec²", "Motion"),
        121 => ("Y acceleration", "Y-axis acceleration", "mm/sec²", "Motion"),
        122 => ("Z acceleration", "Z-axis acceleration", "mm/sec²", "Motion"),
        123 => ("A acceleration", "A-axis acceleration", "mm/sec²", "Motion"),
        124 => ("B acceleration", "B-axis acceleration", "mm/sec²", "Motion"),
        125 => ("C acceleration", "C-axis acceleration", "mm/sec²", "Motion"),
        130 => ("X max travel", "X-axis maximum travel", "mm", "Max Travel"),
        131 => ("Y max travel", "Y-axis maximum travel", "mm", "Max Travel"),
        132 => ("Z max travel", "Z-axis maximum travel", "mm", "Max Travel"),
        133 => ("A max travel", "A-axis maximum travel", "mm", "Max Travel"),
        134 => ("B max travel", "B-axis maximum travel", "mm", "Max Travel"),
        135 => ("C max travel", "C-axis maximum travel", "mm", "Max Travel"),

        // grblHAL Extended Settings (200+)
        200 => ("Trinamic driver", "Trinamic driver enable", "", "Drivers"),
        201 => (
            "Trinamic homing sensitivity",
            "Homing sensitivity",
            "",
            "Drivers",
        ),
        202 => (
            "Trinamic hold current %",
            "Hold current percentage",
            "%",
            "Drivers",
        ),
        203 => (
            "Trinamic run current %",
            "Run current percentage",
            "%",
            "Drivers",
        ),
        204 => (
            "Trinamic HW current %",
            "Hardware current percentage",
            "%",
            "Drivers",
        ),
        205 => (
            "Trinamic step interpolation",
            "Step interpolation enable",
            "",
            "Drivers",
        ),
        206 => (
            "Trinamic PWM chop config",
            "PWM chopper config",
            "",
            "Drivers",
        ),
        207 => ("Trinamic PWM mode", "PWM mode", "", "Drivers"),
        208 => ("Trinamic PWM freq", "PWM frequency", "", "Drivers"),
        209 => ("Trinamic PWM autoscale", "PWM autoscale", "", "Drivers"),
        210 => ("Trinamic PWM autograd", "PWM autograd", "", "Drivers"),

        300 => ("Hostname", "Network hostname", "", "Network"),
        301 => ("IP mode", "IP address mode", "", "Network"),
        302 => ("IP address", "IP address", "", "Network"),
        303 => ("Gateway", "Gateway address", "", "Network"),
        304 => ("Netmask", "Network mask", "", "Network"),
        305 => ("Telnet port", "Telnet port number", "", "Network"),
        306 => ("HTTP port", "HTTP port number", "", "Network"),
        307 => ("WebSocket port", "WebSocket port number", "", "Network"),
        308 => ("FTP port", "FTP port number", "", "Network"),
        309 => ("WiFi SSID", "WiFi SSID", "", "Network"),
        310 => ("WiFi password", "WiFi password", "", "Network"),
        320 => (
            "Bluetooth device name",
            "Bluetooth device name",
            "",
            "Network",
        ),
        321 => (
            "Bluetooth service name",
            "Bluetooth service name",
            "",
            "Network",
        ),

        340 => ("Tool change mode", "Tool change mode", "", "Tool"),
        341 => (
            "Tool change probing distance",
            "Probing distance",
            "mm",
            "Tool",
        ),
        342 => (
            "Tool change locate feed rate",
            "Locate feed rate",
            "mm/min",
            "Tool",
        ),
        343 => (
            "Tool change search seek rate",
            "Search seek rate",
            "mm/min",
            "Tool",
        ),
        344 => (
            "Tool change probe pull-off rate",
            "Probe pull-off rate",
            "mm/min",
            "Tool",
        ),

        370 => ("Invert I/O Port", "Invert I/O port pins", "", "I/O"),
        371 => ("Invert Analog Port 0", "Invert analog port 0", "", "I/O"),
        372 => ("Invert Analog Port 1", "Invert analog port 1", "", "I/O"),

        384 => (
            "Disable G92 persistence",
            "Disable G92 persistence",
            "",
            "Motion",
        ),
        385 => (
            "Dual axis length offset",
            "Dual axis length offset",
            "mm",
            "Motion",
        ),

        395 => ("Startup line 0", "Startup line 0", "", "Startup"),
        396 => ("Startup line 1", "Startup line 1", "", "Startup"),
        397 => ("Startup line 2", "Startup line 2", "", "Startup"),

        481 => ("Autoreport interval", "Autoreport interval", "ms", "System"),

        550 => ("Plasma THC enable", "Enable THC", "", "Plasma"),
        551 => ("Plasma THC mode", "THC mode", "", "Plasma"),
        552 => ("Plasma THC delay", "THC delay", "s", "Plasma"),
        553 => ("Plasma THC threshold", "THC threshold", "V", "Plasma"),
        554 => ("Plasma THC P gain", "THC P gain", "", "Plasma"),
        555 => ("Plasma THC I gain", "THC I gain", "", "Plasma"),
        556 => ("Plasma THC D gain", "THC D gain", "", "Plasma"),
        557 => (
            "Plasma arc voltage scale",
            "Arc voltage scale",
            "",
            "Plasma",
        ),
        558 => (
            "Plasma arc voltage offset",
            "Arc voltage offset",
            "",
            "Plasma",
        ),
        559 => ("Plasma THC VAD threshold", "VAD threshold", "", "Plasma"),
        560 => ("Plasma THC void override", "Void override", "", "Plasma"),

        600 => ("Encoder mode", "Encoder mode", "", "Encoder"),
        601 => ("Encoder CPR", "Encoder counts per rev", "", "Encoder"),
        602 => ("Encoder DPR", "Encoder distance per rev", "mm", "Encoder"),
        603 => (
            "Encoder feed rate",
            "Encoder feed rate",
            "mm/min",
            "Encoder",
        ),

        650 => ("Modbus baud rate", "Modbus baud rate", "", "Modbus"),
        651 => ("Modbus RX timeout", "Modbus RX timeout", "ms", "Modbus"),

        680 => ("RS485 baud rate", "RS485 baud rate", "", "RS485"),

        _ => (
            Box::leak(format!("${}", number).into_boxed_str()),
            "Unknown setting",
            "",
            "Other",
        ),
    }
}

/// Sync firmware capabilities into a `MainWindow` instance
pub fn sync_capabilities_to_ui(window: &MainWindow, capability_manager: &CapabilityManager) {
    let state = capability_manager.get_state();

    // Build capability list for the view
    let mut capabilities = Vec::new();

    capabilities.push(CapabilityItem {
        name: "Arc Support (G2/G3)".into(),
        enabled: state.supports_arcs,
        notes: "Circular interpolation commands".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Variable Spindle (M3/M4 S)".into(),
        enabled: state.supports_variable_spindle,
        notes: "PWM spindle speed control".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Probing (G38.x)".into(),
        enabled: state.supports_probing,
        notes: "Touch probe operations".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Tool Change (M6 T)".into(),
        enabled: state.supports_tool_change,
        notes: "Automatic tool changing".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Homing Cycle ($H)".into(),
        enabled: state.supports_homing,
        notes: "Machine homing to limit switches".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Feed/Spindle Overrides".into(),
        enabled: state.supports_overrides,
        notes: "Real-time adjustment of feed and spindle".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Laser Mode (M3/M4)".into(),
        enabled: state.supports_laser,
        notes: "Dynamic laser power control for engraving/cutting".into(),
    });
    capabilities.push(CapabilityItem {
        name: format!("{} Axes Support", state.max_axes),
        enabled: state.max_axes > 0,
        notes: format!("Maximum {} axes (X,Y,Z,A,B,C)", state.max_axes),
    });
    capabilities.push(CapabilityItem {
        name: format!("{} Coordinate Systems", state.coordinate_systems),
        enabled: state.coordinate_systems > 0,
        notes: "Work coordinate systems (G54-G59)".into(),
    });

    // Update UI
    window.set_device_capabilities(capabilities);
}

/// Update device info panel on the passed MainWindow
pub fn update_device_info_panel(
    window: &MainWindow,
    firmware_type: FirmwareType,
    version: SemanticVersion,
    capability_manager: &CapabilityManager,
) {
    // Update capability manager with detected firmware
    capability_manager.update_firmware(firmware_type, version);

    window.set_device_firmware_type(format!("{:?}", firmware_type));
    window.set_device_firmware_version(version.to_string());
    window.set_device_name(format!("{:?} Device", firmware_type));

    // Build capabilities vector
    let state = capability_manager.get_state();
    let mut capabilities = Vec::new();

    capabilities.push(CapabilityItem {
        name: "Arc Support (G2/G3)".into(),
        enabled: state.supports_arcs,
        notes: "Circular interpolation commands".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Variable Spindle (M3/M4 S)".into(),
        enabled: state.supports_variable_spindle,
        notes: "PWM spindle speed control".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Probing (G38.x)".into(),
        enabled: state.supports_probing,
        notes: "Touch probe operations".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Tool Change (M6 T)".into(),
        enabled: state.supports_tool_change,
        notes: "Automatic tool changing".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Homing Cycle ($H)".into(),
        enabled: state.supports_homing,
        notes: "Machine homing to limit switches".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Feed/Spindle Overrides".into(),
        enabled: state.supports_overrides,
        notes: "Real-time adjustment of feed and spindle".into(),
    });
    capabilities.push(CapabilityItem {
        name: "Laser Mode (M3/M4)".into(),
        enabled: state.supports_laser,
        notes: "Dynamic laser power control for engraving/cutting".into(),
    });

    capabilities.push(CapabilityItem {
        name: format!("{} Axes Support", state.max_axes),
        enabled: state.max_axes > 0,
        notes: format!("Maximum {} axes (X,Y,Z,A,B,C)", state.max_axes),
    });

    capabilities.push(CapabilityItem {
        name: format!("{} Coordinate Systems", state.coordinate_systems),
        enabled: state.coordinate_systems > 0,
        notes: "Work coordinate systems (G54-G59)".into(),
    });

    window.set_device_capabilities(capabilities);
}

/// Update visible lines in the editor view
pub fn update_visible_lines(window: &MainWindow, editor_bridge: &EditorBridge) {
    let (start_line, end_line) = editor_bridge.viewport_range();
    let mut visible_lines = Vec::new();
    for i in start_line..end_line {
        if let Some(content) = editor_bridge.get_line_at(i) {
            visible_lines.push(crate::TextLine {
                line_number: (i + 1) as i32,
                content: content.clone(),
                is_dirty: false,
            });
        }
    }
    window.set_visible_lines(visible_lines);
}
