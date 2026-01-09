# grblHAL and FluidNC Support

## Overview

GCodeKit5 now supports grblHAL and FluidNC CNC controllers in addition to standard GRBL. Both controllers support network connectivity (TCP/telnet) as well as traditional serial/USB connections.

## Supported Controllers

### GRBL
- **Type**: Standard GRBL 1.1
- **Connection**: Serial/USB
- **Settings**: Writable via `$nn=value` commands
- **Max Axes**: 5
- **Network**: Not supported

### grblHAL
- **Type**: Enhanced GRBL with additional features
- **Connection**: Serial/USB and Network (Telnet, default port 23)
- **Settings**: Writable via `$nn=value` commands
- **Max Axes**: 6
- **Network**: Yes (TCP/telnet, WebSocket)
- **Features**: Plugin support, SD card/filesystem, enhanced status reporting
- **Simulator**: Available (`grblHAL_sim`)

### FluidNC
- **Type**: Modern ESP32-based CNC controller
- **Connection**: Serial/USB and Network (Telnet, WebSocket, default port 23)
- **Settings**: **READ-ONLY** (configured via YAML files)
- **Max Axes**: 6
- **Network**: Yes (WiFi/TCP, WebSocket)
- **Features**: Web interface, file system, WiFi connectivity

## Key Differences

### Settings Management

#### GRBL & grblHAL
- Settings are writable at runtime
- Use `$$` to view settings
- Use `$nn=value` to modify settings (e.g., `$100=250.0`)
- Settings persist in EEPROM/flash
- UI allows editing of all parameters

#### FluidNC
- Settings are **READ-ONLY** at runtime
- Use `$$` to view current settings
- Settings must be changed in `config.yaml` file
- Upload new config file via web interface or SD card
- UI displays parameters but disables editing
- Attempting to write settings returns an error

## Connection Types

### Serial/USB Connection
```rust
let params = ConnectionParams {
    driver: ConnectionDriver::Serial,
    port: "/dev/ttyUSB0".to_string(), // or COM3 on Windows
    baud_rate: 115200,
    timeout_ms: 5000,
    ..Default::default()
};
```

### Network/TCP Connection
```rust
let params = ConnectionParams {
    driver: ConnectionDriver::Tcp,
    port: "192.168.1.100".to_string(), // IP address
    network_port: 23, // Telnet port
    timeout_ms: 5000,
    ..Default::default()
};
```

## Device Profile Configuration

### Example: grblHAL Device (Serial)
```json
{
  "id": "uuid-here",
  "name": "My grblHAL CNC",
  "controller_type": "GrblHal",
  "connection_type": "Serial",
  "port": "/dev/ttyUSB0",
  "baud_rate": 115200,
  "tcp_host": "",
  "tcp_port": 23,
  ...
}
```

### Example: grblHAL Device (Network)
```json
{
  "id": "uuid-here",
  "name": "My grblHAL CNC (WiFi)",
  "controller_type": "GrblHal",
  "connection_type": "Tcp",
  "port": "192.168.1.100",
  "baud_rate": 115200,
  "tcp_host": "192.168.1.100",
  "tcp_port": 23,
  ...
}
```

### Example: FluidNC Device (Network)
```json
{
  "id": "uuid-here",
  "name": "My FluidNC CNC",
  "controller_type": "FluidNC",
  "connection_type": "Tcp",
  "port": "192.168.1.150",
  "tcp_host": "192.168.1.150",
  "tcp_port": 23,
  ...
}
```

## Capabilities

### New Capability Flags

#### SettingsWritable
- `true` for GRBL and grblHAL
- `false` for FluidNC
- Controls whether UI allows editing firmware settings
- Checked before attempting to send `$nn=value` commands

#### NetworkConnectivity
- `true` for grblHAL and FluidNC
- `false` for standard GRBL
- Enables network connection options in UI

## UI Behavior

### Settings Dialog

#### For GRBL/grblHAL:
- All settings are editable
- "Save" button sends `$nn=value` commands to controller
- Changes are immediately persisted in controller EEPROM
- Modified parameters are highlighted
- "Reset to Defaults" available

#### For FluidNC:
- All settings are **read-only** (grayed out)
- Settings display shows current configuration
- Tooltip explains: "FluidNC settings are read-only. Modify config.yaml file."
- "Save" button is disabled
- Users directed to web interface for configuration

