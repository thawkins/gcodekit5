# Device Config

Device Config is where you **retrieve, inspect, and manage firmware settings** (e.g. GRBL `$$`) and the **capabilities** derived from those settings.

## What you’ll see
- **Device Info (left)**: basic details for the currently connected device (name/firmware/version).
- **Settings (right)**: a searchable, filterable list of `$` settings, grouped by category.

## Retrieve settings (recommended first step)
1. Connect to the machine in **Machine Control**.
2. Click **Retrieve** to read `$$` from the controller.
3. Use the **Search** box and **Category** filter to locate specific settings.

Retrieved settings are stored so other parts of the app can use them (e.g. max travel, max feed rate, spindle/laser parameters).

## Working with settings
### Search + filter
- Use **Search** for setting number, name, or description.
- Use **Category** to quickly jump to areas like motion limits, acceleration, spindle, etc.

### Edit & restore
- Some settings may be **read-only** depending on firmware/controller.
- **Restore** writes the selected/loaded settings back to the connected controller.

## Saving and loading
- **Save** exports the current settings to a file for backup/sharing.
- **Load** imports a settings file so you can compare or restore a known-good configuration.

## Capabilities (derived)
Some UI behaviors are derived from settings, for example:
- Homing enabled/disabled (e.g. GRBL `$22`)
- Laser mode (e.g. GRBL `$32`)

## Safety notes
Changing firmware settings can immediately affect motion, limits, and safety features.
- If you are unsure about a setting, **save a backup** first.
- After changes, validate limits/homing behavior at low speed.

## Related
- [Device Manager](help:device_manager)
- [Device Console](help:device_console)
- [Machine Control](help:machine_control)
- [Index](help:index)

