# 6-Axis CNC Support Guide

## Overview

GCodeKit5 provides comprehensive support for 6-axis CNC machines, enabling advanced machining operations including simultaneous 5-axis milling, complex rotary table operations, and full trunnion table support.

## Supported Axes

| Axis | Name | Rotation Plane | Typical Use |
|------|------|----------------|-------------|
| X | Linear | - | Horizontal movement (left/right) |
| Y | Linear | - | Horizontal movement (front/back) |
| Z | Linear | - | Vertical movement (up/down) |
| A | Rotary | Around X-axis | Rotary table tilt |
| B | Rotary | Around Y-axis | Trunnion/tilt head |
| C | Rotary | Around Z-axis | Rotary table spin |

## Getting Started

### Prerequisites

- GCodeKit5 version 0.56.0-alpha or later
- 6-axis capable CNC controller (GRBL, grblHAL, TinyG, g2core, FluidNC, or Smoothieware)
- Rotary axes properly configured on your controller

### Device Configuration

1. **Select Your Device**: Go to Devices → Device Manager
2. **Configure Axis Count**: Set the number of axes your machine supports (4, 5, or 6)
3. **Set Axis Limits**: Configure travel limits for each axis:
   - Linear axes (X/Y/Z): Set in millimeters
   - Rotary axes (A/B/C): Set in degrees (typically 0-360° or 0-9999 for continuous rotation)

### Dynamic UI

GCodeKit5 automatically adapts the UI based on your configured axis count:

- **3-axis machines**: Standard X/Y/Z controls
- **4-axis machines**: Adds A-axis (rotary table) controls
- **5-axis machines**: Adds A and B-axis controls
- **6-axis machines**: Full X/Y/Z/A/B/C control

## Jogging Rotary Axes

### Step Sizes

Rotary axes use degree-based step sizes:

| Step Size | Description |
|-----------|-------------|
| 0.1° | Fine adjustment |
| 0.5° | Small increments |
| 1° | Standard increment |
| 5° | Medium moves |
| 10° | Large moves |
| 45° | Quick positioning |
| 90° | Quarter rotation |

### G-Code Generation

Rotary jog commands are generated using relative positioning:

```gcode
G91 G21 A10 F500   ; Rotate A axis +10 degrees at 500 units/min
G91 G21 B-5 F500   ; Rotate B axis -5 degrees
```

## Work Coordinate Systems

6-axis machines support work coordinate system offsets for all axes:

- **G54-G59**: Standard work coordinate systems
- **G92**: Temporary offset (all 6 axes)
- **G10 L2 Pn**: Persistent offset storage

Example setting WCS with rotary offsets:

```gcode
G10 L2 P1 X0 Y0 Z0 A0 B90 C0   ; Set G54 with B at 90°
```

## Visualizer

The 2D/3D visualizer supports 6-axis toolpaths:

- **Rotary moves**: Shown with distinct colors
- **Helical interpolation**: Visualized as spiral paths
- **Bounds tracking**: Includes all 6 axes in calculations
- **Projection modes**: Different views for rotary machining

## Settings

### Machine Settings

Access via Settings → Machine:

- **Jog Step Size (Rotary)**: Default step for A/B/C axes (degrees)
- **Axis Limits**: Maximum travel for each axis
- **Steps per Degree**: Stepper configuration for rotary axes
- **Direction Inversion**: Reverse rotation direction per axis
- **Calibration Offsets**: Compensate for mechanical errors

### Preset Configurations

GCodeKit5 includes presets for common 6-axis setups:

- **3-Axis Mill**: Standard XYZ
- **4-Axis Mill**: XYZ + A rotary
- **5-Axis Mill**: XYZ + A/B trunnion
- **6-Axis Mill**: Full XYZABC
- **Desktop 6-Axis**: Compact machines (e.g., PocketNC)
- **Industrial 6-Axis**: Large machining centers

## G-Code Compatibility

### Supported Commands

All standard G-code commands work with 6-axis coordinates:

```gcode
G0 X10 Y20 Z5 A45 B0 C0        ; Rapid move
G1 X10 Y20 Z5 A45 B22.5 C0 F1000  ; Linear move
G2 X20 Y10 I5 J0 A90 B45 C0   ; Clockwise arc
G3 X10 Y20 I-5 J10 A0 B0 C180 ; Counter-clockwise arc
```

### Helical Interpolation

6-axis machines support helical moves with simultaneous rotation:

```gcode
G2 X50 Y0 Z-5 I25 J0 A360      ; 360° helix while cutting
```

## Troubleshooting

### Common Issues

**Issue**: Rotary axis doesn't move
- **Check**: Verify axis is enabled in device configuration
- **Check**: Confirm steps per degree is set correctly
- **Check**: Ensure axis limits aren't exceeded

**Issue**: Incorrect rotary positioning
- **Check**: Calibration offset values
- **Check**: Direction inversion settings
- **Check**: Work coordinate system offsets

**Issue**: Visualizer shows wrong toolpath
- **Check**: G-code includes A/B/C coordinates
- **Check**: Visualizer bounds include rotary axes

## Advanced Features

### Continuous Rotation

Set axis limit to 9999° for continuous rotation:

- Useful for indexing operations
- Allows multiple full rotations
- No need to unwind between operations

### Kinematics (g2core)

For g2core controllers, advanced kinematics are supported:

- **Forward kinematics**: Position from joint angles
- **Inverse kinematics**: Joint angles from position
- **Configuration**: Set via g2core settings

## Firmware-Specific Notes

### GRBL
- Maximum 6 axes supported
- Status reports include A/B/C positions
- Use `$10=3` to enable verbose status

### grblHAL
- Native 6-axis support
- Plugin system for custom kinematics
- Network connectivity available

### TinyG/g2core
- JSON status reports
- Automatic axis detection
- Built-in kinematics support

### FluidNC
- YAML configuration
- Web interface for settings
- WiFi and Ethernet support

### Smoothieware
- Config file-based setup
- Up to 6 axes supported
- Smooth motion planning

## Best Practices

1. **Start Small**: Begin with 4-axis before attempting full 6-axis
2. **Test Limits**: Verify axis limits match your machine capabilities
3. **Use Presets**: Start with built-in presets and customize
4. **Save Configurations**: Export settings for backup
5. **Monitor Temperature**: Rotary axes can heat up during continuous operation
6. **Lubrication**: Keep rotary table bearings properly lubricated

## References

- [GRBL Documentation](https://github.com/gnea/grbl/wiki)
- [grblHAL Features](https://github.com/grblHAL/core/wiki)
- [TinyG/G2Core Wiki](https://github.com/synthetos/g2/wiki)
- [FluidNC Documentation](https://github.com/bdring/FluidNC/wiki)

## Support

For issues or questions about 6-axis support:

1. Check this documentation first
2. Review the [6AXISPLAN.md](../6AXISPLAN.md) for implementation details
3. File an issue on GitHub with your machine configuration

---

*Last Updated: 2025-01-20*
*Version: 0.56.0-alpha*
