# Coordinate System Documentation

This document explains the coordinate systems used in GCodeKit5, how they relate to each other, and the transformations applied when rendering.

## Overview

GCodeKit5 works with three different coordinate system conventions:

| System | Origin | Y Direction | Used By |
|--------|--------|-------------|---------|
| **CNC/G-code** | Front-left of machine | Up/Away from operator | G-code files, machine commands |
| **Screen/GTK** | Top-left of window | Down | GTK4 drawing, mouse events |
| **Mathematical** | Center or bottom-left | Up | Calculations, user display |

## Coordinate System Diagrams

### CNC Machine Coordinate System (G-code)

```
        +Y (away from operator)
         ↑
         │
         │      Workpiece
         │    ┌───────────┐
         │    │           │
         │    │   (0,0)   │
         │    │     ↓     │
         └────┼───────────┼────→ +X (right)
              │           │
              └───────────┘
              
    Origin (0,0) at front-left corner
    Z+ points UP (towards ceiling)
```

### Screen/GTK Coordinate System

```
    (0,0)─────────────────→ +X (right)
         │
         │
         │   Canvas/Drawing Area
         │
         │
         ↓
        +Y (down)
        
    Origin (0,0) at top-left corner
```

### GCodeKit5 Display (After Y-flip)

```
         +Y (up, matches CNC)
         ↑
         │
         │    Toolpath Display
         │    ┌───────────┐
         │    │   ~~~~    │
         │    │  /    \   │
         │    │ │      │  │
         └────┼─┴──────┴──┼────→ +X (right)
              │           │
    (0,0)     └───────────┘
    
    Origin (0,0) at bottom-left (matches CNC view)
```

## The Y-Flip Transformation

### Why We Need It

When drawing G-code toolpaths on screen:
- G-code uses Y+ = away from operator (up on screen for intuitive viewing)
- GTK uses Y+ = down on screen
- Without transformation, toolpaths appear upside-down

### The Solution

We apply a Y-flip at render time in the `ViewportTransform`:

```rust
/// Convert world coordinates (mm) to screen coordinates (pixels)
pub fn world_to_screen(&self, world_x: f64, world_y: f64) -> (f64, f64) {
    // Apply zoom and pan
    let screen_x = (world_x - self.pan_x) * self.zoom + self.center_x;
    
    // Y-flip: subtract from height to flip the axis
    let screen_y = self.height - ((world_y - self.pan_y) * self.zoom + self.center_y);
    
    (screen_x, screen_y)
}

/// Convert screen coordinates (pixels) to world coordinates (mm)
pub fn screen_to_world(&self, screen_x: f64, screen_y: f64) -> (f64, f64) {
    let world_x = (screen_x - self.center_x) / self.zoom + self.pan_x;
    
    // Reverse the Y-flip
    let world_y = (self.height - screen_y - self.center_y) / self.zoom + self.pan_y;
    
    (world_x, world_y)
}
```

### Transformation Matrix Form

The complete transformation can be expressed as a matrix:

```
┌ screen_x ┐   ┌ zoom    0    tx ┐   ┌ world_x ┐
│ screen_y │ = │  0   -zoom   ty │ × │ world_y │
└    1     ┘   └  0      0    1  ┘   └    1    ┘

Where:
- zoom = current zoom level (pixels per mm)
- tx = center_x - pan_x * zoom
- ty = height - center_y + pan_y * zoom (includes Y-flip)
```

## Practical Examples

### Example 1: Drawing a Line

G-code command: `G1 X10 Y20` (move to 10mm right, 20mm forward)

```rust
// World coordinates from G-code
let world_start = (0.0, 0.0);   // Origin
let world_end = (10.0, 20.0);   // Target position

// Convert to screen coordinates (assuming zoom=5, no pan, 800x600 canvas)
let screen_start = viewport.world_to_screen(0.0, 0.0);   // (400, 300)
let screen_end = viewport.world_to_screen(10.0, 20.0);   // (450, 200)

// Note: screen_end.y < screen_start.y because Y is flipped
// The line goes UP on screen, matching CNC convention
```

### Example 2: Mouse Click to World Position

```rust
// User clicks at screen position (450, 200)
let (world_x, world_y) = viewport.screen_to_world(450.0, 200.0);
// Result: (10.0, 20.0) - correctly maps to CNC position
```

### Example 3: Drawing a Circle (Arc)

G-code: `G2 X20 Y0 I10 J0` (clockwise arc, center offset I=10, J=0)

