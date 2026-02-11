# Bug Fix: Insets Not Rotating with Shapes

**Date**: December 31, 2025  
**Status**: Fixed  
**Severity**: Medium  
**Component**: Designer / Shape Geometry Operations

## Problem Description

When creating shapes with "Insets" (the offset geometry operation), the inset did not rotate with the shape if a rotation angle was applied. This resulted in incorrect geometry where the inset remained in the original orientation even though the parent shape was rotated.

## Root Cause

The bug was caused by inconsistent handling of rotation values in the `as_csg()` methods of various shape types. The codebase stores rotation angles in **degrees** (as verified by the rotation system tests), but transformation matrices expect values in **radians**.

### Affected Shapes

1. **Circle** (`DesignCircle::as_csg()`): Line 418
   - Used `self.rotation` directly instead of `self.rotation.to_radians()`
   
2. **Ellipse** (`DesignEllipse::as_csg()`): Lines 825-841
   - Did not apply rotation transformation at all - only translated the shape
   
3. **Polygon** (`DesignPolygon::transform()`): Line 2403
   - Converted angle from radians to degrees incorrectly

### Why It Matters

When the `perform_offset()` function is called (for insets, fillets, or chamfers), it:
1. Calls `shape.as_csg()` to get a CSG representation
2. Converts to multipolygon
3. Applies the offset operation
4. Returns the modified shape

If `as_csg()` doesn't properly apply rotation, the offset operation works on an unrotated shape, producing incorrect results.

## Solution

Fixed the rotation handling in three locations:

### 1. DesignCircle::as_csg()

**Before:**
```rust
fn as_csg(&self) -> Sketch<()> {
    let sketch = Sketch::circle(self.radius, 32, None);
    let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation)); // BUG: using degrees as radians
    let translation =
        Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));
    sketch.transform(&(translation * rotation))
}
```

**After:**
```rust
fn as_csg(&self) -> Sketch<()> {
    let sketch = Sketch::circle(self.radius, 32, None);
    let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians())); // FIXED
    let translation =
        Matrix4::new_translation(&Vector3::new(self.center.x, self.center.y, 0.0));
    sketch.transform(&(translation * rotation))
}
```

### 2. DesignEllipse::as_csg()

**Before:**
```rust
fn as_csg(&self) -> Sketch<()> {
    // Approximate ellipse with polygon
    let steps = 32;
    let mut points = Vec::with_capacity(steps);
    for i in 0..steps {
        let theta = 2.0 * std::f64::consts::PI * (i as f64) / (steps as f64);
        let x = self.rx * theta.cos();
        let y = self.ry * theta.sin();
        points.push([x, y]);
    }

    let sketch = Sketch::polygon(&points, None);
    let translation = Matrix4::new_translation(&Vector3::new(
        self.center.x as f64,
        self.center.y as f64,
        0.0,
    ));
    sketch.transform(&translation) // BUG: no rotation applied
}
```

**After:**
```rust
fn as_csg(&self) -> Sketch<()> {
    // Approximate ellipse with polygon
    let steps = 32;
    let mut points = Vec::with_capacity(steps);
    for i in 0..steps {
        let theta = 2.0 * std::f64::consts::PI * (i as f64) / (steps as f64);
        let x = self.rx * theta.cos();
        let y = self.ry * theta.sin();
        points.push([x, y]);
    }

    let sketch = Sketch::polygon(&points, None);
    let rotation = Matrix4::new_rotation(Vector3::new(0.0, 0.0, self.rotation.to_radians())); // FIXED
    let translation = Matrix4::new_translation(&Vector3::new(
        self.center.x as f64,
        self.center.y as f64,
        0.0,
    ));
    sketch.transform(&(translation * rotation)) // FIXED
}
```

### 3. DesignPolygon::transform()

**Before:**
```rust
fn transform(&mut self, t: &Transform) {
    let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
    self.center = Point::new(p.x as f64, p.y as f64);

    let angle = t.m12.atan2(t.m11) as f64; // This is in radians
    self.rotation += angle; // BUG: adding radians to degrees

    let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
    self.radius *= sx;
}
```

**After:**
```rust
fn transform(&mut self, t: &Transform) {
    let p = t.transform_point(point(self.center.x as f32, self.center.y as f32));
    self.center = Point::new(p.x as f64, p.y as f64);

    let angle_deg = t.m12.atan2(t.m11).to_degrees() as f64; // FIXED: convert to degrees
    self.rotation += angle_deg;

    let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
    self.radius *= sx;
}
```

## Testing

Added comprehensive test suite in `crates/gcodekit5-designer/tests/shape_inset_rotation_tests.rs`:

- `test_rectangle_inset_with_rotation()` - Rectangle with 45° rotation
- `test_circle_inset_with_rotation()` - Circle with 30° rotation
- `test_ellipse_inset_with_rotation()` - Ellipse with 60° rotation
- `test_triangle_inset_with_rotation()` - Triangle with 90° rotation
- `test_polygon_inset_with_rotation()` - Hexagon with 45° rotation
- `test_multiple_rotation_angles()` - Various angles (0°, 15°, 30°, 45°, 60°, 90°, 135°, 180°, 270°)
- `test_zero_rotation_baseline()` - Baseline test for unrotated shapes

## Impact

- **User Impact**: High - Users can now reliably use insets on rotated shapes
- **Breaking Changes**: None - This is a bug fix
- **Performance**: No change - Same computational complexity
- **Code Quality**: Improved consistency in rotation handling

## Related Issues

This fix ensures consistency with:
- The rotation system tests in `rotation_system_tests.rs`
- The Rectangle shape which already used `.to_radians()` correctly
- Triangle and other shapes that properly handle rotation

## Verification

To verify the fix works:

1. Create a rectangle in the Designer
2. Set rotation to 45 degrees
3. Apply a -2mm offset (inset)
4. The inset should be rotated along with the rectangle
5. The inset should be smaller and centered within the rotated rectangle

Previously, the inset would appear at the wrong angle, not following the shape's rotation.
