# Laser Vector Engraver

Converts vector artwork (SVG / DXF) into G-code for laser cutting/engraving.

## What it generates
- Vector contours (polylines) converted into G0/G1 moves
- Optional multi-pass depth stepping (useful for CNC or repeated laser passes)
- Optional hatch fill (if enabled in the tool)

## Key inputs
### Vector file
- **Vector path**: SVG/DXF source.

### Motion
- **Feed rate**: Cutting/engraving feed.
- **Travel rate**: Non-burning moves.

### Power
- **Cut power / Engrave power**: `S` values used for different operations.
- **Power scale (S)**: Controller max S.
- **Invert power**: Useful depending on how your controller interprets intensity.

### Multi-pass
- **Multi-pass / # passes / Z increment**: Repeats paths multiple times, optionally stepping Z for CNC.

### Output sizing & placement
- **Desired width**: Scales the artwork to a known width.
- **Offset X / Y**: Moves the output relative to origin.

### Optional fill (hatching)
- **Hatch angle / spacing / tolerance / cross-hatch**: Generates line fills for engraving.

## Workflow
1. Load an SVG/DXF and confirm preview.
2. Set sizing + offsets to match stock.
3. Set power and feed.
4. Generate, preview bounds, then run.

## Notes
- Complex artwork can produce very large G-code; consider simplifying vectors.
- Always verify units in the source artwork (mm vs px) and use the desired width setting to normalize.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)


