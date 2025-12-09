# Slint Focus Management Issue - Tabbed View Focus

**Issue**: [Slint GitHub Issue #10092](https://github.com/slint-ui/slint/issues/10092)

**Status**: Investigated and documented - waiting for Slint team response

## Problem Statement

We need to automatically focus a `CustomTextEdit` control in a tabbed view when:
1. User clicks the tab (view becomes visible)
2. User selects view from menu
3. Tool programmatically switches to editor view

The cursor appears and works fine with mouse clicks, but keyboard input doesn't work until the user manually clicks in the editor.

## Attempted Solutions (All Failed ❌)

### 1. Overlay Rectangle with Absolute Positioning ❌
- **Attempt**: Placed cursor outside HorizontalBox with `x:` and `y:` properties
- **Result**: Slint doesn't allow absolute positioning for children in layout containers
- **Error**: "Cannot set x/y for elements placed in this layout"

### 2. Focus Trigger Property with changed Handler ❌
- **Attempt**: Created `in-out property <int> gcode-focus-trigger` in MainWindow, added `changed focus-trigger =>` handler in CustomTextEdit, incremented trigger from menu/tab/Rust code
- **Result**: Changed handler never fired, property bindings weren't updating
- **Root Cause**: 2-way bindings (`<=>`) apparently prevent changed handlers from firing reliably

### 3. BlinkingCursor in Empty Line HorizontalLayout ❌
- **Attempt**: Added BlinkingCursor as child of HorizontalLayout in empty line
- **Result**: Cursor exists but doesn't render in empty line context

### 4. Simple Green Rectangle Cursor ❌
- **Attempt**: Replaced BlinkingCursor with simple `Rectangle { width: 2px; height: 20px; background: #00ff00; }`
- **Result**: Broke cursor blinking for lines with content

### 5. Timer with FocusScope.focus() ❌
- **Attempt**: Added timer in Rectangle's `init` callback, timer calls `FocusScope.focus()` after 50ms delay
- **Result**: Works on first view load (init runs), but fails when switching back to view
- **Key Discovery**: `if current-view == "gcode-editor"` creates Rectangle ONCE and shows/hides it. The `init` callback only runs once at creation, not on subsequent visibility changes

### 6. Using forward-focus Property ✅ (Partial Success)
- **Attempt**: Added `forward-focus: editor-focus` to GcodeEditorPanel, timer calls `gcode-editor-panel.focus()`
- **Result**: Works on first view switch! Focus properly cascades to FocusScope
- **Limitation**: Doesn't work when switching back because `if` conditional doesn't recreate element

### 7. Attempting One-Way Property ❌
- **Attempt**: Changed `in-out property <int> gcode-focus-trigger` to `in property <int> gcode-focus-request`
- **Result**: Can't use `+=` on input properties, can't use 2-way binding with input properties
- **Issue**: Slint syntax restrictions prevent this approach

### 8. Trying changed Handler on Root Property ❌
- **Attempt**: Added `changed current-view =>` at MainWindow level to detect view changes
- **Result**: Can't use changed inside conditional blocks, only at property definition level

## Key Discoveries

### 1. If-Conditional Limitation
The `if current-view == "gcode-editor" : Rectangle { }` pattern **shows/hides the Rectangle** rather than recreating it:
- `init` callbacks only run once at creation
- `changed` handlers on properties inside don't re-fire on visibility changes
- Each time the condition toggles from false→true, the same Rectangle instance is reused

### 2. 2-Way Binding Behavior
Properties bound with `<=>` don't appear to trigger `changed` handlers reliably when the binding source changes

### 3. Layout Constraints
Absolute positioning (x, y, pos-x, pos-y) doesn't work for elements in layout containers

### 4. What DOES Work
- `forward-focus` property correctly forwards focus from parent to child FocusScope
- Timer-based focus on initial view load
- Keyboard input works perfectly once focused (even with mouse)

## Current Implementation Status

**Working** ✅:
- Cursor visible at (1,1) on empty editor
- Keyboard input works on first view switch
- `forward-focus` property routes focus to FocusScope correctly

**Not Working** ❌:
- Re-focusing when switching back to gcode-editor from another view
- `changed` handlers on 2-way bound properties

**Workaround**: Users must click once in the editor to focus it, then keyboard works normally

## Recommended Solutions for Slint

### 1. Element Recreation in Conditionals
Make `if` conditionals recreate elements when the condition changes to true, not just show/hide existing elements. This would allow `init` callbacks to re-run and timers to restart.

### 2. Explicit Focus Callbacks
Add a Rust-side callback mechanism to explicitly set focus on specific UI elements by name/ID:
```rust
window.set_focus_to("element_id");
```

### 3. View/Pane Component
Create a built-in component for tabbed interfaces that handles focus management automatically

### 4. Focus Events
Add `onVisible`/`onShow` events that fire when conditionally-shown elements become visible:
```slint
if condition : SomeComponent {
    on-show => { /* runs each time visibility changes to true */ }
}
```

## Environment Details

- **Slint Version**: Latest
- **Platform**: Fedora 43 (Linux)
- **Language**: Rust + Slint
- **Project**: [gcodekit5](https://github.com/thawkins/gcodekit5) - G-code editor for CNC machines
- **Related Files**:
  The Slint files mentioned previously (e.g., `crates/gcodekit5-ui/ui.slint` and `crates/gcodekit5-ui/ui_panels/gcode_editor.slint`) have been removed as part of the Slint UI deletion effort.
  - `src/main.rs` - View switching logic

## Questions for Slint Team

1. Is element recreation in conditionals planned for a future release?
2. Is there a recommended pattern for tabbed focus management that we may have missed?
3. Would a focus callback mechanism be considered?
4. Are there any performance implications to making conditionals recreate elements?

## References

- GitHub Issue: https://github.com/slint-ui/slint/issues/10092
- Slint Focus Documentation: https://docs.slint.dev/latest/docs/slint/guide/development/focus/
- Project Repository: https://github.com/thawkins/gcodekit5
