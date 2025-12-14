# Laser Image Engraver

Converts a bitmap (PNG/JPEG/etc) into a raster-style laser engraving toolpath (G-code).

## What it generates
- A scanline engraving path (horizontal or vertical)
- Optional bidirectional scanning to reduce travel time
- Output power values scaled into the controller’s `S` range

## Key inputs
### Image file
- **Image Path**: The source bitmap.

### Output settings
- **Width**: Desired output width in mm (height is derived from image aspect ratio).
- **Feed rate**: Engraving feed.
- **Travel rate**: Non-burning moves between scan segments.

### Laser power
- **Min / Max power (%)**: Maps image brightness to a power range.
- **Power scale (S)**: Controller maximum `S` value (often 1000 on GRBL).

### Scanning
- **Scan direction**: Horizontal/Vertical.
- **Pixels per mm**: Controls resolution; higher = more detail but more time.
- **Line spacing**: Distance between scan lines.
- **Bidirectional**: Engrave on both forward and return passes.

### Transformations
- Invert, mirror, rotation to match how the part is oriented on your machine.

### Halftoning
Optional methods to represent grayscale using dots/patterns.

### Work offsets
Offsets the entire engraving and optionally homes before start.

## Workflow
1. Select an image, confirm the preview.
2. Choose output width and resolution.
3. Set min/max power and feed.
4. Generate and preview; verify that travel moves and bounds match your stock.

## Safety
- Start with conservative power and test on scrap.
- Verify your machine is in laser mode (if applicable) and that ventilation is adequate.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)
