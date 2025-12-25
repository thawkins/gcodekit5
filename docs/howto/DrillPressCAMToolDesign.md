# Drill Press CAMTool Design

## Overview
The Drill Press CAMTool is designed to provide a simple yet powerful interface for drilling operations on GRBL-based CNC machines. It bridges the gap between simple manual drilling and complex CAM software by providing specialized cycles for common drilling tasks.

## Features
- **Top and Bottom Z**: Precise control over where the hole starts and ends.
- **Peck Drilling**: Graduated plunge depth to prevent tool overheating and clear chips.
- **Helical Interpolation**: Support for holes larger than the tool diameter using spiral toolpaths.
- **Customizable Speeds**: Independent plunge and cutting feed rates.
- **Spindle Control**: Integrated spindle speed management.

## Parameters

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `Hole Diameter` | Length | The final diameter of the hole. |
| `Tool Diameter` | Length | The diameter of the actual tool being used. |
| `Top Z` | Length | The Z coordinate of the material surface. |
| `Bottom Z` | Length | The final depth of the hole. |
| `Peck Depth` | Length | The maximum depth of each plunge before retracting. |
| `Plunge Rate` | Speed | The feed rate for vertical movement (Z). |
| `Feed Rate` | Speed | The feed rate for horizontal movement (XY) during helical cycles. |
| `Spindle Speed` | RPM | The rotation speed of the spindle. |
| `Safe Z` | Length | The height at which the tool can safely move between locations. |

## Toolpath Strategies

### 1. Simple Drill Cycle
Used when `Tool Diameter` equals `Hole Diameter` and `Peck Depth` is 0.
- Rapid to `Safe Z`.
- Rapid to `X, Y`.
- Feed to `Bottom Z` at `Plunge Rate`.
- Rapid to `Safe Z`.

### 2. Peck Drill Cycle
Used when `Tool Diameter` equals `Hole Diameter` and `Peck Depth` > 0.
- Rapid to `Safe Z`.
- Rapid to `X, Y`.
- Loop:
    - Feed to `Current Depth + Peck Depth`.
    - Rapid to `Top Z` (to clear chips).
    - Rapid to `Current Depth - 1mm` (clearance).
- Rapid to `Safe Z`.

### 3. Helical Cycle
Used when `Tool Diameter` < `Hole Diameter`.
- Calculate radius: `(Hole Diameter - Tool Diameter) / 2`.
- Rapid to `Safe Z`.
- Rapid to `X + Radius, Y`.
- Spiral down to `Bottom Z` using G2/G3 arcs.
- Perform one full circle at `Bottom Z`.
- Move to center and retract.

## Implementation Steps

1. **Generator Implementation**:
   - Create `crates/gcodekit5-camtools/src/drill_press.rs`.
   - Implement `DrillPressGenerator` with `generate()` returning `Result<String>`.

2. **UI Integration**:
   - Add `DrillPressTool` to `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs`.
   - Create input fields for all parameters using `libadwaita` components.
   - Implement unit conversion logic for Metric/Imperial switching.

3. **Direct Execution & Auto-Centering**:
   - Update `MachineControlView` to expose `start_job(gcode: &str)` and `emergency_stop()`.
   - Update `DrillPressTool` to use "Drill" and "eStop" buttons in the sidebar for direct execution.
   - Implement dynamic UI visibility: hide action buttons and show a "Device Offline" message when the machine is disconnected.
   - Implement auto-centering by fetching machine limits ($130, $131) on initialization.

4. **Testing**:
   - Add unit tests for G-code output verification.
   - Verify helical interpolation math.
