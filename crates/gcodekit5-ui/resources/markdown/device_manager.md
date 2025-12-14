# Device Manager

Device Manager is where you create and maintain **device profiles** (your machine definitions) including connection info, travel limits, and capabilities.

A *device profile* is separate from the live machine connection: it’s a saved record that can be reused and shared.

## Left panel: devices list
- **Search** filters profiles by name/description.
- The profile matching the current connection shows a **Connected** badge.
- **Add Device** creates a new profile.

## Right panel: edit tabs
Device profiles are edited via tabs (e.g. Connection, Work Area, Capabilities).

### Connection
Configure how to reach the controller:
- Serial port and baud rate
- TCP host/port (if supported)
- Connection timeout / auto-reconnect

### Work area / travel limits
Set machine travel limits (X/Y/Z min/max). These are used across the app for:
- Visualizer bounds overlays
- Designer “fit to device” behaviors
- CAM sanity checks

### Capabilities
Enable features that affect UI and toolpaths:
- **Has Spindle** → enables spindle power + max spindle speed fields
- **Has Laser** → enables laser power fields
- Coolant support (if your controller/machine uses it)

## Workflow tips
- Keep one profile marked **Active** as your default machine.
- Use descriptive names (e.g. “Shapeoko 4 XL (GRBL)” or “Laser Diode Rig”).
- If you change physical travel, update limits immediately to keep previews accurate.

## Related
- [Device Config](help:device_config)
- [Machine Control](help:machine_control)
- [Visualizer](help:visualizer)
- [Index](help:index)


