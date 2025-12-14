# CNC Tools

The CNC Tools tab manages cutter/tool definitions used for CAM and G-code generation.

## Units and conventions
- **All dimensions are in mm** and displayed with **3 decimal places**.
- **Tool ID** must be unique and stable (recommended: `vendor_series_diameter_flutes`, e.g. `harvey_20008_6p0_2f`).
- This tool library is **not** a device tool table; tool “numbers” are not used here.

## Tool fields (quick reference)
- **Diameter**: cutting diameter.
- **Shaft diameter**: shank diameter (used for holder compatibility).
- **Flute length**: axial cutting length.
- **Flutes**: number of cutting flutes.
- **Corner radius**: only applies to corner-radius end mills.
- **Tip angle**: used for drills / spot drills / V-bits.
- **Shank**: by default derived from shaft diameter (Straight), or can be set to Collet/Tapered.

## Workflow examples
- Create a tool → set geometry/material/coating → Save.
- Import a supplier GTC catalog → filter by diameter/material → select a tool to review/edit.
- Export custom tools to JSON for backup/sharing.

## Related
- [CAM Tools](help:cam_tools)
- [Index](help:index)
