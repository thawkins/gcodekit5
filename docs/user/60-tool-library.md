# Tool Library & Materials

## Tool Library

The tool library stores your cutting tools so they can be selected when generating toolpaths.

### Tool Properties

| Property | Description |
|----------|-------------|
| **Tool Number** | Reference number for the tool |
| **Name / Description** | Descriptive label |
| **Diameter** | Tool diameter in mm |
| **Length Offset** | Tool length offset for Z compensation |
| **Flute Count** | Number of cutting flutes |
| **Max RPM** | Maximum spindle speed for this tool |

### Managing Tools

- **Add Tool** — Click Add and enter the tool properties
- **Edit Tool** — Select a tool from the list and modify its values
- **Delete Tool** — Remove a tool from the library

Tool data is stored persistently and available across sessions.

### Using Tools

When generating toolpaths in the [Designer](40-designer.md) or [CAM Tools](50-camtools.md), select a tool from the library. The toolpath generator uses the tool diameter for offset calculations and the flute count for feeds and speeds.

## Materials Database

The materials database stores cutting parameters for different materials, used by the [Speeds & Feeds Calculator](50-camtools.md).

### Material Properties

| Property | Description |
|----------|-------------|
| **Material Name** | Descriptive name (e.g., "Aluminum 6061", "Baltic Birch Plywood") |
| **Recommended Feed Rate** | Suggested feed rate in mm/min |
| **Recommended Spindle Speed** | Suggested RPM |
| **Tool Compatibility** | Which tool types and diameters work well |

### Managing Materials

- **Add Material** — Define a new material with its cutting parameters
- **Edit Material** — Modify properties of an existing material
- **Delete Material** — Remove a material from the database

### Built-in Materials

GCodeKit5 includes a set of common materials with recommended parameters. You can customize these or add your own.

## Tool Change

GCodeKit5 supports three tool change modes:

| Mode | Description |
|------|-------------|
| **None** | No tool change support — single-tool operations |
| **Manual** | Pauses the program and prompts the operator to change the tool |
| **Automatic** | Sends ATC (Automatic Tool Changer) commands to the controller |

Configure the tool change mode in [Settings](70-settings.md) or the [Advanced Features](85-safety-diagnostics.md) panel.

## See Also

- [CAM Tools](50-camtools.md) — Using tools in G-code generation
- [Designer](40-designer.md) — Toolpath generation from designs
- [Settings](70-settings.md) — Application configuration
