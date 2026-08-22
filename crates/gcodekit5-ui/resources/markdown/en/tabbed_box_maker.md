# Tabbed Box Maker

Generates a finger-jointed box layout and outputs G-code for laser cutting or CNC routing.

The generator creates multiple 2D panels (sides, top/bottom, dividers) and arranges them into a flat layout for cutting.

## Key inputs
### Box dimensions
- **X (Width), Y (Depth), H (Height)**: Overall box size.
- **Outside dims**: If enabled, the dimensions are treated as the outside size; otherwise they’re inside size.

### Box configuration
- **Box type**: Allows removing specific faces (no top, no bottom, etc.).
- **Dividers X / Y**: Number of internal dividers.
- **Divider keying**: Which faces include slots/tabs for dividers.
- **Optimize layout**: Attempts to rotate/pack panels to fit within device bounds.

### Material settings
- **Thickness**: Stock thickness.
- **Burn / Tool dia**: Kerf/tool compensation term.

### Finger joint settings
- **Finger width / Space width**: Expressed as multiples of thickness.
- **Surrounding spaces**: Extra spacing near edges.
- **Play**: Fit tolerance.
- **Extra length**: Adds length to tabs for tighter fits.

### Laser settings
- **Passes / Power (S) / Feed rate**: Cutting parameters (laser-style), used when producing the final G-code.

### Work origin offsets
Offsets the entire layout, and optionally homes before start.

## Optimize layout
The optimizer tries to position and rotate parts to fit within the current device bounds.

If a layout cannot fit:
- You will be prompted with **Cancel** or **Continue**.
- **Continue** ignores the out-of-bounds condition.

## Workflow
1. Set box dimensions and material thickness.
2. Configure joints and divider options.
3. Generate and preview bounds.
4. Cut the panels and assemble.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)


