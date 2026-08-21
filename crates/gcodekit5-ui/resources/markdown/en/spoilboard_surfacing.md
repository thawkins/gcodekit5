# Spoilboard Surfacing

Generates a planar surfacing toolpath to flatten your spoilboard.

## What it generates
- A back-and-forth raster toolpath over a rectangular area
- Safe Z retracts and lead-ins suitable for a surfacing bit / flycutter

## Key inputs
### Spoilboard dimensions
- **Width / Height**: Area to surface.

### Tool settings
- **Tool diameter**: Diameter of the surfacing cutter.
- **Cut depth**: Depth per pass.
- **Stepover (%)**: Lateral overlap per pass (typically 40–70%).

### Machine settings
- **Feed rate**: Cutting feed.
- **Spindle speed**: Target spindle RPM.
- **Safe Z**: Clearance height for rapids.
- **Home before start**: Optionally inserts `$H` at the start.

## Workflow
1. Set the surfacing rectangle to match the spoilboard area you want flattened.
2. Choose a conservative depth and appropriate stepover.
3. Generate and preview bounds.
4. Run with proper dust collection.

## Safety
- Confirm Z zero and ensure clamps are below surfacing height.
- Use conservative depths if your machine is light-duty.

## Related
- [CAM Tools](help:cam_tools)
- [Visualizer](help:visualizer)
- [Index](help:index)


