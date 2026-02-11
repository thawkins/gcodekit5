# Toolpath Visualizer

## Overview

The 3D Visualizer renders your G-code toolpath and provides stock removal simulation. It uses OpenGL for hardware-accelerated rendering.

## Toolpath Display

When a G-code file is loaded, the visualizer shows:

- **Rapid moves (G0)** — Displayed in a distinct color (typically red/orange)
- **Feed moves (G1)** — Displayed in a different color (typically green/blue)
- **Arc moves (G2/G3)** — Rendered as smooth curves
- **Current tool position** — Shown as a marker during connection or streaming

## Camera Controls

### Mouse Controls

| Action | Control |
|--------|---------|
| **Rotate** | Click and drag |
| **Zoom** | Scroll wheel |
| **Pan** | Middle-click and drag |

### NavCube

The NavCube in the corner of the visualizer provides quick access to standard views:

| View | Description |
|------|-------------|
| **Top** | Camera above, looking down (XY plane) |
| **Front** | Camera in front, looking at XZ plane |
| **Right** | Camera at right, looking at YZ plane |
| **Isometric** | Camera at 45° angle showing all axes |

Click any face of the NavCube to snap to that view.

### Keyboard Controls

| Key | Action |
|-----|--------|
| **+** / **-** | Zoom in / out |
| **0** | Fit toolpath to window |
| **Arrow keys** | Pan the view |

## Display Options

Toggle visibility of visual elements:

| Option | Description |
|--------|-------------|
| **Grid** | Reference grid on the XY plane |
| **Axes** | X (red), Y (green), Z (blue) axis indicators |
| **Stock material** | 3D representation of the workpiece |
| **Toolpath** | The G-code path lines |
| **Tool marker** | Current tool/laser position indicator |

Grid spacing is configurable in [Settings](70-settings.md).

## Stock Removal Simulation

The visualizer can simulate material removal:

- Displays a height-map rendering showing depth of cuts
- Color gradient indicates material depth
- Progress percentage shown during simulation
- Updates as the program streams to the machine

## Execution Visualization

While streaming a program:

- The **executed portion** of the toolpath is highlighted differently from the remaining path
- The **tool marker** moves in real-time to show the current machine position
- Progress is visible at a glance

## Performance

The visualizer is optimized for large files. Files with 100,000+ lines render within a few seconds. For very large files, consider zooming in to a region of interest for smoother interaction.

## See Also

- [G-Code Editor](20-gcode-editor.md) — Edit your G-code files
- [Streaming G-Code](80-streaming.md) — Running programs
