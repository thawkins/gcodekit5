# Spoilboard Grid

Generates a grid pattern (lines) for marking or lightly engraving a spoilboard.

## What it generates
- A set of horizontal/vertical lines at a fixed spacing
- Intended for laser marking, V-bit scribing, or shallow engraving

## Key inputs
### Spoilboard dimensions
- **Width / Height**: Size of the grid area.
- **Grid spacing**: Distance between lines.

### Laser settings
- **Laser power (S)**: Output power.
- **Feed rate**: Cutting/marking speed.
- **Laser mode**: M3 (constant) vs M4 (dynamic) depending on controller.

### Home before start
Optionally inserts `$H` at the start.

## Workflow
1. Set width/height to your usable spoilboard area.
2. Pick spacing (commonly 10mm or 25mm).
3. Choose a low power suitable for marking.
4. Generate and preview, then run.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)


