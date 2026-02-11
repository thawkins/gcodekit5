# Designer (CAD)

## Overview

The Designer is a built-in CAD tool for creating 2D and 3D designs that can be converted to G-code toolpaths. It supports drawing shapes, importing files, and generating cutting/engraving operations.

## Drawing Shapes

Select a shape tool from the toolbox and click/drag on the canvas to create it.

### Available Shapes

| Shape | Description |
|-------|-------------|
| **Rectangle** | Rectangular shape with width and height |
| **Circle** | Circle with a specified radius |
| **Ellipse** | Ellipse with major and minor axes |
| **Line** | Straight line between two points |
| **Polyline** | Multi-segment path (click points, double-click to finish) |
| **Triangle** | Three-point triangular shape |
| **Polygon** | Regular polygon with configurable number of sides |
| **Text** | Text rendered as outlines (configurable font, size, bold/italic) |

### Parametric Generators

| Generator | Description |
|-----------|-------------|
| **Gear** | Involute spur gear with configurable module and tooth count |
| **Sprocket** | Chain sprocket with configurable pitch and tooth count |

### Fast Shape Gallery

A gallery of pre-built parametric shape templates for quick insertion.

## Shape Manipulation

### Selection
- **Click** a shape to select it (displays handles)
- **Ctrl+Click** to add/remove from selection
- **Drag** a selection box to select multiple shapes
- **Ctrl+A** to select all shapes

### Transform Operations
- **Move** — Drag a selected shape to reposition
- **Resize** — Drag corner handles to scale
- **Rotate** — Drag rotation handle or enter exact angle
- **Mirror** — Flip horizontally or vertically
- **Scale** — Scale by a precise factor

### Alignment
Align multiple selected shapes:
- Left, Right, Top, Bottom edge alignment
- Center horizontally or vertically

### Arrays
Duplicate shapes in patterns:
- **Linear array** — Copies along a line
- **Circular array** — Copies around a center point
- **Grid array** — Copies in a rectangular grid

### Grouping
- **Ctrl+G** — Group selected shapes
- **Ctrl+Shift+G** — Ungroup

## Non-Destructive Geometry Operations

These operations modify shape geometry without destroying the original:

| Operation | Description |
|-----------|-------------|
| **Offset** | Expand or contract the shape boundary by a distance |
| **Fillet** | Round corners with a specified radius |
| **Chamfer** | Bevel corners at a specified distance |

## Layers

The layers panel lets you organize shapes into layers:
- Create, rename, and delete layers
- Toggle layer visibility
- Lock layers to prevent editing
- Reorder layer stacking

## Importing Files

Import external designs via **File → Import**:

| Format | Description |
|--------|-------------|
| **SVG** | Scalable Vector Graphics — paths, circles, rectangles, ellipses |
| **DXF** | Drawing Exchange Format — lines, circles, arcs, polylines |
| **STL** | 3D models for CAM processing |

Imported geometry is scaled correctly to millimeters.

## Toolpath Generation

Convert designer shapes into G-code toolpaths:

### Operations

| Operation | Description |
|-----------|-------------|
| **Contour** | Cut along the outline of a shape (inside, outside, or on-line) |
| **Pocket** | Remove material from inside a closed shape |
| **V-Carve** | Engrave using a V-shaped bit with variable depth |
| **Adaptive Clearing** | Advanced stock removal with optimized tool engagement |
| **Multi-pass** | Automatic depth stepping for deep cuts |
| **Drilling** | Generate drill points from circles or marked positions |

### Toolpath Parameters
- Tool selection from the [Tool Library](60-tool-library.md)
- Feed rate, plunge rate, spindle speed
- Depth of cut per pass
- Safe Z height for rapid moves

## Undo / Redo

- **Ctrl+Z** — Undo the last action
- **Ctrl+Y** — Redo
- Full history is maintained until the file is closed

## Project Files

Save your designs as `.gck4` project files, which preserve all shapes, layers, properties, and toolpath settings.

## See Also

- [CAM Tools](50-camtools.md) — Specialized G-code generators
- [Tool Library & Materials](60-tool-library.md) — Tool and material management
