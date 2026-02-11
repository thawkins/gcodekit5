# Supporting 2-Axis (and Other Non-3-Axis) Devices

## Overview

GCodeKit5 supports CNC devices with fewer than 3 axes. When a device profile
is configured with fewer than 3 axes, all G-code generators automatically
suppress Z-axis positioning commands, producing output suitable for 2-axis
machines such as pen plotters, vinyl cutters, and laser engravers without a
motorised Z stage.

If no device profile is active, or the number of axes is not set, the
application defaults to **3 axes**.

---

## Setting the Number of Axes

1. Open **Device Manager** from the main menu.
2. Select a device from the list, or create a new device profile.
3. Navigate to the **Capabilities** tab.
4. Locate the **"No of Axes"** field (accepts an integer from 1 to 6).
5. Enter the number of axes your device supports (e.g. `2` for an XY-only
   machine).
6. Click **Save** to persist the change.

> **Validation:** The value must be an integer between 1 and 6. An error
> dialog is shown if an invalid value is entered.

---

## Setting the Active Device

The active device profile determines the axis count used by all G-code
generators at generation time.

1. In **Device Manager**, select the device you wish to use.
2. Click **Set as Active** which is found in the top right hand corner of the display panel of the currently selected device.

The application immediately synchronises the selected profile's axis count
to a global state variable. All subsequent G-code generation operations will
read this value.

On application startup, the axis count from the previously active device
profile is restored automatically.

---

## Effect on G-Code Generation

### When Number of Axes ≥ 3 (default behaviour)

All G-axis commands are generated normally, including:

- `G00 Z<safe_z>` — rapid retract to safe height
- `G01 Z<depth> F<rate>` — controlled plunge to cutting depth
- `G01 X… Y… Z… F…` — linear moves with Z component
- `G02/G03 X… Y… Z… I… J… F…` — arc moves with helical Z component
- Inter-shape safe-Z retracts in the designer

### When Number of Axes < 3

The following Z-axis commands are **suppressed** (omitted from the output):

| Command type | 3+ axes output | < 3 axes output |
|---|---|---|
| Safe-Z retract before XY rapid | `G00 Z10.000` | *(omitted)* |
| Rapid move to position | `G00 X… Y… Z10.000` | `G00 X… Y…` |
| Plunge to cutting depth | `G01 Z-1.000 F500` | *(omitted)* |
| Linear move with Z change | `G01 X… Y… Z… F…` | `G01 X… Y… F…` |
| Arc move with helical Z | `G02 X… Y… Z… I… J… F…` | `G02 X… Y… I… J… F…` |
| Footer safe-Z raise | `G00 Z10.000` | *(omitted)* |
| Inter-shape retract (designer) | `G00 Z10.000` | *(omitted)* |

XY motion, spindle control (`M3`/`M5`), feed rates, and program
start/end commands (`M30`) are always generated regardless of axis count.

### Affected Generators

The following camtools and designer generators respect the axis count:

- **Vector Engraver** — suppresses safe-Z and multi-pass Z step-down
- **Bitmap Laser Engraver** — suppresses initial safe-Z positioning
- **Jigsaw Puzzle** — suppresses safe-Z, plunge, retract, and per-pass depth
- **Tabbed Box Maker** — suppresses safe-Z, per-pass depth, and final retract
- **Gerber PCB** — suppresses safe-Z, plunge/retract in polylines and
  alignment holes
- **Designer** — suppresses all Z commands in the core toolpath-to-gcode
  converter and inter-shape retracts

### Tools That Require 3+ Axes

The following tools are inherently Z-dependent and **cannot operate** with
fewer than 3 axes. If launched with an active device that has < 3 axes, they
display an error dialog and refuse to generate:

- **Drill Press** — requires Z-axis for plunge drilling
- **Spoilboard Surfacing** — requires Z-axis for surface milling passes

The error dialog reads:

> **Insufficient Axes**
>
> *"The [tool name] tool requires at least 3 axes (X, Y, Z). The active
> device has fewer than 3 axes configured."*

---

## Technical Details

### Where the Axis Count is Stored

| Layer | Location | Type |
|---|---|---|
| Device profile (persisted) | `DeviceProfile.num_axes` in `gcodekit5-devicedb` | `u8`, default `3` |
| UI model | `DeviceProfileUiModel.num_axes` | `String` |
| Global runtime state | `device_status::ACTIVE_NUM_AXES` in `gcodekit5-ui` | `AtomicU8` |
| Generator parameters | `num_axes` field on each parameter struct | `u8`, default `3` |
| Designer state | `DesignerState.num_axes` | `u8`, default `3` |

### Serialisation Compatibility

The `num_axes` field uses `#[serde(default)]` (directly or via a default
function) on all serialisable parameter structs. Existing saved device
profiles and parameter files that predate this feature will deserialise
with `num_axes = 3`, preserving backward compatibility.

### How the Value Flows

```
DeviceProfile (devices.json)
  → DeviceManager UI (edit/save)
  → set_active_profile()
  → device_status::set_active_num_axes()
  → cam tool UI reads device_status::get_active_num_axes()
  → passes num_axes into generator parameter struct
  → generator checks `if num_axes >= 3` before emitting Z commands
```
