# Application Settings

## Overview

Access settings from the **Settings** panel. Changes take effect immediately unless noted otherwise.

## General Settings

| Setting | Description |
|---------|-------------|
| **Default Units** | Metric (mm) or Imperial (inches) for all displays |
| **Theme** | Light, Dark, or System theme |
| **Auto-connect** | Automatically connect to the last-used device on startup |
| **Reconnect Timeout** | Time in seconds before auto-reconnect attempts (default: 10s) |

## Connection Settings

| Setting | Description |
|---------|-------------|
| **Default Connection Type** | Serial, TCP/IP, or WebSocket |
| **Device Type** | CNC, Laser, or Other — affects laser mode ($32) and UI behavior |

## Display Settings

| Setting | Description |
|---------|-------------|
| **DRO Decimal Places** | Number of decimal places shown in the DRO |
| **Grid Spacing** | Default grid spacing for the Designer and Visualizer |
| **Unit Display** | Show units label next to values |

## Keyboard Shortcuts

All keyboard shortcuts are customizable:

1. Navigate to the **Shortcuts** section in Settings
2. Click on a shortcut entry
3. Press the new key combination
4. The shortcut is remapped immediately

Click **Reset to Defaults** to restore all shortcuts to their default bindings. Conflicts are detected and flagged if two actions share the same key combination.

See [Keyboard Shortcuts Reference](90-shortcuts.md) for the full default list.

## Firmware Settings

When connected to a controller, you can view and modify firmware parameters:

### GRBL Settings

- View all `$` settings with descriptions (e.g., `$110` = X max rate)
- Modify individual settings and click **Apply**
- **Export** settings to a text file for backup
- **Import** settings from a file to restore configuration

### Laser Mode

When the device type is set to **Laser**, GCodeKit5 automatically configures `$32=1` (GRBL laser mode) to enable variable power during motion.

## See Also

- [Tool Library & Materials](60-tool-library.md) — Tool and material management
- [Keyboard Shortcuts](90-shortcuts.md) — Shortcut reference
- [Device Setup](04-device-setup.md) — Connection configuration
