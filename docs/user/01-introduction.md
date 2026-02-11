# Introduction to GCodeKit5

## What is GCodeKit5?

GCodeKit5 is a modern, cross-platform G-code sender and CNC machine controller written in Rust with a GTK4 user interface. It provides a comprehensive solution for controlling CNC machines, laser engravers, and other G-code-based devices, with an integrated CAD designer and a full suite of CAM tools.

## Who is it for?

- **Hobbyist CNC enthusiasts** with desktop mills and routers
- **Laser engraver operators** using GRBL-based controllers
- **Makerspaces and fab labs** needing reliable machine control
- **Professional prototypers** requiring advanced CAM features
- **PCB makers** using CNC isolation routing

## Key Features

### Machine Control
- Multi-axis CNC control (up to 6 axes: X, Y, Z, A, B, C)
- Real-time Digital Readout (DRO) with machine and work positions
- WASD keyboard jogging with configurable step sizes
- Feed rate, spindle speed, and rapid overrides (0–200%)
- Work Coordinate Systems (G54–G59)
- Homing, probing, and soft limit management

### Connectivity
- **Serial/USB** connection with auto port discovery
- **TCP/IP** network connections
- **WebSocket** connections (FluidNC WiFi support)
- Auto-reconnect with configurable timeout
- Support for 6 firmware types: GRBL, grblHAL, TinyG, g2core, FluidNC, Smoothieware

### G-Code Editing
- Syntax highlighting for G-codes, M-codes, coordinates, and comments
- Search and replace with case-sensitive matching
- Real-time execution tracking with current-line indicator
- File statistics (move counts, estimated run time)

### 3D Visualization
- OpenGL-based 3D toolpath visualization
- Stock removal simulation with height-map rendering
- Camera controls: rotate, zoom, pan, and preset views via NavCube
- Grid, axes, bounding box, and tool position overlays

### Designer (CAD)
- Create shapes: rectangle, circle, ellipse, line, polyline, polygon, triangle, text
- Parametric generators for gears and sprockets
- Import SVG, DXF, and STL files
- Non-destructive geometry operations (offset, fillet, chamfer)
- Toolpath generation with pocket, contour, and V-carve operations
- Layer management and shape alignment tools

### CAM Tools
- **Tabbed Box Maker** — Finger-jointed box designs for laser/CNC
- **Jigsaw Puzzle Maker** — Interlocking puzzle piece generation
- **Drill Press** — Standard, peck, and helical drilling
- **Laser Bitmap Engraver** — Raster image to G-code conversion
- **Laser Vector Engraver** — Contour cutting with hatching and multi-pass
- **Spoilboard Surfacing** — Parallel toolpath for bed leveling
- **Spoilboard Grid** — Reference grid generation
- **Gerber Converter** — PCB isolation routing from Gerber files
- **Speeds & Feeds Calculator** — Optimal cutting parameters from material/tool data

### Safety & Diagnostics
- Emergency stop with state tracking (Armed, Triggered, Resetting, Stopped)
- Motion interlock and feed hold
- Communication, buffer, and performance diagnostics
- Soft limit enforcement

### Additional Features
- Customizable macro system with variable substitution
- Tool library and materials database
- Full keyboard shortcut customization
- Built-in help system with markdown-based topics
- Device console with message filtering and command history

## System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| OS | Linux (Ubuntu 22.04+), macOS 10.15+, Windows 10 | Latest stable |
| Memory | 512 MB | 2 GB |
| Display | 1024×768 | 1920×1080 at 125% scaling |
| Rust | 1.88+ (MSRV) | Latest stable |
| GPU | OpenGL 3.3 capable | OpenGL 4.1+ |

## Next Steps

- [Installation](02-installation.md) — Install GCodeKit5 on your system
- [Quick Start](03-quick-start.md) — Get running in 5 minutes
- [Device Setup](04-device-setup.md) — Connect your CNC controller
