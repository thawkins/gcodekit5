# Touch Probe User Guide

## Overview

GCodeKit5 includes comprehensive touch probe support for automated work coordinate setup, tool length measurement, and workpiece inspection. This guide covers the hardware requirements, safety procedures, and step-by-step instructions for each probing routine.

## Table of Contents

1. [Hardware Requirements](#hardware-requirements)
2. [Safety Warnings](#safety-warnings)
3. [Quick Start](#quick-start)
4. [Probe Routines](#probe-routines)
   - [Z-Touch (Surface Probe)](#z-touch-surface-probe)
   - [Edge Find](#edge-find)
   - [Corner Find](#corner-find)
   - [Bore Center](#bore-center)
   - [Boss/Pin Center](#bosspin-center)
   - [Tool Length](#tool-length)
5. [Settings](#settings)
6. [Troubleshooting](#troubleshooting)

---

## Hardware Requirements

### Touch Probe Types

| Type | Description | Connection |
|------|-------------|------------|
| **Touch Probe** | Spring-loaded stylus with electrical contact | Z-probe input pin on controller |
| **Tool Length Setter** | Fixed plate on machine bed | Same Z-probe input or dedicated second pin |
| **3D Taster** | Mechanical or electrical edge finder | May be manual or connected to probe input |

### Controller Compatibility

GCodeKit5 supports probing on the following firmwares:

| Firmware | Probe Support | Notes |
|----------|---------------|-------|
| **GRBL 1.1** | ✅ Full | Standard G38.2/G38.3 commands |
| **grblHAL** | ✅ Full | Extended probe features |
| **FluidNC** | ✅ Full | JSON + WebSocket support |
| **TinyG/g2core** | ✅ Full | JSON probe results |
| **Smoothieware** | ✅ Full | RepRap dialect |

### Wiring

Connect your probe to the controller's Z-probe input:

- **Normally Open (NO)**: Circuit closes when probe touches
- **Normally Closed (NC)**: Circuit opens when probe touches (safer - fail-safe)

> ⚠️ **Important**: Check your controller manual for the correct probe pin. Incorrect wiring can damage the controller or probe.

---

## Safety Warnings

⚠️ **READ BEFORE USING PROBE FUNCTIONS**

1. **Emergency Stop**: Always have your hand near the E-Stop button when running probe cycles
2. **Safe Height**: Always verify safe height is set correctly before probing
3. **Probe Direction**: Ensure probe moves TOWARD the workpiece, not away
4. **Max Depth**: Set reasonable max probe depth to prevent crashes
5. **Retract**: Probe routines include automatic retract - do not interrupt
6. **Spindle**: Ensure spindle is OFF and tool is stopped before probing
7. **Clearance**: Ensure probe stylus has clearance for rapid moves

**Before First Use:**
- Test probe trigger by hand before automated cycles
- Verify probe signal shows in controller status
- Start with slow feed rates

---

## Quick Start

### Basic Z-Touch Probe

1. **Connect** your touch probe to the controller
2. **Jog** to approximately 10mm above the workpiece surface
3. Open **CAM Tools → Probe Tools**
4. Select **Z-Touch** routine
5. Verify safe height (default: 5mm)
6. Click **Probe**
7. The probe will:
   - Move to safe height
   - Probe down until trigger
   - Retract to safe height
   - Show result dialog
8. Click **Apply to WCS** to set Z=0

### Quick Probe from Machine Control

For fast Z-probing:
1. Go to **Machine Control** tab
2. In the DRO panel, click **🔽 Probe Z**
3. Probe runs automatically at current XY position

---

## Probe Routines

### Z-Touch (Surface Probe)

**Purpose**: Find the Z=0 surface of your workpiece

**Parameters**:
| Parameter | Default | Description |
|-----------|---------|-------------|
| Safe Height | 5 mm | Height to rapid above surface |
| Max Depth | 20 mm | Maximum distance to probe down |
| Fast Feed | 100 mm/min | Initial probe speed |
| Slow Feed | 25 mm/min | Re-probe speed for accuracy |
| Backoff | 2 mm | Distance to back off before re-probe |

**Procedure**:
1. Position probe approximately 10mm above surface
2. Run Z-Touch routine
3. Probe touches surface, retracts, re-probes slowly
4. Result shows trigger position
5. Apply to WCS sets Z=0 at surface

**G-code Generated**:
```gcode
G0 Z5      ; Move to safe height
G38.2 Z-20 F100  ; Fast probe down
G0 Z[trigger+2]  ; Back off
G38.2 Z[trigger-0.5] F25  ; Slow re-probe
G0 Z5      ; Retract
```

---

### Edge Find

**Purpose**: Find an edge on X or Y axis with high precision

**Parameters**:
| Parameter | Default | Description |
|-----------|---------|-------------|
| Axis | X | Axis to probe (X, Y, or Z) |
| Direction | Negative | Direction to probe |
| Probe Distance | 10 mm | Distance to probe |
| Fast Feed | 100 mm/min | Initial probe speed |
| Slow Feed | 25 mm/min | Re-probe speed |
| Backoff | 2 mm | Distance to back off |

**Procedure**:
1. Position probe approximately 5mm from edge
2. Select axis and direction
3. Run Edge Find
4. Probe finds edge, backs off, re-probes slowly
5. Result shows edge position

**Tip**: For best accuracy:
- Use slow feed rate for final probe
- Ensure probe is perpendicular to edge
- Clean surface for electrical contact

---

### Corner Find

**Purpose**: Find the intersection of two edges (corner)

**Parameters**:
| Parameter | Default | Description |
|-----------|---------|-------------|
| Corner | X-min/Y-min | Which corner to locate |
| Probe Distance | 10 mm | Distance along each edge |
| Fast Feed | 100 mm/min | Initial probe speed |
| Slow Feed | 25 mm/min | Re-probe speed |

**Procedure**:
1. Position probe inside corner area
2. Select corner type (e.g., X-min/Y-min)
3. Run Corner Find
4. Probe finds X edge, then Y edge
5. Computed corner shown in results

**Corner Types**:
- X-min/Y-min: Front-left corner
- X-max/Y-min: Front-right corner  
- X-min/Y-max: Back-left corner
- X-max/Y-max: Back-right corner

---

### Bore Center

**Purpose**: Find the center of an internal circular feature (hole)

**Parameters**:
| Parameter | Default | Description |
|-----------|---------|-------------|
| Diameter | 20 mm | Approximate bore diameter |
| Safe Height | 10 mm | Height for rapid moves |
| Fast Feed | 100 mm/min | Probe speed |
| Slow Feed | 25 mm/min | Not used (4-point method) |

**Procedure**:
1. Position probe approximately at bore center
2. Enter approximate bore diameter
3. Run Bore Center
4. Probe touches 4 points: left, right, front, back
5. Center computed from chord intersections

**Accuracy**: ±0.01mm with proper calibration

---

### Boss/Pin Center

**Purpose**: Find the center of an external circular feature (pin/boss)

**Parameters**: Same as Bore Center

**Procedure**:
1. Position probe outside the boss
2. Enter approximate boss diameter
3. Run Boss Center
4. Probe touches 4 points from outside
5. Center computed and displayed

**Difference from Bore Center**: Probe approaches from outside rather than inside.

---

### Tool Length

**Purpose**: Measure tool length using a fixed setter plate

**Requirements**:
- Fixed setter plate at known Z height
- Plate position configured in settings

**Parameters**:
| Parameter | Default | Description |
|-----------|---------|-------------|
| Setter Plate X | 100 mm | X position of setter plate |
| Setter Plate Y | 100 mm | Y position of setter plate |
| Setter Plate Z | -50 mm | Known Z height of plate surface |
| Safe Height | 10 mm | Height for rapid moves |
| Max Depth | 20 mm | Maximum probe depth |
| Feed Rate | 100 mm/min | Probe speed |

**Procedure**:
1. Install tool to measure
2. Run Tool Length routine
3. Machine moves to setter plate position
4. Probes down until trigger
5. Tool length = trigger Z - setter plate Z

**After Measurement**:
- Apply G43.1 tool length offset
- Or use for tool table entry

---

## Settings

### Probe Defaults

Access via **Settings → Probing**:

| Setting | Default | Description |
|---------|---------|-------------|
| Default Fast Feed | 100 mm/min | Default fast probe rate |
| Default Slow Feed | 25 mm/min | Default slow probe rate |
| Default Safe Height | 5 mm | Default retract height |
| Default Backoff | 2 mm | Default backoff distance |
| Auto-Update WCS | Off | Automatically apply probe results |
| Target WCS | G54 | Default WCS for updates |

### Setter Plate Configuration

Configure your tool length setter plate:

| Setting | Description |
|---------|-------------|
| Setter Plate X | Machine X coordinate |
| Setter Plate Y | Machine Y coordinate |
| Setter Plate Z | Known Z height (machine coords) |

---

## Troubleshooting

### Probe Does Not Trigger

| Symptom | Cause | Solution |
|---------|-------|----------|
| Probe moves to max depth | No electrical contact | Check wiring, clean probe tip |
| No status in app | Probe not detected | Check controller configuration |
| Intermittent trigger | Poor contact | Clean surface, check probe spring |

### Incorrect Positions

| Symptom | Cause | Solution |
|---------|-------|----------|
| Position offset | Probe tip diameter | Use tip compensation |
| Inconsistent results | Machine backlash | Enable backlash compensation |
| Wrong axis | Wrong routine selected | Verify axis selection |

### Error Messages

| Error | Meaning | Solution |
|-------|---------|----------|
| "Probe failed" | No trigger before max depth | Check probe, increase max depth |
| "Probe stuck" | Probe still triggered after retract | Manually free probe, check wiring |
| "Timeout" | No response from controller | Check connection |

---

## Tips for Best Results

1. **Calibration**: Calibrate probe tip diameter in your controller
2. **Cleanliness**: Keep probe tip and workpiece clean
3. **Speed**: Use slower feeds for better accuracy
4. **Repetition**: Run routine 2-3 times and average results
5. **Temperature**: Allow machine to warm up before precision work
6. **Documentation**: Record probe results for repeat setups

---

## Language Support

Probing interface supports:
- English
- German (Deutsch)
- Spanish (Español)
- French (Français)
- Italian (Italiano)
- Portuguese (Português)

Tooltips are available for all parameters in supported languages.

---

## Safety Checklist

Before each probing session:

- [ ] Spindle is OFF
- [ ] Emergency stop is accessible
- [ ] Safe height is appropriate
- [ ] Max depth won't cause crash
- [ ] Probe is clean and functioning
- [ ] Workpiece is secure
- [ ] Clearance for rapid moves verified

---

*Last updated: 2026-05-08*
*Version: 0.55.0-alpha.7*
