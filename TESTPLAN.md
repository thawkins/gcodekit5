# GCodeKit5 Manual Test Plan

This document provides a comprehensive manual test plan for GCodeKit5, organized by the main UI panels/tabs. Each section contains detailed test cases with preconditions, steps, and expected results.

**Version**: 0.50.2-alpha.0  
**Last Updated**: 2026-02-07

---

## Table of Contents

1. [Connection Panel](#1-connection-panel)
2. [Digital Readout (DRO) Panel](#2-digital-readout-dro-panel)
3. [Jog Control Panel](#3-jog-control-panel)
4. [G-Code Editor Panel](#4-g-code-editor-panel)
5. [Console Panel](#5-console-panel)
6. [Visualizer Panel](#6-visualizer-panel)
7. [Overrides Panel](#7-overrides-panel)
8. [Macros Panel](#8-macros-panel)
9. [Settings Panel](#9-settings-panel)
10. [Designer/CAD Panel](#10-designercad-panel)
11. [CAM Tools](#11-cam-tools)
12. [File Operations](#12-file-operations)
13. [Machine Control](#13-machine-control)
14. [Keyboard Shortcuts](#14-keyboard-shortcuts)
15. [Advanced Features Panel](#15-advanced-features-panel)
16. [Safety Diagnostics Panel](#16-safety-diagnostics-panel)
17. [Help System](#17-help-system)

---

## 1. Connection Panel

### 1.1 Serial Port Discovery

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-001 | Port list refresh | USB serial device connected | 1. Open Connection panel<br>2. Click Refresh button | Port appears in dropdown list with correct name (e.g., /dev/ttyUSB0, COM3) |
| CON-002 | Empty port list | No serial devices connected | 1. Open Connection panel<br>2. Observe port dropdown | Dropdown shows empty or "No ports found" message |
| CON-003 | Hot-plug detection | Application running | 1. Connect USB serial device<br>2. Click Refresh | Newly connected port appears in list |
| CON-004 | Port removal detection | Device connected, shown in list | 1. Disconnect USB device<br>2. Click Refresh | Disconnected port removed from list |

### 1.2 Connection Lifecycle

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-020 | Successful GRBL connection | GRBL device available | 1. Select correct port<br>2. Click Connect | 1. Status shows "Connected"<br>2. Firmware version displayed<br>3. Console shows GRBL startup message |
| CON-021 | Connection failure - wrong port | No device on selected port | 1. Select non-existent port<br>2. Click Connect | Error message displayed, status remains "Disconnected" |
| CON-022 | Graceful disconnect | Successfully connected | 1. Click Disconnect | 1. Status shows "Disconnected"<br>2. Port released (can connect with other app)<br>3. Controls disabled |
| CON-023 | Unexpected disconnect | Connected, then unplug device | 1. While connected, unplug USB | 1. Error notification shown<br>2. Status updates to "Disconnected"<br>3. Application remains stable |
| CON-024 | Reconnect after disconnect | Previously connected | 1. Disconnect<br>2. Reconnect same port | Connection succeeds without issues |

### 1.3 Firmware Detection

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-030 | GRBL detection | GRBL controller connected | 1. Connect to device | Firmware type shows "Grbl", version parsed correctly |
| CON-031 | FluidNC detection | FluidNC controller connected | 1. Connect to device | Firmware type shows "FluidNC" |
| CON-032 | grblHAL detection | grblHAL controller connected | 1. Connect to device | Firmware type shows "grblHAL", network connectivity supported |
| CON-033 | TinyG detection | TinyG controller connected | 1. Connect to device | Firmware type shows "TinyG" |
| CON-034 | g2core detection | g2core controller connected | 1. Connect to device | Firmware type shows "g2core" |
| CON-035 | Smoothieware detection | Smoothieware controller connected | 1. Connect to device | Firmware type shows "Smoothieware" |
| CON-036 | Unknown firmware | Non-standard controller | 1. Connect to device | Firmware shows "Unknown" or prompts for manual selection |

---

## 2. Digital Readout (DRO) Panel

### 2.1 Position Display

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DRO-001 | Machine position display | Connected to controller | 1. Observe DRO panel | Shows X, Y, Z machine positions with 3 decimal places |
| DRO-002 | Work position display | Connected, WCS active | 1. Observe DRO panel | Work position differs from machine position by WCS offset |
| DRO-003 | 4-axis display | 4-axis machine connected | 1. Observe DRO panel | A-axis position displayed alongside X, Y, Z |
| DRO-004 | Position updates | Connected, machine moving | 1. Jog machine<br>2. Observe DRO | Position values update in real-time during movement |
| DRO-005 | Negative position display | Machine at negative coordinates | 1. Move to negative position | Negative values displayed with minus sign |

### 2.2 Unit Display

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DRO-010 | Metric display | Units set to mm | 1. Observe DRO | Values shown in mm, unit label shows "mm" |
| DRO-011 | Imperial display | Units set to inches | 1. Change to imperial units<br>2. Observe DRO | Values converted to inches, unit label shows "in" |
| DRO-012 | Unit toggle | Connected | 1. Toggle units mm ↔ in<br>2. Observe DRO | Values correctly converted between units |

### 2.3 Work Coordinate System (WCS)

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DRO-020 | WCS selection | Connected | 1. Select different WCS (G54-G59) | DRO reflects selected coordinate system offsets |
| DRO-021 | Zero axis | Connected, at non-zero position | 1. Click "Zero X" button | X work position becomes 0, machine position unchanged |
| DRO-022 | Zero all axes | Connected | 1. Click "Zero All" button | All work positions become 0 |
| DRO-023 | Set position manually | Connected | 1. Click on position value<br>2. Enter new value | Work offset adjusted to make current position match entered value |

---

## 3. Jog Control Panel

### 3.1 Axis Movement

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| JOG-001 | Jog X positive | Connected, not alarmed | 1. Click X+ button | Machine moves in positive X direction by step size |
| JOG-002 | Jog X negative | Connected, not alarmed | 1. Click X- button | Machine moves in negative X direction by step size |
| JOG-003 | Jog Y positive | Connected, not alarmed | 1. Click Y+ button | Machine moves in positive Y direction by step size |
| JOG-004 | Jog Y negative | Connected, not alarmed | 1. Click Y- button | Machine moves in negative Y direction by step size |
| JOG-005 | Jog Z positive | Connected, not alarmed | 1. Click Z+ button | Machine moves in positive Z direction by step size |
| JOG-006 | Jog Z negative | Connected, not alarmed | 1. Click Z- button | Machine moves in negative Z direction by step size |
| JOG-007 | Rapid jog movement | Connected, continuous mode | 1. Hold jog button | Machine moves continuously until button released |

### 3.2 Step Size Selection

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| JOG-010 | Step size 0.001 | Jog panel open | 1. Select 0.001 step<br>2. Jog X+ | Machine moves exactly 0.001 mm/in |
| JOG-011 | Step size 0.01 | Jog panel open | 1. Select 0.01 step<br>2. Jog X+ | Machine moves exactly 0.01 mm/in |
| JOG-012 | Step size 0.1 | Jog panel open | 1. Select 0.1 step<br>2. Jog X+ | Machine moves exactly 0.1 mm/in |
| JOG-013 | Step size 1.0 | Jog panel open | 1. Select 1.0 step<br>2. Jog X+ | Machine moves exactly 1.0 mm/in |
| JOG-014 | Step size 10.0 | Jog panel open | 1. Select 10.0 step<br>2. Jog X+ | Machine moves exactly 10.0 mm/in |
| JOG-015 | Step size 100.0 | Jog panel open | 1. Select 100.0 step<br>2. Jog X+ | Machine moves exactly 100.0 mm/in |

### 3.3 Feed Rate Control

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| JOG-020 | Custom jog feed rate | Connected | 1. Enter jog feed rate (e.g., 500)<br>2. Jog axis | Movement uses specified feed rate |
| JOG-021 | Feed rate validation | Connected | 1. Enter invalid feed rate (0, negative, text) | Input rejected or corrected to valid value |
| JOG-022 | Feed rate units | Units set to mm | 1. Enter feed rate | Feed rate interpreted as mm/min |

### 3.4 Safety Features

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| JOG-030 | Jog during alarm | Machine in alarm state | 1. Attempt to jog | Jog commands ignored or error shown |
| JOG-031 | Jog while running program | Program executing | 1. Attempt to jog | Jog commands ignored or error shown |
| JOG-032 | Soft limit prevention | Soft limits enabled | 1. Jog toward soft limit | Movement stops at limit, error/warning shown |

---

## 4. G-Code Editor Panel

### 4.1 File Operations

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| EDT-001 | Open G-code file | Valid .gcode file exists | 1. File → Open<br>2. Select file | File contents displayed in editor with line numbers |
| EDT-002 | Open large file | File > 1MB | 1. Open large G-code file | File loads without freezing, may show progress |
| EDT-003 | Save file | File loaded, modified | 1. File → Save | File saved to disk, modification indicator cleared |
| EDT-004 | Save As | File loaded | 1. File → Save As<br>2. Enter new name | File saved with new name |
| EDT-005 | New file | Editor has content | 1. File → New | Editor cleared, prompts to save if unsaved changes |

### 4.2 Syntax Highlighting

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| EDT-010 | G-code highlighting | File with G-codes loaded | 1. Observe editor | G commands (G0, G1, G2, G3, etc.) highlighted in distinct color |
| EDT-011 | M-code highlighting | File with M-codes loaded | 1. Observe editor | M commands (M3, M5, M8, etc.) highlighted in distinct color |
| EDT-012 | Coordinate highlighting | File with coordinates | 1. Observe editor | X, Y, Z, A, B, C values highlighted |
| EDT-013 | Parameter highlighting | File with F, S codes | 1. Observe editor | F (feed), S (speed), T (tool) parameters highlighted |
| EDT-014 | Comment highlighting | File with comments | 1. Observe editor | Comments (lines starting with ;) highlighted differently |

### 4.3 Search and Replace

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| EDT-020 | Find text | File loaded | 1. Edit → Find<br>2. Enter "G1"<br>3. Click Find | First occurrence highlighted, count shown |
| EDT-021 | Find next | Multiple matches exist | 1. Find text<br>2. Click Find Next | Navigation to next occurrence |
| EDT-022 | Find all | Multiple matches exist | 1. Find text | Total match count displayed |
| EDT-023 | Replace single | Match found | 1. Find "G0"<br>2. Replace with "G00"<br>3. Click Replace | Single occurrence replaced |
| EDT-024 | Replace all | Multiple matches | 1. Find "G1"<br>2. Replace with "G01"<br>3. Click Replace All | All occurrences replaced, count shown |
| EDT-025 | Case sensitivity | Mixed case text | 1. Search with case-sensitive option | Only exact case matches found |

### 4.4 Execution Tracking

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| EDT-030 | Current line indicator | Program running | 1. Start program execution | Current line marked with ▶ indicator |
| EDT-031 | Executed line marking | Program running | 1. Observe during execution | Executed lines marked with ✓ |
| EDT-032 | Auto-scroll during execution | Program running | 1. Observe editor | Editor scrolls to keep current line visible |

---

## 5. Console Panel

### 5.1 Message Display

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-101 | Command echo | Connected | 1. Send command from console | Command echoed in console with timestamp |
| CON-102 | Response display | Connected | 1. Send command | Controller response displayed |
| CON-103 | Error display | Send invalid command | 1. Send "INVALID" | Error response displayed in error color |
| CON-104 | Status message filtering | Connected, polling | 1. Enable status suppression | Status query responses (`<...>`) not shown |
| CON-105 | Timestamp display | Console has messages | 1. Observe timestamps | Each message shows HH:MM:SS timestamp |

### 5.2 Message Filtering

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-110 | Filter by level | Console has mixed messages | 1. Select "Errors only" filter | Only error messages displayed |
| CON-111 | Text search filter | Console has messages | 1. Enter search text<br>2. Observe results | Only matching messages shown |
| CON-112 | Clear filter | Filter active | 1. Clear filter | All messages shown again |
| CON-113 | Verbose mode toggle | Verbose messages exist | 1. Toggle verbose mode | Verbose/debug messages shown/hidden |

### 5.3 Command Entry

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-120 | Send manual command | Connected | 1. Type "$" in command field<br>2. Press Enter | Command sent, response received |
| CON-121 | Command history | Commands sent previously | 1. Press Up arrow | Previous command recalled |
| CON-122 | Command history navigation | Multiple commands in history | 1. Press Up/Down arrows | Navigate through command history |
| CON-123 | History limit | Many commands sent | 1. Send > 100 commands<br>2. Navigate history | Oldest commands removed, max 100 retained |

### 5.4 Console Management

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CON-130 | Clear console | Console has messages | 1. Click Clear button | All messages removed |
| CON-131 | Auto-scroll | Many messages arriving | 1. Observe console | Console auto-scrolls to newest message |
| CON-132 | Manual scroll | Auto-scroll active | 1. Scroll up manually | Auto-scroll pauses, can review history |
| CON-133 | Message capacity | Many messages | 1. Generate > 1000 messages | Oldest messages removed, max 1000 retained |

---

## 6. Visualizer Panel

### 6.1 Toolpath Visualization

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| VIS-001 | Display G-code toolpath | G-code file loaded | 1. Observe visualizer | Toolpath rendered as 3D lines |
| VIS-002 | Rapid moves | G-code has G0 commands | 1. Observe visualizer | Rapid moves shown in distinct color (typically red/orange) |
| VIS-003 | Feed moves | G-code has G1 commands | 1. Observe visualizer | Feed moves shown in different color (typically green/blue) |
| VIS-004 | Arc moves | G-code has G2/G3 | 1. Observe visualizer | Arcs rendered as smooth curves |
| VIS-005 | Large file performance | 100,000+ line file | 1. Load large G-code file | Visualizer renders within 5 seconds, remains responsive |

### 6.2 Camera Controls

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| VIS-010 | Rotate view | Visualizer showing toolpath | 1. Click and drag mouse | View rotates around center point |
| VIS-011 | Zoom in | Visualizer showing toolpath | 1. Scroll mouse wheel up | View zooms in toward center |
| VIS-012 | Zoom out | Visualizer showing toolpath | 1. Scroll mouse wheel down | View zooms out from center |
| VIS-013 | Pan view | Visualizer showing toolpath | 1. Middle-click and drag | View pans without rotation |
| VIS-014 | Reset view | View rotated/zoomed | 1. Click Reset or press Home | Returns to default isometric view |

### 6.3 View Presets

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| VIS-020 | Top view | Visualizer active | 1. Click Top view button | Camera positioned directly above, looking down |
| VIS-021 | Front view | Visualizer active | 1. Click Front view button | Camera positioned in front, looking at XZ plane |
| VIS-022 | Right view | Visualizer active | 1. Click Right view button | Camera positioned at right, looking at YZ plane |
| VIS-023 | Isometric view | Visualizer active | 1. Click Isometric view button | Camera at 45° angle showing all axes |
| VIS-024 | Fit to window | Toolpath loaded | 1. Click Fit button | Toolpath scaled and centered to fill view |

### 6.4 Display Options

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| VIS-030 | Toggle grid | Visualizer active | 1. Toggle grid visibility | Grid appears/disappears |
| VIS-031 | Toggle axes | Visualizer active | 1. Toggle axes visibility | Coordinate axes shown/hidden |
| VIS-032 | Toggle bounding box | Toolpath loaded | 1. Toggle bounding box | Bounding box around toolpath shown/hidden |
| VIS-033 | Toggle tool marker | Connected | 1. Toggle tool marker | Current tool position indicator shown/hidden |
| VIS-034 | Wireframe mode | 3D model loaded | 1. Toggle wireframe mode | Solid surfaces become wireframe |

### 6.5 Execution Visualization

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| VIS-040 | Progress tracking | Program running | 1. Run program<br>2. Observe visualizer | Executed portion highlighted differently |
| VIS-041 | Current position | Connected, machine at position | 1. Observe visualizer | Tool marker shows current machine position |
| VIS-042 | Real-time position update | Machine moving | 1. Jog or run program | Tool marker moves in real-time |

---

## 7. Overrides Panel

### 7.1 Feed Rate Override

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| OVR-001 | Increase feed +10% | Connected | 1. Click +10% button | Feed override increases by 10% |
| OVR-002 | Increase feed +1% | Connected | 1. Click +1% button | Feed override increases by 1% |
| OVR-003 | Decrease feed -1% | Connected | 1. Click -1% button | Feed override decreases by 1% |
| OVR-004 | Decrease feed -10% | Connected | 1. Click -10% button | Feed override decreases by 10% |
| OVR-005 | Reset feed to 100% | Override at non-100% | 1. Click Reset button | Feed override returns to 100% |
| OVR-006 | Feed override limits | Override at limit | 1. Try to exceed 200% or go below 10% | Override clamped to valid range |
| OVR-007 | Feed override display | Override adjusted | 1. Adjust override | Current percentage displayed accurately |

### 7.2 Spindle Speed Override

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| OVR-010 | Increase spindle +10% | Connected, spindle on | 1. Click spindle +10% | Spindle override increases by 10% |
| OVR-011 | Increase spindle +1% | Connected, spindle on | 1. Click spindle +1% | Spindle override increases by 1% |
| OVR-012 | Decrease spindle -1% | Connected, spindle on | 1. Click spindle -1% | Spindle override decreases by 1% |
| OVR-013 | Decrease spindle -10% | Connected, spindle on | 1. Click spindle -10% | Spindle override decreases by 10% |
| OVR-014 | Reset spindle to 100% | Override at non-100% | 1. Click Reset button | Spindle override returns to 100% |
| OVR-015 | Spindle override limits | Override at limit | 1. Try to exceed 200% or go below 10% | Override clamped to valid range |

### 7.3 Rapid Override

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| OVR-020 | Rapid 25% | Connected | 1. Click 25% rapid button | Rapid movements at 25% speed |
| OVR-021 | Rapid 50% | Connected | 1. Click 50% rapid button | Rapid movements at 50% speed |
| OVR-022 | Rapid 100% | Connected | 1. Click 100% rapid button | Rapid movements at full speed |

---

## 8. Macros Panel

### 8.1 Macro Management

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MAC-001 | Create new macro | Macros panel open | 1. Click New Macro<br>2. Enter name "Test Macro"<br>3. Enter G-code "G0 X0 Y0"<br>4. Save | Macro appears in button grid |
| MAC-002 | Edit macro | Existing macro | 1. Right-click macro<br>2. Select Edit<br>3. Modify content<br>4. Save | Macro updated with new content |
| MAC-003 | Delete macro | Existing macro | 1. Right-click macro<br>2. Select Delete<br>3. Confirm | Macro removed from grid |
| MAC-004 | Macro button layout | Multiple macros | 1. Create 8 macros | Macros arranged in grid (default 4 columns) |

### 8.2 Macro Execution

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MAC-010 | Run simple macro | Connected, macro exists | 1. Click macro button | G-code commands executed in order |
| MAC-011 | Run multi-line macro | Macro with multiple lines | 1. Click macro button | All lines executed sequentially |
| MAC-012 | Macro during program | Program running | 1. Click macro button | Macro execution blocked or queued |

### 8.3 Variable Substitution

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MAC-020 | Create macro with variables | Macros panel | 1. Create macro with "G0 X${x} Y${y}"<br>2. Define variable defaults | Macro saved with variable placeholders |
| MAC-021 | Run macro with prompts | Macro has variables | 1. Click macro<br>2. Enter values when prompted | Variables substituted, command executed |
| MAC-022 | Variable default values | Macro has defaults | 1. Click macro<br>2. Accept defaults | Default values used if not overridden |

### 8.4 Macro Import/Export

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MAC-030 | Export macros | Macros exist | 1. Click Export<br>2. Choose location | JSON file saved with all macros |
| MAC-031 | Import macros | JSON file exists | 1. Click Import<br>2. Select file | Macros loaded from file |
| MAC-032 | Import duplicate handling | Macro with same name exists | 1. Import file with duplicate | Prompted to overwrite or rename |

---

## 9. Settings Panel

### 9.1 Connection Settings

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-001 | Auto-connect on startup | Device previously connected | 1. Enable auto-connect<br>2. Restart with device connected | Application connects automatically |
| SET-002 | Connection timeout | Settings open | 1. Set connection timeout<br>2. Attempt connection to unresponsive device | Connection fails after specified timeout |

### 9.2 Display Settings

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-010 | Default units | Settings open | 1. Change default units to Imperial | All displays show inches by default |
| SET-011 | DRO decimal places | Settings open | 1. Change DRO precision | DRO shows selected number of decimal places |
| SET-012 | Theme selection | Settings open | 1. Select Light/Dark/System theme | UI theme changes accordingly |

### 9.3 Keyboard Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-020 | View shortcuts | Settings open | 1. Navigate to Shortcuts section | List of all keyboard shortcuts displayed |
| SET-021 | Customize shortcut | Settings open | 1. Select shortcut<br>2. Press new key combo<br>3. Save | Shortcut remapped to new key combination |
| SET-022 | Reset shortcuts | Custom shortcuts set | 1. Click Reset to Defaults | All shortcuts restored to defaults |
| SET-023 | Shortcut conflict | Existing shortcut | 1. Assign key used by another action | Warning shown about conflict |

### 9.4 Tool Library

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-030 | View tool library | Settings → Tools | 1. Open Tool Library | List of all defined tools shown |
| SET-031 | Add new tool | Tool Library open | 1. Click Add Tool<br>2. Enter details<br>3. Save | New tool added to library |
| SET-032 | Edit tool | Existing tool | 1. Select tool<br>2. Click Edit<br>3. Modify<br>4. Save | Tool updated |
| SET-033 | Delete tool | Existing tool | 1. Select tool<br>2. Click Delete<br>3. Confirm | Tool removed from library |
| SET-034 | Import GTC tools | GTC package available | 1. Click Import<br>2. Select .zip file | Tools imported from GTC catalog |

### 9.5 Materials Library

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-040 | View materials | Settings → Materials | 1. Open Materials Library | List of materials with cutting parameters |
| SET-041 | Add custom material | Materials open | 1. Click Add<br>2. Enter material properties<br>3. Save | Material added to library |
| SET-042 | Edit material parameters | Existing material | 1. Select material<br>2. Modify cutting parameters<br>3. Save | Parameters updated |

### 9.6 Firmware Settings

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SET-050 | Read GRBL settings | Connected to GRBL | 1. Open Firmware Settings<br>2. Click Refresh | All $ settings displayed with current values |
| SET-051 | Modify GRBL setting | Settings displayed | 1. Change $110 (X max rate)<br>2. Click Apply | Setting written to controller, verified |
| SET-052 | Export settings | Settings displayed | 1. Click Export | Settings saved to text file |
| SET-053 | Import settings | Settings file exists | 1. Click Import<br>2. Select file | Settings applied to controller |

---

## 10. Designer/CAD Panel

### 10.1 Shape Creation

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-001 | Draw rectangle | Designer active | 1. Select Rectangle tool<br>2. Click and drag on canvas | Rectangle created with specified dimensions |
| DES-002 | Draw circle | Designer active | 1. Select Circle tool<br>2. Click center, drag radius | Circle created with specified radius |
| DES-003 | Draw line | Designer active | 1. Select Line tool<br>2. Click start, click end | Line created between two points |
| DES-004 | Draw ellipse | Designer active | 1. Select Ellipse tool<br>2. Click and drag | Ellipse created with specified radii |
| DES-005 | Draw polyline | Designer active | 1. Select Polyline tool<br>2. Click multiple points<br>3. Double-click to finish | Polyline with multiple segments created |
| DES-006 | Draw polygon | Designer active | 1. Select Polygon tool<br>2. Specify sides<br>3. Click and drag | Regular polygon created |
| DES-007 | Add text | Designer active | 1. Select Text tool<br>2. Click location<br>3. Enter text | Text object created at location |

### 10.2 Parametric Shapes

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-010 | Create spur gear | Designer active | 1. Select Gear tool<br>2. Enter module, teeth count<br>3. Click OK | Involute spur gear generated |
| DES-011 | Create sprocket | Designer active | 1. Select Sprocket tool<br>2. Enter chain pitch, teeth<br>3. Click OK | Sprocket profile generated |
| DES-012 | Gear parameter validation | Gear dialog | 1. Enter invalid parameters (0 teeth) | Error message, creation blocked |

### 10.3 Selection and Manipulation

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-020 | Select single shape | Shapes on canvas | 1. Click on shape | Shape highlighted with handles |
| DES-021 | Select multiple shapes | Multiple shapes | 1. Ctrl+Click or drag select box | Multiple shapes selected |
| DES-022 | Move shape | Shape selected | 1. Drag selected shape | Shape moves to new location |
| DES-023 | Resize shape | Shape selected | 1. Drag corner handle | Shape resized proportionally |
| DES-024 | Rotate shape | Shape selected | 1. Drag rotation handle | Shape rotates around center |
| DES-025 | Delete shape | Shape selected | 1. Press Delete key | Shape removed from canvas |

### 10.4 Transform Operations

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-030 | Scale shape | Shape selected | 1. Edit → Transform → Scale<br>2. Enter scale factor | Shape scaled uniformly |
| DES-031 | Mirror horizontal | Shape selected | 1. Edit → Transform → Mirror H | Shape mirrored horizontally |
| DES-032 | Mirror vertical | Shape selected | 1. Edit → Transform → Mirror V | Shape mirrored vertically |
| DES-033 | Rotate by angle | Shape selected | 1. Edit → Transform → Rotate<br>2. Enter angle | Shape rotated by exact angle |

### 10.5 Undo/Redo

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-040 | Undo operation | Action performed | 1. Press Ctrl+Z | Last action undone |
| DES-041 | Redo operation | Undo performed | 1. Press Ctrl+Y | Undone action restored |
| DES-042 | Multiple undo | Multiple actions | 1. Press Ctrl+Z multiple times | Actions undone in reverse order |
| DES-043 | Undo after save | File saved | 1. Perform undo | Undo history preserved after save |

### 10.6 File Import

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| DES-050 | Import DXF | DXF file available | 1. File → Import → DXF<br>2. Select file | DXF geometry imported as shapes |
| DES-051 | Import SVG | SVG file available | 1. File → Import → SVG<br>2. Select file | SVG paths imported as shapes |
| DES-052 | Import STL | STL file available | 1. File → Import → STL<br>2. Select file | 3D model imported for CAM processing |
| DES-053 | Import with scale | File with known dimensions | 1. Import file<br>2. Verify dimensions | Geometry scaled correctly to mm |
| DES-054 | Import layer handling | Multi-layer DXF | 1. Import file | Layers preserved or flattened as configured |

---

## 11. CAM Tools

### 11.1 Bitmap Engraving (Laser)

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-001 | Load image | Bitmap tool open | 1. Click Load Image<br>2. Select PNG/JPG | Image displayed in preview |
| CAM-002 | Set dimensions | Image loaded | 1. Enter width/height<br>2. Lock/unlock aspect ratio | Image scales correctly |
| CAM-003 | Power settings | Image loaded | 1. Set min/max power<br>2. Set power scale | Power range applied to output |
| CAM-004 | Bidirectional scanning | Settings configured | 1. Enable bidirectional | Preview shows alternating scan direction |
| CAM-005 | Halftone method | Image loaded | 1. Select Floyd-Steinberg | Image dithered for laser engraving |
| CAM-006 | Generate toolpath | All settings configured | 1. Click Generate | G-code generated, preview shown |
| CAM-007 | Mirror/rotate | Image loaded | 1. Toggle mirror X/Y<br>2. Select rotation | Image transformation reflected in output |

### 11.2 Vector Engraving (Laser)

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-010 | Select shapes | Designer has shapes | 1. Select shapes to engrave | Shapes highlighted for processing |
| CAM-011 | Cut power settings | Shapes selected | 1. Set cut power and speed | Parameters applied to cut paths |
| CAM-012 | Multi-pass cutting | Thick material | 1. Enable multi-pass<br>2. Set number of passes | G-code includes multiple Z passes |
| CAM-013 | Hatching | Closed shapes | 1. Enable hatching<br>2. Set angle and spacing | Fill lines generated inside shapes |
| CAM-014 | Cross-hatching | Hatching enabled | 1. Enable cross-hatch | Second pass at perpendicular angle |

### 11.3 Tabbed Box Maker

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-020 | Box dimensions | Tool open | 1. Enter X, Y, H dimensions<br>2. Enter thickness | Box preview updated |
| CAM-021 | Finger joint settings | Dimensions set | 1. Set finger width<br>2. Set spacing | Joint pattern preview updated |
| CAM-022 | Box type selection | Settings configured | 1. Select "No Top" | Top panel removed from output |
| CAM-023 | Divider settings | Box configured | 1. Add X and Y dividers | Divider panels added to output |
| CAM-024 | Generate box | All settings | 1. Click Generate | G-code and layout preview generated |

### 11.4 Spoilboard Surfacing

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-030 | Surface dimensions | Tool open | 1. Enter width and height | Preview shows surfacing area |
| CAM-031 | Tool diameter | Dimensions set | 1. Enter tool diameter<br>2. Set stepover % | Path spacing calculated correctly |
| CAM-032 | Cutting parameters | Tool configured | 1. Set feed rate, spindle speed, depth | Parameters applied to output |
| CAM-033 | Generate surfacing | All configured | 1. Click Generate | Parallel toolpath generated |

### 11.5 Drill Press

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-040 | Single hole | Tool open | 1. Set hole position and diameter | Single hole preview shown |
| CAM-041 | Peck drilling | Deep hole | 1. Enable peck drilling<br>2. Set peck depth | G-code includes peck cycles |
| CAM-042 | Helical drilling | Large hole | 1. Enable helical<br>2. Set helix angle | Helical toolpath generated |

### 11.6 Jigsaw Puzzle

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-050 | Puzzle grid | Tool open | 1. Set dimensions<br>2. Set pieces across/down | Puzzle grid preview shown |
| CAM-051 | Tab settings | Grid configured | 1. Set tab size<br>2. Set jitter | Randomized tab patterns |
| CAM-052 | Seed control | Settings configured | 1. Change seed value | Different pattern generated |

### 11.7 Spoilboard Grid

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-060 | Grid dimensions | Tool open | 1. Enter grid width and height | Preview shows grid area |
| CAM-061 | Grid spacing | Dimensions set | 1. Set X and Y spacing | Grid lines calculated at specified intervals |
| CAM-062 | Hole parameters | Grid configured | 1. Set hole diameter and depth | Hole positions shown on grid |
| CAM-063 | Generate grid G-code | All configured | 1. Click Generate | G-code with drill operations at each grid point generated |

### 11.8 Gerber Converter (PCB Milling)

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-070 | Load Gerber file | Tool open | 1. Click Load<br>2. Select Gerber file | Gerber layers parsed and displayed in preview |
| CAM-071 | Layer selection | Gerber loaded | 1. Select layer type (TopCopper, BottomCopper, SolderMask, ScreenPrint) | Selected layer displayed for processing |
| CAM-072 | Isolation routing | Copper layer selected | 1. Set tool diameter<br>2. Set isolation width | Isolation toolpath generated around traces |
| CAM-073 | Drill file processing | Drill file available | 1. Load Excellon drill file | Drill positions parsed and shown |
| CAM-074 | Generate PCB G-code | Settings configured | 1. Click Generate | G-code for PCB isolation milling generated |

### 11.9 Speeds and Feeds Calculator

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| CAM-080 | Material selection | Calculator open | 1. Select material from library | Material cutting parameters loaded |
| CAM-081 | Tool parameters | Material selected | 1. Enter tool diameter, flutes, type | Tool parameters applied to calculation |
| CAM-082 | Calculate RPM | Material and tool set | 1. Click Calculate | RPM calculated based on surface speed and tool diameter |
| CAM-083 | Calculate feed rate | RPM calculated | 1. Observe feed rate | Feed rate calculated from RPM, flutes, and chip load |
| CAM-084 | Warnings display | Calculation complete | 1. Observe result | Warnings shown if RPM exceeds machine limits or parameters are out of range |
| CAM-085 | Unit switching | Calculator showing results | 1. Toggle between mm and inch | Values converted correctly between unit systems |

---

## 12. File Operations

### 12.1 Project Files (.gck4)

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| FILE-001 | New project | Application running | 1. File → New Project | Empty canvas, default settings |
| FILE-002 | Save project | Shapes on canvas | 1. File → Save<br>2. Enter filename | .gck4 file created with all shapes |
| FILE-003 | Open project | .gck4 file exists | 1. File → Open<br>2. Select file | Project restored with all shapes |
| FILE-004 | Save modified project | Project modified | 1. Modify shapes<br>2. File → Save | Changes saved to existing file |
| FILE-005 | Unsaved changes warning | Modified project | 1. Attempt to close/new | Prompt to save changes |

### 12.2 G-Code Files

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| FILE-010 | Open G-code | .gcode file exists | 1. File → Open<br>2. Select .gcode | File loaded in editor and visualizer |
| FILE-011 | Save G-code | G-code in editor | 1. File → Save | File saved with correct extension |
| FILE-012 | Export G-code | CAM operation complete | 1. File → Export G-code | Generated G-code saved to file |
| FILE-013 | Recent files | Files previously opened | 1. File → Recent Files | List of recently opened files shown |
| FILE-014 | File encoding | Non-ASCII file | 1. Open file with special characters | Characters displayed correctly (UTF-8 support) |

### 12.3 Import/Export

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| FILE-020 | Import DXF | DXF file available | 1. File → Import → DXF | Geometry imported correctly |
| FILE-021 | Import SVG | SVG file available | 1. File → Import → SVG | Paths converted to shapes |
| FILE-022 | Import STL | STL file available | 1. File → Import → STL | 3D model imported for processing |
| FILE-023 | Export SVG | Shapes on canvas | 1. File → Export → SVG | Shapes exported as SVG paths |
| FILE-024 | Import with units | File in inches | 1. Import file<br>2. Verify scale | Properly converted to mm |

---

## 13. Machine Control

### 13.1 Program Execution

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MCH-001 | Start program | G-code loaded, connected | 1. Click Start/Run | Program begins execution, progress shown |
| MCH-002 | Pause program | Program running | 1. Click Pause (!) | Machine enters Feed Hold, resumes on Resume |
| MCH-003 | Resume program | Program paused | 1. Click Resume (~) | Execution continues from pause point |
| MCH-004 | Stop program | Program running | 1. Click Stop | Execution stops, queue cleared |
| MCH-005 | E-Stop | Any state | 1. Click E-Stop | Immediate stop, alarm state |

### 13.2 Homing and Probing

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MCH-010 | Home all axes | Connected, limit switches | 1. Click Home | Machine homes all axes, position reset |
| MCH-011 | Home single axis | Connected | 1. Click Home X | Only X axis homed |
| MCH-012 | Probe Z | Probe connected, workpiece | 1. Click Probe Z<br>2. Confirm | Z probe cycle runs, WCS Z set |
| MCH-013 | Probe XY | Probe connected | 1. Click Probe XY<br>2. Follow prompts | Corner or edge found, WCS set |

### 13.3 Spindle Control

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MCH-020 | Spindle on CW | Connected, spindle capable | 1. Set speed<br>2. Click Spindle CW | Spindle runs clockwise at speed |
| MCH-021 | Spindle on CCW | Connected | 1. Set speed<br>2. Click Spindle CCW | Spindle runs counter-clockwise |
| MCH-022 | Spindle off | Spindle running | 1. Click Spindle Off | Spindle stops |
| MCH-023 | Spindle speed change | Spindle running | 1. Change speed value | Speed updated in real-time |

### 13.4 Coolant Control

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MCH-030 | Mist on | Connected, coolant capable | 1. Click Mist On | Mist coolant activated |
| MCH-031 | Flood on | Connected | 1. Click Flood On | Flood coolant activated |
| MCH-032 | Coolant off | Coolant running | 1. Click Coolant Off | All coolant stopped |

### 13.5 Alarm Handling

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| MCH-040 | Clear alarm | Machine in alarm | 1. Click Unlock/Reset | Alarm cleared, machine unlocked |
| MCH-041 | Alarm notification | Alarm triggered | 1. Trigger limit switch | Alarm displayed with description |
| MCH-042 | Soft reset | Machine unresponsive | 1. Click Soft Reset (Ctrl+X) | Controller reset, connection maintained |

---

## 14. Keyboard Shortcuts

### 14.1 Navigation Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| KEY-001 | Jog X+ (D key) | Connected, jog panel focused | 1. Press D | Machine jogs X+ by step size |
| KEY-002 | Jog X- (A key) | Connected | 1. Press A | Machine jogs X- by step size |
| KEY-003 | Jog Y+ (W key) | Connected | 1. Press W | Machine jogs Y+ by step size |
| KEY-004 | Jog Y- (S key) | Connected | 1. Press S | Machine jogs Y- by step size |
| KEY-005 | Jog Z+ (Q key) | Connected | 1. Press Q | Machine jogs Z+ by step size |
| KEY-006 | Jog Z- (Z key) | Connected | 1. Press Z | Machine jogs Z- by step size |

### 14.2 File Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| KEY-010 | New file | Application running | 1. Press Ctrl+N | New file/project created |
| KEY-011 | Open file | Application running | 1. Press Ctrl+O | Open file dialog appears |
| KEY-012 | Save file | File open | 1. Press Ctrl+S | File saved |
| KEY-013 | Save As | File open | 1. Press Ctrl+Shift+S | Save As dialog appears |
| KEY-014 | Quit application | Application running | 1. Press Ctrl+Q | Application closes (prompts to save if unsaved changes) |

### 14.3 Edit Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| KEY-020 | Undo | Action performed | 1. Press Ctrl+Z | Action undone |
| KEY-021 | Redo | Undo performed | 1. Press Ctrl+Y | Action redone |
| KEY-022 | Cut | Shape selected | 1. Press Ctrl+X | Shape cut to clipboard |
| KEY-023 | Copy | Shape selected | 1. Press Ctrl+C | Shape copied to clipboard |
| KEY-024 | Paste | Clipboard has shape | 1. Press Ctrl+V | Shape pasted |
| KEY-025 | Delete | Shape selected | 1. Press Delete | Shape deleted |
| KEY-026 | Select all | Shapes on canvas | 1. Press Ctrl+A | All shapes selected |

### 14.4 View Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| KEY-030 | Zoom in | Visualizer active | 1. Press + or Ctrl+= | View zooms in |
| KEY-031 | Zoom out | Visualizer active | 1. Press - or Ctrl+- | View zooms out |
| KEY-032 | Fit to window | Content loaded | 1. Press Ctrl+0 or Home | Content fits window |
| KEY-033 | Reset view | View modified | 1. Press Home | Default view restored |

### 14.5 Machine Control Shortcuts

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| KEY-040 | E-Stop | Any state | 1. Press Escape | Emergency stop triggered, stream stopped |
| KEY-041 | Feed Hold | Program running | 1. Press Space | Program paused (feed hold) |
| KEY-042 | Cycle Start | Program paused | 1. Press ~ | Program resumed |

---

## 15. Advanced Features Panel

### 15.1 Probing

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| ADV-001 | Z probe cycle | Connected, probe wired | 1. Open Advanced Features<br>2. Click Probe Z<br>3. Confirm probe action | Z probe runs, result stored, WCS Z offset updated |
| ADV-002 | Probe result display | Probe completed | 1. Observe probe result | Probed Z height displayed as f64 value |
| ADV-003 | Probe status indicator | Probe in progress | 1. Start probe<br>2. Observe status | Probing active indicator shown during cycle |
| ADV-004 | Probe on all firmware | Connected to any supported firmware | 1. Check probe availability | Probing available on GRBL, grblHAL, FluidNC, TinyG, g2core, Smoothieware |

### 15.2 Soft Limits

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| ADV-010 | View soft limits | Connected to controller | 1. Open Advanced Features<br>2. View soft limits | Current soft limit values displayed for each axis |
| ADV-011 | Soft limit enforcement | Soft limits enabled | 1. Send command beyond limits | Command rejected, error displayed |
| ADV-012 | Soft limit toggle | Connected | 1. Enable/disable soft limits | Setting applied to controller |

### 15.3 Work Coordinate Systems

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| ADV-020 | View WCS offsets | Connected | 1. Open WCS section | All coordinate systems (G54-G59) displayed with offsets |
| ADV-021 | Set WCS offset | Connected | 1. Select coordinate system<br>2. Enter offset values | Offset applied to controller |
| ADV-022 | Switch WCS | Connected | 1. Select different WCS (G54-G59) | Active WCS changes, DRO reflects new offsets |

### 15.4 Command History

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| ADV-030 | View command history | Commands sent previously | 1. Open command history | List of previously sent commands displayed |
| ADV-031 | Replay command | History populated | 1. Select command from history<br>2. Click Send | Command re-sent to controller |

---

## 16. Safety Diagnostics Panel

### 16.1 Emergency Stop

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SAF-001 | E-Stop armed state | Connected, idle | 1. Observe E-Stop indicator | E-Stop shows "Armed" state |
| SAF-002 | E-Stop trigger | Connected | 1. Trigger E-Stop | State changes to "Triggered", machine stops all motion |
| SAF-003 | E-Stop reset | E-Stop triggered | 1. Click Reset | E-Stop goes through "Resetting" to "Armed" state |
| SAF-004 | E-Stop stopped state | E-Stop in stopped mode | 1. Observe state | "Stopped" state displayed, machine locked |

### 16.2 Motion Interlock

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SAF-010 | Interlock active | Safety panel open | 1. Check interlock status | Motion interlock state displayed |
| SAF-011 | Interlock prevents motion | Interlock engaged | 1. Attempt jog or program run | Motion blocked, warning displayed |

### 16.3 Communication Diagnostics

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| SAF-020 | Communication status | Connected | 1. Open Safety Diagnostics | Communication health indicators shown |
| SAF-021 | Buffer status | Connected, streaming | 1. Observe buffer indicators | Buffer fill level and status displayed |
| SAF-022 | Performance metrics | Connected | 1. View performance section | Communication latency and throughput shown |

---

## 17. Help System

### 17.1 Help Browser

| Test ID | Test Case | Preconditions | Steps | Expected Result |
|---------|-----------|---------------|-------|-----------------|
| HLP-001 | Open help browser | Application running | 1. Open Help menu or click Help | Help browser window opens with topic list |
| HLP-002 | Navigate topics | Help browser open | 1. Click on a help topic | Topic content displayed in markdown format |
| HLP-003 | Help link navigation | Topic with links | 1. Click help:topic_id link | Navigates to linked topic within help browser |
| HLP-004 | CAM tool help | CAM tool open | 1. Click Help button on CAM tool | Context-sensitive help for specific CAM tool displayed |
| HLP-005 | Gerber help | Gerber tool open | 1. Access Gerber help | Gerber converter documentation displayed |
| HLP-006 | Speeds/Feeds help | Calculator open | 1. Access calculator help | Speeds and feeds documentation displayed |

---

## Appendix A: Test Environment Setup

### Hardware Requirements
- Computer running Linux (tested on Ubuntu 22.04+)
- USB serial adapter or built-in serial port
- CNC controller (GRBL, grblHAL, FluidNC, TinyG, g2core, Smoothieware, or compatible)
- Optional: 3-axis CNC machine for physical tests
- Optional: Touch probe for probing tests

### Software Requirements
- GCodeKit5 v0.50.2-alpha.0 or later
- Rust toolchain 1.88+ (MSRV)
- GTK4 runtime libraries
- Test G-code files (provided in `test_files/` directory)
- Test images for bitmap engraving

### Test Data Files
- `simple.gcode` - Basic movement commands
- `circles.gcode` - G2/G3 arc commands
- `large.gcode` - 100,000+ line file for performance testing
- `test_image.png` - Grayscale image for engraving
- `test.dxf` - DXF import test file
- `test.svg` - SVG import test file
- `test.stl` - STL 3D model import test file
- `test.gbr` - Gerber PCB file for converter testing

---

## Appendix B: Test Execution Checklist

### Pre-Test Checklist
- [ ] Application installed and launches correctly
- [ ] Test hardware connected (if applicable)
- [ ] Test data files available
- [ ] Console/log monitoring enabled

### Post-Test Checklist
- [ ] All test results recorded
- [ ] Bugs/issues documented with reproduction steps
- [ ] Test environment restored to clean state
- [ ] Logs collected for failed tests

---

## Appendix C: Known Limitations

1. **Physical machine tests** (jogging, homing, spindle, probing) require actual hardware
2. **Performance tests** may vary based on system specifications
3. **Keyboard shortcuts** may conflict with system shortcuts on some platforms
4. **Serial port tests** require appropriate permissions (dialout group on Linux)
5. **Firmware-specific tests** require the corresponding controller hardware (GRBL, grblHAL, FluidNC, TinyG, g2core, Smoothieware)
6. **Gerber converter tests** require sample Gerber/Excellon files

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-30 | AI Assistant | Initial comprehensive test plan |
| 1.1 | 2026-02-07 | AI Assistant | Updated jog shortcuts from Numpad to WASD+QZ keys; added all 6 supported firmware types (grblHAL, TinyG, g2core, Smoothieware); added Spoilboard Grid, Gerber Converter, Speeds/Feeds Calculator CAM tool tests; added Advanced Features Panel (probing, soft limits, WCS, command history); added Safety Diagnostics Panel; added Help System tests; added STL import tests; updated keyboard shortcuts; updated hardware/software requirements |
