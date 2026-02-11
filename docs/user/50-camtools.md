# CAM Tools

GCodeKit5 includes a suite of specialized CAM tools for generating G-code directly, without needing external CAM software.

## Using CAM Tools

1. Open a CAM tool from the **CAM Tools** tab
2. Configure the parameters for your project
3. Click **Generate G-Code**
4. Preview the toolpath in the visualizer
5. Send to machine or save to file

Each CAM tool also has a **Help** button that opens context-sensitive documentation.

---

## Tabbed Box Maker

Generate laser or CNC cut boxes with finger joints (tab-and-slot construction).

**Parameters**:
- Box dimensions (width, height, depth)
- Material thickness
- Finger/tab width and spacing
- Box type (open top, closed, with dividers)
- X and Y divider count

**Output**: G-code for cutting all box panels with interlocking finger joints, laid out flat.

---

## Jigsaw Puzzle Maker

Create laser-cut jigsaw puzzles with randomized interlocking tabs.

**Parameters**:
- Puzzle dimensions (width, height)
- Number of pieces across and down
- Tab size and jitter (randomization)
- Random seed (change for different patterns)

**Output**: G-code for cutting the puzzle piece grid.

---

## Drill Press

Emulate a drill press on your CNC with multiple drilling strategies.

**Drilling Modes**:
- **Standard drilling** — Single plunge to depth
- **Peck drilling** — Incremental plunges with retract for chip clearing
- **Helical interpolation** — Spiral milling for holes larger than the tool diameter

**Parameters**:
- Hole position (X, Y)
- Hole diameter, top Z, bottom Z
- Tool diameter
- Peck depth (for peck drilling)
- Helix angle (for helical mode)
- Plunge rate, feed rate, spindle speed, safe Z height

**Import/Export**: Save and load drill parameters as JSON.

---

## Laser Bitmap Engraver

Convert raster images (PNG, JPEG, BMP) to G-code for laser engraving.

**Parameters**:
- **Image**: Width (mm), rotation, mirror X/Y
- **Halftone method**: Threshold, ordered dither, or error diffusion (Floyd-Steinberg)
- **Laser**: Feed rate, travel rate, min/max power, power scaling
- **Raster**: Pixels per mm, line spacing, bidirectional scanning
- **Positioning**: Offset X, offset Y

**Scan directions**: Left-to-right, right-to-left, top-to-bottom.

**Output**: G-code with variable laser power based on pixel intensity.

---

## Laser Vector Engraver

Engrave or cut vector paths (from SVG/DXF imports or designer shapes) with a laser.

**Parameters**:
- **Cutting**: Feed rate, travel rate, cut power, engrave power, power scale
- **Multi-pass**: Enable multiple passes with Z step-down per pass
- **Hatching**: Fill closed shapes with parallel lines at a specified angle and spacing
- **Cross-hatching**: Add a second perpendicular hatch pass
- **Effects**: Dwell time, width adjustment, position offset

**Output**: G-code for vector cutting and/or fill engraving.

---

## Spoilboard Surfacing

Generate a parallel toolpath to flatten your CNC machine's spoilboard or wasteboard.

**Parameters**:
- Surface width and height
- Tool diameter and stepover percentage
- Feed rate, spindle speed, cutting depth
- Safe Z height

**Output**: G-code for back-and-forth parallel passes covering the entire surface area.

---

## Spoilboard Grid

Generate a reference grid of holes on your spoilboard for workholding.

**Parameters**:
- Grid width and height
- X and Y spacing between holes
- Hole diameter and depth

**Output**: G-code with drill operations at each grid intersection point.

---

## Gerber Converter (PCB Milling)

Convert Gerber and Excellon files to G-code for PCB isolation routing and drilling.

**Supported Layers**:
- Top Copper
- Bottom Copper
- Solder Mask
- Screen Print

**Parameters**:
- Tool diameter for isolation routing
- Isolation width
- Drill file processing (Excellon format)

**Output**: G-code for isolation milling around copper traces and drilling through-holes.

---

## Speeds and Feeds Calculator

Calculate optimal cutting parameters based on material properties and tool geometry.

**Inputs**:
- Material type (from the materials library or custom)
- Tool diameter, flute count, tool type
- Machine constraints (max RPM)

**Outputs**:
- Recommended RPM
- Calculated feed rate (mm/min)
- Chip load per tooth
- Warnings if parameters exceed machine limits or recommended ranges

---

## Common Settings Across CAM Tools

Many CAM tools share these parameters:

| Parameter | Description |
|-----------|-------------|
| Feed Rate | Cutting speed in mm/min |
| Plunge Rate | Z-axis descent speed in mm/min |
| Safe Height | Z clearance for rapid positioning moves |
| Spindle Speed | RPM for the spindle or laser power |
| Material Thickness | For through-cutting operations |

All dimensional values are entered in mm (or inches, depending on your unit setting). Internal calculations always use mm.
