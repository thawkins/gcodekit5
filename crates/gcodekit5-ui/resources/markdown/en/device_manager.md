# Device Manager

Device Manager is where you create and maintain **device profiles** (your machine definitions) including connection info, travel limits, and capabilities.

A *device profile* is separate from the live machine connection: it’s a saved record that can be reused and shared.

## Left panel: devices list
- **Search** filters profiles by name/description.
- The profile matching the current connection shows a **Connected** badge.
- **Add Device** creates a new profile.

## Right panel: edit tabs
Device profiles are edited via tabs (e.g. Connection, Work Area, Capabilities).

### Action buttons
- **💾 Save** - Saves changes to the device profile
- **❌ Cancel** - Discards changes and closes the edit form
- **🗑️ Delete** - Removes the device profile (with confirmation)
- **🔄 Sync from Device** - Updates device information from the currently connected device (only enabled when connected). This reads:
  - Maximum travel dimensions from $130, $131, $132 (X/Y/Z max travel)
  - Max spindle speed from $30
  - Laser mode capability from $32
  - Firmware version information
- **✓ Set Active** - Makes this profile the default device

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
- **Quick setup:** Connect to your device, create a new profile, then use **Sync from Device** to automatically populate dimensions and capabilities from the controller's $$ settings.

## Related
- [Device Config](help:device_config)
- [Machine Control](help:machine_control)
- [Visualizer](help:visualizer)
- [Index](help:index)


