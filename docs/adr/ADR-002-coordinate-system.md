# ADR-002: Coordinate System (Y-flip)

## Status
Accepted

## Context

CNC machines and CAD software use different coordinate system conventions:

- **CNC/G-code Convention**: Y-axis increases moving away from the operator (towards the back of the machine). Origin (0,0) is typically at the front-left corner.

- **Screen/Graphics Convention**: Y-axis increases downward (positive Y goes down the screen). Origin (0,0) is at the top-left.

- **Mathematical Convention**: Y-axis increases upward (Cartesian coordinates).

GCodeKit5 must display G-code toolpaths in a way that matches the physical machine orientation while rendering correctly on screen.

## Decision

We implement a **Y-flip transformation** at the rendering layer:

1. **Internal Representation**: All coordinates are stored in CNC machine coordinates (Y increases towards back/up in Cartesian sense).

2. **Rendering Transformation**: When drawing to screen, the viewport applies a Y-axis flip so that:
   - The origin (0,0) appears at the bottom-left of the canvas
   - Positive Y extends upward on screen
   - This matches how operators visualize their machine bed

3. **Centralized Transform**: The `ViewportTransform` struct handles all coordinate conversions between world coordinates (mm) and screen coordinates (pixels).

### Implementation

```rust
// In ViewportTransform
pub fn world_to_screen(&self, world: Point) -> Point {
    Point {
        x: (world.x - self.pan_x) * self.zoom + self.width / 2.0,
        y: self.height - ((world.y - self.pan_y) * self.zoom + self.height / 2.0),
    }
}
```

## Consequences

### Positive
- Visual display matches physical machine orientation
- Operators can intuitively understand toolpath preview
- Consistent with most CAM software conventions
- Single point of transformation logic

### Negative
- Must remember to apply transform for all drawing operations
- Mouse input must be reverse-transformed to get world coordinates
- Can cause confusion if transform is forgotten in new drawing code

### Neutral
- Grid lines and axis labels must account for the flip
- Text rendering may need special handling to avoid upside-down text

## Alternatives Considered

1. **Store coordinates in screen space**: Would require transforming all imported G-code, making parsing more complex and error-prone.

2. **No flip (Y-down matches screen)**: Would show toolpaths upside-down relative to machine, confusing operators.

3. **User-configurable orientation**: Adds complexity; most users expect the standard CNC convention.

## References

- [GRBL Coordinate System](https://github.com/gnea/grbl/wiki/Grbl-v1.1-Configuration#10---status-report-mask)
- [G-code Coordinate Systems](https://www.cnccookbook.com/g-code-coordinate-systems/)
