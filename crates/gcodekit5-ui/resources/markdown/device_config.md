# Device Config

Device Config lets you retrieve and edit device firmware settings (e.g. GRBL `$` settings).

## Retrieve settings
- Click **Retrieve** to read all settings from the connected device.
- Retrieved values are stored in the connected device profile so other parts of the app can use them (e.g. spindle min/max).

## Capabilities
Capabilities are derived from settings:
- Homing cycle uses `$22`
- Laser mode uses `$32`

## Related
- [Device Manager](help:device_manager)
- [Device Console](help:device_console)
- [Index](help:index)
