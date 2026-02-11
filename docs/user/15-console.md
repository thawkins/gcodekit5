# Console

## Overview

The Console panel provides direct communication with your CNC controller. It displays messages from the device and allows you to send manual commands.

## Message Display

Messages are color-coded by level:

| Level | Description |
|-------|-------------|
| **Info** | Standard responses from the controller |
| **Success** | Confirmation messages (e.g., "ok") |
| **Warning** | Non-critical issues |
| **Error** | Failures, faults, or alarm messages |
| **Debug** | Diagnostic messages (hidden by default) |

Each message includes an optional timestamp in HH:MM:SS format.

## Message Filtering

Toggle message levels on and off to focus on what matters:

- Click a level filter button to show/hide that level
- Enable **Debug** to see diagnostic messages
- Filter buttons apply immediately to the current console content

## Sending Commands

Type a command in the input field at the bottom of the console and press **Enter** to send it to the controller.

**Common commands for GRBL**:
- `$$` — View all GRBL settings
- `$#` — View coordinate offsets
- `$I` — View build info
- `?` — Request status report
- `$H` — Home all axes
- `$X` — Clear alarm lock

## Command History

Navigate through previously sent commands using the **Up** and **Down** arrow keys in the command input field, similar to a terminal shell.

## Auto-Scroll

The console auto-scrolls to show the newest messages. Scroll up manually to review history — auto-scroll pauses until you scroll back to the bottom.

## See Also

- [Machine Control](10-machine-control.md) — DRO and manual control
- [Troubleshooting](06-connection-troubleshooting.md) — Diagnosing connection issues
