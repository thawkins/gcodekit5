# Bug Fix: Rectangle Width/Height Editing Causes Shape to Move and Wrong Size

**Date**: December 31, 2025  
**Status**: Fixed  
**Severity**: High  
**Component**: Designer / Properties Panel

## Problem Description

When editing the width or height of a rectangle (or any shape) in the Properties Panel inspector, the shape would:
1. Incorrectly move to a different position instead of resizing in place
2. Not resize to the requested dimensions (applying uniform scale instead of independent width/height changes)

The position (X, Y) values were not being preserved correctly when only the size was being updated, and the shape transformation system was applying uniform scaling even when different width and height values were requested.

## Root Cause

There were two bugs:

1. **Position Bug**: In the `Canvas::calculate_position_and_size_updates()` method at lines 1261-1262 in `/home/thawkins/Projects/gcodekit5/crates/gcodekit5-designer/src/canvas.rs`.

A similar bug had previously been fixed in the `set_selected_position_and_size_with_flags()` method (line 1346-1347), but the same issue existed in the helper method `calculate_position_and_size_updates()` which is now being used for undo-aware updates.

2. **Size Bug**: In the `DesignRectangle::transform()` method at lines 291-293 in `/home/thawkins/Projects/gcodekit5/crates/gcodekit5-designer/src/model.rs`.

The transform method was extracting only the X-axis scale factor and applying it to both width and height:
```rust
let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
self.width *= sx;
self.height *= sx; // BUG: Assume uniform scale
```

This meant that when the user changed only the width, both width and height would scale proportionally.

When determining the target position for a shape update, the code had:

```rust
// 2. Determine target values
let target_x = if update_position { x } else { x };
let target_y = if update_position { y } else { y };
```

This meant that whether `update_position` was `true` or `false`, it always used the provided `x` and `y` values from the UI. When editing only width or height, the Properties Panel passes `update_position=false` and `update_size=true`, but it still passes the current X/Y values read from the text entries, which may be stale or incorrectly parsed during user typing.

## User Impact

When a user:
1. Draws a rectangle
2. Clicks on it to select it
3. Changes the width or height in the Properties Panel
4. The rectangle would jump to a different position instead of resizing in place

This made it very difficult to precisely size shapes, as users had to constantly reposition them after every size adjustment.

## Solution

Fixed both issues:

### Position Fix

Changed the conditional logic in `calculate_position_and_size_updates()` to use the shape's current center position when `update_position` is `false`:

**Before:**
```rust
// 2. Determine target values
let target_x = if update_position { x } else { x };
let target_y = if update_position { y } else { y };
```

**After:**
```rust
// Calculate the current center
let old_center_x = min_x + old_w / 2.0;
let old_center_y = min_y + old_h / 2.0;

// 2. Determine target values
// When update_position is false, preserve the current center
let target_center_x = if update_position { x } else { old_center_x };
let target_center_y = if update_position { y } else { old_center_y };

// Calculate target top-left from center
let target_x = target_center_x - target_w / 2.0;
let target_y = target_center_y - target_h / 2.0;
```

Now when `update_position=false`:
- Uses the shape's current center position
- Preserves the shape's location while only adjusting size
- Prevents unwanted movement when editing width/height

### Size Fix

Changed the transform extraction in `DesignRectangle::transform()` to extract separate X and Y scale factors:

**Before:**
```rust
// Extract uniform scale magnitude from the X basis vector.
let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
self.width *= sx;
self.height *= sx; // Assume uniform scale
```

**After:**
```rust
// Extract scale factors from basis vectors (supports non-uniform scaling)
// X basis vector: (m11, m12) - determines width scale
// Y basis vector: (m21, m22) - determines height scale
let sx = (t.m11 * t.m11 + t.m12 * t.m12).sqrt() as f64;
let sy = (t.m21 * t.m21 + t.m22 * t.m22).sqrt() as f64;
self.width *= sx;
self.height *= sy;
```

Now the transform correctly applies independent scaling to width and height, allowing users to change one dimension without affecting the other.

## Related Code

The Properties Panel's width/height focus handlers (lines 1810-1815 and 1858-1863 in `designer_properties.rs`) correctly call:

```rust
designer_state.set_selected_position_and_size_with_flags(x, y, w, h, false, true);
```

With `update_position=false` and `update_size=true`, which now works correctly.

## Testing

Comprehensive tests were added in `/home/thawkins/Projects/gcodekit5/crates/gcodekit5-designer/tests/position_size_update_test.rs`:

1. **test_resize_width_preserves_position**: Create a rectangle, change width, verify center stays fixed and new width is correct
2. **test_resize_height_preserves_position**: Create a rectangle, change height, verify center stays fixed and new height is correct  
3. **test_move_position_updates_correctly**: Verify position updates work when `update_position=true`

All tests pass, confirming:
- Rectangle center position is preserved when editing width or height
- Width and height can be changed independently
- The shape resizes to the exact dimensions specified by the user

Manual verification steps:
1. Create a rectangle at center position (50, 50) with width 60 and height 40
2. Select the rectangle  
3. Change the width to 80 in the Properties Panel
4. ✅ The rectangle center remains at (50, 50) with new width 80 and height 40
5. Change the height to 60 in the Properties Panel
6. ✅ The rectangle center remains at (50, 50) with width 80 and new height 60

## Impact

- **User Impact**: Critical - Users can now resize shapes without them moving AND shapes resize to exact dimensions
- **Breaking Changes**: None - This is a bug fix that makes behavior match user expectations
- **Performance**: No change
- **Code Quality**: Improved correctness of position/size update logic and transform system
- **Test Coverage**: Added comprehensive test suite with 3 tests covering all scenarios