```rust
// In CNC coordinates, this is a clockwise arc
// After Y-flip, it STILL appears clockwise on screen
// because both the points AND the direction are flipped

// The arc center in world coords
let center = (10.0, 0.0);

// Start and end points
let start = (0.0, 0.0);
let end = (20.0, 0.0);

// When drawing with Cairo after transformation:
// - Points are transformed correctly
// - Arc direction (CW/CCW) is preserved visually
```

## Implications for Rotations

### Rotation Direction

Due to the Y-flip:
- **Positive rotation** (counter-clockwise in standard math) appears **counter-clockwise** on screen
- This is because both the Y-axis AND rotation direction are flipped, canceling out

```rust
// Rotating a point 90° counter-clockwise around origin
// In standard math: (1, 0) → (0, 1)
// After Y-flip display: still appears as CCW rotation on screen

fn rotate_point(x: f64, y: f64, angle_rad: f64) -> (f64, f64) {
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    (
        x * cos_a - y * sin_a,
        x * sin_a + y * cos_a,
    )
}
```

### Designer Shape Rotation

In the Designer, shapes store rotation in degrees (CCW positive):

```rust
pub struct Shape {
    pub x: f64,           // Center X in mm
    pub y: f64,           // Center Y in mm
    pub rotation: f64,    // Degrees, CCW positive
    // ...
}
```

When rendering:
1. Translate to shape center
2. Apply rotation (works correctly due to Y-flip symmetry)
3. Draw shape geometry
4. Reverse transformations

## Grid and Axis Rendering

### Grid Lines

Grid lines are drawn in screen space but labeled in world space:

```rust
fn draw_grid(&self, cr: &cairo::Context, viewport: &ViewportTransform) {
    let grid_spacing = 10.0; // mm
    
    // Find visible world bounds
    let (min_x, min_y) = viewport.screen_to_world(0.0, viewport.height);
    let (max_x, max_y) = viewport.screen_to_world(viewport.width, 0.0);
    
    // Draw vertical lines (constant X)
    let start_x = (min_x / grid_spacing).floor() * grid_spacing;
    for x in (start_x as i32..=max_x as i32).step_by(grid_spacing as usize) {
        let (sx, _) = viewport.world_to_screen(x as f64, 0.0);
        cr.move_to(sx, 0.0);
        cr.line_to(sx, viewport.height);
    }
    
    // Draw horizontal lines (constant Y) - note the flipped iteration
    // ...
}
```

### Origin Marker

The origin (0,0) is drawn at the bottom-left of the visible work area:

```rust
fn draw_origin(&self, cr: &cairo::Context, viewport: &ViewportTransform) {
    let (ox, oy) = viewport.world_to_screen(0.0, 0.0);
    
    // X-axis arrow (red, pointing right)
    cr.set_source_rgb(1.0, 0.0, 0.0);
    cr.move_to(ox, oy);
    cr.line_to(ox + 50.0, oy);
    cr.stroke();
    
    // Y-axis arrow (green, pointing UP due to Y-flip)
    cr.set_source_rgb(0.0, 1.0, 0.0);
    cr.move_to(ox, oy);
    cr.line_to(ox, oy - 50.0);  // Negative because screen Y is flipped
    cr.stroke();
}
```

## Text Rendering Considerations

Text requires special handling because the Y-flip would render it upside-down:

```rust
fn draw_label(&self, cr: &cairo::Context, world_x: f64, world_y: f64, text: &str) {
    let (sx, sy) = viewport.world_to_screen(world_x, world_y);
    
    // Save current transformation
    cr.save();
    
    // Move to position
    cr.translate(sx, sy);
    
    // Do NOT apply the Y-flip to text - draw in screen space
    cr.show_text(text);
    
    cr.restore();
}
```

## Summary

| Operation | Key Point |
|-----------|-----------|
| **Store coordinates** | Always in CNC/world space (mm, Y+ = up) |
| **Transform for display** | Apply Y-flip in ViewportTransform |
| **Mouse input** | Reverse transform to get world coords |
| **Rotations** | Work correctly due to flip symmetry |
| **Text** | Draw in screen space (no flip) |
| **Grid labels** | Show world coordinates |

## Related Documentation

- [ADR-002: Coordinate System](adr/ADR-002-coordinate-system.md) - Architecture decision record
- [GTK4.md](../GTK4.md) - GTK4-specific implementation notes
- `crates/gcodekit5-visualizer/src/viewport.rs` - ViewportTransform implementation
