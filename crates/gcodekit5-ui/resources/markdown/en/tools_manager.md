# CNC Tools

The CNC Tools tab manages cutter/tool definitions used by CAM generators and feeds/speeds calculations.

## What this library is (and isn’t)
- ✅ A **tool library** for CAM and generators.
- ❌ Not a controller “tool table” (no automatic tool length offsets, etc.).

## Left panel: browse & filter
Use the left panel to quickly find tools:
- **Search** matches name, ID, type, and common diameter text.
- **Type** filter limits results (end mills, drills, V-bits, etc.).
- **Material** filter limits by tool material/coating class.
- **Diameter min/max** filters help narrow large catalogs.

## Right panel: edit tool details
Tools are edited with tabbed sections (Geometry, Materials, Notes, etc.).

### Geometry (most important)
- **Diameter**: cutting diameter
- **Shaft diameter**: shank diameter (holder compatibility)
- **Flute length**: axial cutting length
- **Overall length**: total tool length
- **Flutes**: number of cutting flutes
- **Corner radius** (if applicable)
- **Tip angle** (drills / spot drills / V-bits)

### Identity
- **Tool ID** should be unique and stable (recommended: `vendor_series_diameter_flutes`, e.g. `harvey_20008_6p0_2f`).
- Tool “numbers” are not required unless a specific workflow expects them.

## Library management
Use the **Library** panel to:
- Import supplier catalogs (GTC)
- Import/export custom tools for backup/sharing
- Reset custom/imported tools (destructive)

## Workflow examples
- Create a tool → set geometry → Save.
- Import a GTC catalog → filter by diameter/material → review/edit a tool.
- Export custom tools to JSON for backup/sharing.

## Related
- [CAM Tools](help:cam_tools)
- [Materials](help:materials_manager)
- [Index](help:index)