### Device Configuration

When selecting controller type:
- **GRBL**: Standard serial connection options
- **grblHAL**: Serial + Network options available
- **FluidNC**: Serial + Network options available, settings marked as read-only

## Testing

### grblHAL Simulator

The grblHAL simulator is installed and available for testing:

**Start simulator:**
```bash
./scripts/start-grblhal-sim.sh
```

**Connect to:**
- Path: `~/Projects/gcodekit5/target/temp/ttyGRBL`
- Baud: Any (virtual serial)

**Stop simulator:**
```bash
./scripts/stop-grblhal-sim.sh
```

**Test commands:**
```bash
./scripts/test-grblhal-sim.sh
```

### Network Testing

For grblHAL network mode:
```bash
grblHAL_sim -p 23  # Start on port 23 (telnet)
```

Then connect via TCP:
```bash
telnet localhost 23
```

## Code Examples

### Detecting Controller Type

```rust
use gcodekit5_communication::firmware::FirmwareDetector;

let detector = FirmwareDetector::new();
if let Ok(Some(info)) = detector.detect_from_response("[VER:1.1f.20260107:]\n[FIRMWARE:grblHAL]") {
    match info.controller_type {
        ControllerType::GrblHal => {
            println!("grblHAL detected!");
            // Enable network options, settings are writable
        },
        ControllerType::FluidNC => {
            println!("FluidNC detected!");
            // Enable network options, settings are READ-ONLY
        },
        _ => {}
    }
}
```

### Checking Settings Capability

```rust
use gcodekit5_communication::firmware::{Capability, CapabilitiesTrait};

let capabilities = get_controller_capabilities();

if capabilities.has_capability(Capability::SettingsWritable) {
    // Show editable settings UI
    enable_settings_editor();
} else {
    // Show read-only settings, disable editing
    disable_settings_editor();
    show_readonly_message("Settings are read-only. Use web interface or config file.");
}

if capabilities.has_capability(Capability::NetworkConnectivity) {
    // Show TCP/WiFi connection options
    enable_network_options();
}
```

### Loading Firmware Settings

```rust
use gcodekit5_ui::ui::firmware_integration::FirmwareSettingsIntegration;

let mut integration = FirmwareSettingsIntegration::new("Unknown", "Unknown");

match controller_type {
    ControllerType::Grbl => {
        integration.load_grbl_defaults()?;
        // All parameters are editable
    },
    ControllerType::GrblHal => {
        integration.load_grblhal_defaults()?;
        // All parameters are editable (GRBL-compatible)
    },
    ControllerType::FluidNC => {
        integration.load_fluidnc_defaults()?;
        // All parameters are read-only
    },
    _ => {}
}
```

## Migration Guide

### Existing GRBL Devices
- No changes required
- GRBL devices continue to work as before
- All existing settings and configurations preserved

### Upgrading to grblHAL
1. Flash grblHAL firmware to your controller
2. In GCodeKit5, edit device profile
3. Change controller type to "grblHAL"
4. Optionally enable network connection if supported
5. Settings management works the same way as GRBL

### Adding FluidNC Device
1. Create new device profile
2. Set controller type to "FluidNC"
3. Choose connection type (Serial or TCP)
4. For network: Enter IP address and port (default 23)
5. Note: Settings cannot be modified from GCodeKit5
6. Use FluidNC web interface for configuration changes

## Troubleshooting

### "Settings are read-only" Error
- This is expected for FluidNC controllers
- Modify settings via FluidNC web interface or config.yaml
- GCodeKit5 displays current settings but cannot modify them

### Network Connection Failed
- Verify controller is on same network
- Check IP address and port number
- Ensure firewall allows connection on specified port
- Test with telnet: `telnet <ip> <port>`

### grblHAL Not Detected
- Verify firmware version string contains "grblHAL"
- Check that controller responds to `$I` command
- Firmware may need update to latest grblHAL version

## References

- [grblHAL GitHub](https://github.com/grblHAL)
- [grblHAL Documentation](https://github.com/grblHAL/core)
- [FluidNC GitHub](https://github.com/bdring/FluidNC)
- [FluidNC Documentation](http://wiki.fluidnc.com/)
