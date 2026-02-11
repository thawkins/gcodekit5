# GCodeKit5 Design File Format (.gck4)

This document describes the GCodeKit5 design file format used for saving and loading CAD/CAM designs.

## Overview

GCodeKit5 uses JSON-based design files with the `.gck4` extension. The format stores complete design state including shapes, viewport settings, toolpath parameters, and metadata.

## File Structure

```json
{
  "version": "1.0",
  "metadata": {
    "name": "Design Name",
    "created": "2026-01-30T10:00:00Z",
    "modified": "2026-01-30T12:00:00Z",
    "author": "Optional Author",
    "description": "Optional Description"
  },
  "viewport": {
    "zoom": 1.0,
    "pan_x": 0.0,
    "pan_y": 0.0
  },
  "shapes": [...],
  "default_properties": {...},
  "toolpath_params": {...}
}
```

## Version

| Version | Description |
|---------|-------------|
| 1.0 | Initial format (current) |

## Metadata

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Design name |
| `created` | ISO 8601 datetime | Creation timestamp |
| `modified` | ISO 8601 datetime | Last modification timestamp |
| `author` | string | Optional author name |
| `description` | string | Optional description |

## Viewport State

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `zoom` | float | 1.0 | Zoom level (1.0 = 100%) |
| `pan_x` | float | 0.0 | Horizontal pan offset (mm) |
| `pan_y` | float | 0.0 | Vertical pan offset (mm) |

## Shape Data

Each shape in the `shapes` array has the following structure:

### Common Properties

| Field | Type | Description |
|-------|------|-------------|
| `id` | integer | Unique shape identifier |
| `shape_type` | string | Type of shape (see below) |
| `name` | string | Display name |
| `x` | float | X position (mm) |
| `y` | float | Y position (mm) |
| `width` | float | Width (mm) |
| `height` | float | Height (mm) |
| `rotation` | float | Rotation angle (degrees) |
| `selected` | boolean | Selection state |
| `group_id` | integer? | Optional group ID |

### Shape Types

| Type | Description | Additional Fields |
|------|-------------|-------------------|
| `rectangle` | Rectangle/slot | `corner_radius`, `is_slot` |
| `circle` | Circle | - |
| `ellipse` | Ellipse | - |
| `line` | Line segment | - |
| `triangle` | Triangle | - |
| `polygon` | Regular polygon | `sides` |
| `path` | Bezier path | `path_data` (SVG path) |
| `text` | Text shape | `text_content`, `font_*` |
| `gear` | Involute gear | `teeth`, `module`, `pressure_angle` |
| `sprocket` | Roller chain sprocket | `teeth`, `pitch`, `roller_diameter` |

### Operation Properties

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `operation_type` | string | "profile" | `"profile"` or `"pocket"` |
| `use_custom_values` | boolean | false | Use shape-specific toolpath settings |
| `pocket_depth` | float | 0.0 | Total pocket depth (mm) |
| `start_depth` | float | 0.0 | Starting depth (mm) |
| `step_down` | float | 0.0 | Depth per pass (mm) |
| `step_in` | float | 0.0 | Stepover distance (mm) |
| `ramp_angle` | float | 0.0 | Ramp entry angle (degrees) |
| `pocket_strategy` | string | "ContourParallel" | Pocket toolpath strategy |
| `raster_fill_ratio` | float | 0.5 | Raster fill ratio (0-1) |

### Modifier Properties

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `offset` | float | 0.0 | Contour offset (mm) |
| `fillet` | float | 0.0 | Fillet radius (mm) |
| `chamfer` | float | 0.0 | Chamfer size (mm) |
| `lock_aspect_ratio` | boolean | true | Lock aspect ratio during resize |

### Text Properties

| Field | Type | Description |
|-------|------|-------------|
| `text_content` | string | Text string |
| `font_size` | float | Font size (points) |
| `font_family` | string | Font family name |
| `font_bold` | boolean | Bold weight |
| `font_italic` | boolean | Italic style |

### Path Properties

| Field | Type | Description |
|-------|------|-------------|
| `path_data` | string | SVG path data (M, L, C, Q, Z commands) |

### Gear Properties

| Field | Type | Description |
|-------|------|-------------|
| `teeth` | integer | Number of teeth |
| `module` | float | Module (mm) |
| `pressure_angle` | float | Pressure angle (degrees, typically 20) |

### Sprocket Properties

| Field | Type | Description |
|-------|------|-------------|
| `teeth` | integer | Number of teeth |
| `pitch` | float | Chain pitch (mm) |
| `roller_diameter` | float | Roller diameter (mm) |

## Toolpath Parameters

Global toolpath generation settings:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `feed_rate` | float | 1000.0 | Feed rate (mm/min) |
| `spindle_speed` | float | 3000.0 | Spindle speed (RPM) |
| `tool_diameter` | float | 3.175 | Tool diameter (mm) |
| `cut_depth` | float | -5.0 | Default cut depth (mm) |
| `stock_width` | float | 200.0 | Stock width (mm) |
| `stock_height` | float | 200.0 | Stock height (mm) |
| `stock_thickness` | float | 10.0 | Stock thickness (mm) |
| `safe_z_height` | float | 10.0 | Safe Z height (mm) |

## Default Properties

The `default_properties` field stores a `ShapeData` object used as a template for new shapes. It contains the default operation type, depths, and other settings applied to newly created shapes.

## Pocket Strategies

| Strategy | Description |
|----------|-------------|
| `ContourParallel` | Offset contours from outside in |
| `ZigZag` | Back-and-forth raster pattern |
| `Spiral` | Continuous spiral from outside in |
| `Adaptive` | Adaptive clearing (constant engagement) |

## Example File

```json
{
  "version": "1.0",
  "metadata": {
    "name": "Example Design",
    "created": "2026-01-30T10:00:00Z",
    "modified": "2026-01-30T10:00:00Z",
    "author": "",
    "description": ""
  },
  "viewport": {
    "zoom": 1.0,
    "pan_x": 0.0,
    "pan_y": 0.0
  },
  "shapes": [
    {
      "id": 1,
      "shape_type": "rectangle",
      "name": "Main Pocket",
      "x": 10.0,
      "y": 10.0,
      "width": 50.0,
      "height": 30.0,
      "rotation": 0.0,
      "selected": false,
      "operation_type": "pocket",
      "use_custom_values": true,
      "pocket_depth": -5.0,
      "start_depth": 0.0,
      "step_down": 1.0,
      "step_in": 1.5,
      "corner_radius": 3.0,
      "is_slot": false
    }
  ],
  "toolpath_params": {
    "feed_rate": 1000.0,
    "spindle_speed": 12000.0,
    "tool_diameter": 3.175,
    "cut_depth": -5.0,
    "stock_width": 100.0,
    "stock_height": 100.0,
    "stock_thickness": 10.0,
    "safe_z_height": 5.0
  }
}
```

## Error Handling

The loader handles:
- Missing optional fields (uses defaults)
- Unknown shape types (returns error)
- Invalid JSON (returns parse error)
- File I/O errors (returns I/O error)

All errors are returned as `anyhow::Result` with context messages.

## Compatibility

- Forward compatibility: Unknown fields are ignored
- Backward compatibility: Missing fields use defaults via `#[serde(default)]`

## Related Files

- `crates/gcodekit5-designer/src/serialization.rs` - Implementation
- `crates/gcodekit5-designer/src/designer_state/file_io.rs` - File I/O methods
- `crates/gcodekit5-designer/tests/io/serialization.rs` - Tests
