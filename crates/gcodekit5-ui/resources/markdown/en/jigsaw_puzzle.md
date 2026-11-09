# Jigsaw Puzzle Generator

Creates a jigsaw-style cutting pattern and outputs G-code.

This tool is designed primarily for **laser cutters** (or very small CNC tools) and produces a single 2D toolpath that outlines interlocking puzzle pieces.

## What it generates
- An outer rectangle (the overall puzzle boundary)
- Internal cut lines forming pieces, using a pseudo-random jigsaw connector pattern
- Optional multiple passes for cutting through thicker stock

## Key inputs
### Puzzle dimensions
- **Width / Height**: Overall finished puzzle size.
- **Corner radius**: Rounds the outside corners.

### Grid configuration
- **Pieces Across / Pieces Down**: How many pieces in each direction. More pieces means more internal cuts.

### Pattern parameters
- **Kerf**: Compensates for cut width (laser kerf / tool diameter). Increase if pieces are too tight.
- **Random seed**: Changes the piece shapes while keeping the same grid layout. Use **Shuffle** to quickly try new patterns.
- **Tab size / Jitter**: Controls connector size and variation.

### Laser settings
- **Passes**: Number of repeated cutting passes.
- **Power (S)**: Spindle/laser power value (typical GRBL laser uses `S`).
- **Feed rate**: Cutting feed.

### Work offsets
- **Offset X / Y**: Moves the entire puzzle away from machine origin.
- **Home before start**: If enabled, `$H` is inserted at the beginning.

## Workflow
1. Choose puzzle size and piece count.
2. Set kerf and connector sizing.
3. Pick laser power / feed.
4. Generate, preview in Visualizer, then run.

## Notes
- For CNC cutting: ensure your tool diameter and kerf settings make sense, and consider reducing piece count.
- Always preview the result before cutting.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)


