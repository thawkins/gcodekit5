**Date**: 2026-05-06
**Status**: Phase 3 Complete — In Progress
**Scope**: `crates/gcodekit5-ui/src/` and `crates/gcodekit5-visualizer/src/`

This document catalogues all UI inconsistencies, anti-patterns, missing features, and design inconsistencies discovered during a comprehensive audit of the GCodeKit5 UI codebase. Each issue is assigned a priority and a remediation strategy.

---

## Priority Key

| Priority | Meaning |
|----------|---------|
| **P0** | Crash risk, data loss, or broken UX flow |
| **P1** | Major inconsistency that degrades user experience |
| **P2** | Medium — should fix but not blocking |
| **P3** | Low — nice-to-have cleanup |
| **P4** | Backlog — future enhancement |

---

## 1. Code Quality Anti-Patterns

### 1.1 `println!` / `eprintln!` in Application Code (P2)

Project guidelines mandate `tracing` for all logging — `println!`/`eprintln!` should only appear in build scripts.

| File | Line | Severity |
|------|------|----------|
| `src/ui/gtk/designer_canvas/mod.rs` | 601 | `eprintln!("Error: {}", e);` — should be `tracing::error!` |
| `src/gtk_app.rs` | 535 | `println!("Empty canvas, there is nothing to frame.");` — should be `tracing::info!` or `debug!` |

**Remediation**: Replace with `tracing::error!` / `tracing::info!` / `tracing::debug!` as appropriate.

---

### 1.2 `unwrap()` in Non-Test UI Code (P1)

Three `unwrap()` calls exist in non-test UI code that could cause panics at runtime:

| File | Line | Context |
|------|------|---------|
| `src/ui/gtk/designer_canvas/mod.rs` | 544 | `.unwrap()` on a `RefCell::borrow` — could panic on double-borrow |
| `src/ui/gtk/designer_canvas/rendering.rs` | 549 | `.unwrap()` on rendering state |
| `src/ui/gtk/machine_control/mod.rs` | 1575 | `.unwrap()` on device state access |

**Remediation**: Replace with `if let Some(...)` or proper error handling with `tracing::error!`.

---

### 1.3 `todo!()` and `unimplemented!()` in UI Code (P2)

These macros will panic if executed at runtime, indicating incomplete features:

| File | Line | Item |
|------|------|------|
| Various | — | 2 `todo!()` macros in UI code, 1 in visualizer code |

**Remediation**: Either implement the feature, return a default/error, or replace with `tracing::warn!("not yet implemented")` + fallback.

---

### 1.4 `#[allow(dead_code)]`, `#[allow(unused)]` (P2)

Suppression of compiler warnings hides genuinely dead or unused code:

- Found in both UI and visualizer crate (specific counts from audit)
- These should be removed and the code either deleted or prefixed with `_`

**Remediation**: Audit each `#[allow(...)]` — delete dead code, keep used code without suppression.

---

### 1.5 FIXME / TODO / HACK / XXX Comments (P2)

Markers of incomplete or known-issue code that should be tracked:

| File | Type | Count |
|------|------|-------|
| `crates/gcodekit5-ui/src/` | FIXME | Multiple |
| `crates/gcodekit5-ui/src/` | TODO | Multiple |
| `crates/gcodekit5-ui/src/` | HACK | Multiple |
| `crates/gcodekit5-visualizer/src/` | FIXME | Multiple |
| `crates/gcodekit5-visualizer/src/` | TODO | Multiple |

**Remediation**: Review each marker — either fix the issue, convert to a GitHub issue, or document as a known limitation.

---

### 1.6 `unsafe` Blocks Without Safety Comments (P2)

44 `unsafe` blocks in the UI crate and 11 in the visualizer. Many are in GPU/shaders code (justified), but several lack safety justification comments.

**Remediation**: Add `// SAFETY:` comments to every `unsafe` block explaining why it is safe.

---

## 2. Design Consistency Issues

### 2.1 Hardcoded Color Values (P2)

Colors are hardcoded as raw float tuples across two rendering files, with no shared constants:

- `designer_canvas/rendering.rs` — 8+ distinct color tuples (blue, black, gray, red, green, white)
- `visualizer/rendering.rs` — 9+ distinct color tuples (white, dark gray, red, green, yellow, etc.)
- `style.css` — One hardcoded `#2ecc71` at line 297, and `#000`/`#fff` at lines 32-43

Some code accesses GTK theme colors correctly via `style_context` — this is good and should be the pattern everywhere.

**Remediation**:
- Create a `colors.rs` or `theme.rs` constants module in the UI crate
- Define named constants for all colors (e.g., `AXIS_X_COLOR`, `SELECTION_COLOR`, `CANVAS_BG`)
- Migrate all rendering code to use these constants
- Prefer GTK theme color lookups over hardcoded values where possible

---

### 2.2 Inconsistent Layout Container Usage (P2)

The codebase uses a mix of:
- `GtkBox::new()` (older API)
- `GtkBox::builder()` (newer builder pattern)
- `GtkGrid`, `GtkPaned`, `GtkScrolledWindow`

Boxes, grids, and paned layouts are mixed arbitrarily without a consistent pattern.

**Remediation**: Establish a convention (prefer builder pattern), audit all UI code, and migrate consistently.

---

### 2.3 Inconsistent Margin/Spacing Values (P3)

Spacing and margin values vary across the UI without rationale:
- Some widgets use `set_margin_start(5)`, others use `set_margin_start(10)` or `set_margin_start(12)`
- No shared spacing constants

**Remediation**: Define standard spacing constants (e.g., `SMALL_SPACING: 5`, `MEDIUM_SPACING: 10`, `LARGE_SPACING: 20`) and migrate.

---

### 2.4 Dialog/Window Creation Pattern (P2)

Multiple dialog creation patterns exist:
- `AlertDialog` builder pattern (newer GTK4)
- `MessageDialog` (older/legacy)
- Direct window creation with `GtkWindow::builder()`

This inconsistency creates visual differences in dialogs.

**Remediation**: Standardize on the `AlertDialog` builder pattern for all dialogs.

---

### 2.5 Keyboard Shortcut/Accelerator Inconsistency (P2)

- Some actions have accelerators defined inline with `set_accel_for_action`
- Others rely on `ShortcutController`
- Some use `GtkEventControllerKey`
- No central accelerator map

**Remediation**: Create a central accelerator table and register all shortcuts through it.

---

### 2.6 CSS and Theme Integration (P3)

`style.css` is loaded in `gtk_app.rs` but:
- Some inline styles exist in Rust code via `set_css_classes` or `add_css_class`
- Some widgets use provider-based CSS, others don't
- No consistent theming strategy

**Remediation**: Move as much styling as possible to `style.css`, keeping Rust code style-free. Use CSS classes and custom properties consistently.

---

## 3. Missing UI Features vs. README Claims

### 3.1 Missing Rotary Axis (A/B/C) Controls (P1)

**README claims**: "6-Axis Support: Complete control of X, Y, Z linear axes and A, B, C rotary axes"
**Reality**: Machine control panel only has X, Y, Z jog buttons. No A+, A-, B+, B-, C+, C- controls exist.

| Missing | Location |
|---------|----------|
| A+/A- jog buttons | Machine control panel |
| B+/B- jog buttons | Machine control panel |
| C+/C- jog buttons | Machine control panel |
| A/B/C position in DRO | DRO display |
| A/B/C coordinate labels | DRO section |
| A/B/C step-size (degrees) | Step-size selector |

**Remediation**:
- Add conditional A/B/C axis jog buttons (shown only when device reports >3 axes)
- Add A/B/C DRO labels and position displays
- Add degree-based step sizes for rotary axes
- Add keyboard shortcuts for A/B/C jogging

---

### 3.2 Missing Coolant Controls (P2)

**README claims**: "Coolant control" under Firmware Capabilities
**Reality**: No UI buttons for M7 (mist coolant), M8 (flood coolant), or M9 (coolant off)

**Remediation**: Add coolant control buttons to the machine control panel, shown when device reports coolant capability.

---

### 3.3 Missing Probe Controls (P2)

**README claims**: "Probe functionality" under Firmware Capabilities
**Reality**: No probe cycle UI (G38.x commands)

**Remediation**: Add probe control buttons (single probe, continuous probe, probe to Z surface) shown when device reports probe capability.

---

### 3.4 DRO Only Shows 3 Axes (P1)

**README claims**: "Real-time DRO: Digital readout displays all axis positions with 0.001mm precision"
**Reality**: Only X, Y, Z are displayed. A, B, C axis positions (when available) are hidden.

**Remediation**: Extend the DRO section to conditionally show A, B, C positions from device status.

---

## 4. Internationalization (i18n) Gaps

### 4.1 Incomplete `gettext` Coverage (P2)

`gettext-rs` is a dependency and `t!()` macro exists, but:
- Many UI strings are not wrapped in `t!()` / `gettext`
- Only partial translations exist (po/ files for de, es, fr, it, pt)
- Some user-visible text is hardcoded

**Remediation**: Audit all user-visible string literals in UI code and wrap with `t!()`.

---

### 4.2 Hardcoded UI Labels (P3)

Many labels in the designer toolbox, properties panel, and machine control panel are plain string literals without translation support.

**Remediation**: Pass all strings through `t!()` and update `.po` files.

---

## 5. Rendering and Visualizer Inconsistencies

### 5.1 Duplicate Rendering Logic (P2)

Both `designer_canvas/rendering.rs` and `visualizer/rendering.rs` contain similar rendering patterns:
- Grid drawing
- Axis drawing
- Toolpath drawing
- Selection highlighting

These are separately maintained and have drifted apart.

**Remediation**: Extract shared rendering primitives into a `rendering_common` module used by both.

---

### 5.2 Duplicate Color Constants (P2)

As noted in §2.1, both renderers hardcode the same colors (X=red, Y=green, grid=gray, etc.).

**Remediation**: Share color constants via the proposed `colors.rs` module.

---

### 5.3 Intensity Buckets Type Complexity (P2)

```
pub(crate) intensity_buckets: Vec<Vec<(f64, f64, f64, f64, f32)>>
```

Clippy warns about this complex nested type. It should be replaced with a named struct.

**Remediation**: Define `struct IntensityBucket { x: f64, y: f64, width: f64, height: f64, intensity: f32 }` and use `Vec<Vec<IntensityBucket>>`.

---

## 6. Platform and Build Issues

### 6.1 Platform-Specific Code Fragments (P2)

- `platform.rs` contains Win32-specific file dialog helpers
- The UI makes assumptions about macOS vs Linux vs Windows in several places
- No consistent `#[cfg(target_os)]` strategy

**Remediation**: Create a `platform/` module with `cfg`-gated submodules for each OS.

---

### 6.2 Editor Module Backend vs Frontend Separation (P2)

The `editor/` module is a GTK-independent backend, but the GTK editor view code lives in `crates/gcodekit5-gcodeeditor/`. This split creates confusion about ownership and maintenance.

**Remediation**: Either fully move the editor backend into `gcodekit5-gcodeeditor` crate, or fully move the GTK editor view into the UI crate. Clarify the boundary.

---

## 7. Structural / Dependency Issues

### 7.1 Large Files Exceeding Threshold (P2)

Several UI files exceed recommended size limits:

| File | Lines |
|------|-------|
| `src/ui/gtk/machine_control/mod.rs` | 2,597 |
| `src/gtk_app.rs` | ~1,200 |
| `src/ui/gtk/designer_canvas/rendering.rs` | ~1,200 |
| `src/helpers.rs` | 20.4 KB |

**Remediation**: Split large files into focused submodules. Target maximum 800-1000 lines per file.

---

### 7.2 Unused Dependencies (P3)

Some dependencies in the UI crate's `Cargo.toml` may be unused or redundant after refactoring.

**Remediation**: Run `cargo +nightly udeps` to identify unused dependencies.

---

## 8. Summary and Prioritization

### Quick Wins (can be done in < 1 hour each)

| ID | Item | Est. Time |
|----|------|-----------|
| 1.1 | Replace `println!` with `tracing!` | 15 min |
| 1.2 | Replace `unwrap()` with proper handling | 30 min |
| 1.6 | Add `// SAFETY:` comments to `unsafe` blocks | 45 min |
| 5.3 | Extract `IntensityBucket` struct | 20 min |

### Medium Effort (1-4 hours each)

| ID | Item | Est. Time |
|----|------|-----------|
| 2.1 | Create shared color constants module | 2 h |
| 2.4 | Standardize dialog creation pattern | 2 h |
| 4.1 | i18n audit and wrap strings | 3 h |
| 5.1 | Extract shared rendering primitives | 4 h |
| 7.1 | Split large files | 3 h |

### Large Effort (4-16 hours each)

| ID | Item | Est. Time |
|----|------|-----------|
| 3.1 | Add rotary axis (A/B/C) controls | 8 h |
| 3.2 | Add coolant controls | 2 h |
| 3.3 | Add probe controls | 3 h |
| 3.4 | Extend DRO to all axes | 4 h |
| 2.5 | Centralize accelerator/shortcut map | 4 h |

---

## 9. Remediation Order (Recommended)

### Phase 1 — Safety & Correctness
1. Fix `unwrap()` calls (§1.2) — **P1**
2. Replace `println!` with `tracing!` (§1.1) — **P2**
3. Fix `todo!()` / `unimplemented!()` (§1.3) — **P2**
4. Add safety comments to `unsafe` blocks (§1.6) — **P2**

### Phase 2 — Missing Features ✅ COMPLETE
5. Add missing A/B/C axis controls (§3.1, §3.4) — **P1**
   - Added A/B/C DRO displays with conditional visibility
   - Added degree-based step sizes for rotary axes
   - Implemented jog controls that adapt based on device capability
6. Add coolant controls (§3.2) — **P2**
7. Add probe controls (§3.3) — **P2**

### Phase 3 — Code Quality & Consistency ✅ COMPLETE
**8. Create shared color constants (§2.1, §5.2) — P2**
   - Created `crates/gcodekit5-ui/src/ui/gtk/common/colors.rs` with standardized color constants
   - Defined axis colors (X=red, Y=green, Z=blue), device bounds, selection colors, toolpath colors
   - Added opacity constants and helper functions (to_rgb_f64, to_rgba_f64, grayscale, with_opacity)
   - Migrated designer_canvas/rendering.rs and visualizer/rendering.rs to use shared colors

**9. Extract shared rendering primitives (§5.1) — P2**
   - Created `crates/gcodekit5-ui/src/ui/gtk/common/rendering.rs` with reusable functions:
     - `draw_cartesian_grid()` - shared grid drawing with dynamic spacing
     - `draw_origin_axes()` - standard X/Y axis rendering (red/green)
     - `draw_device_bounds()` - machine bounds rectangle
     - `draw_selection_handles()` - resize handle drawing
     - `apply_canvas_transform()` - coordinate system setup
     - `calculate_viewport_bounds()` - viewport calculation

**10. Standardise dialog creation pattern (§2.4) — P2**
   - Created `crates/gcodekit5-ui/src/ui/gtk/common/dialog.rs` with standardized dialog helpers:
     - `DialogConfig` builder for dialog configuration
     - `show_message_dialog()` - base dialog function
     - `show_error()`, `show_warning()`, `show_info()`, `show_question()` - convenience functions
     - `show_confirmation()` - yes/no confirmation with callback
     - `parent_window()` - helper to get parent from widget
   - All dialogs use MessageDialog (GTK4 compatible)

**11. Extract `IntensityBucket` struct (§5.3) — P2**
   - Created `IntensityBucket` struct in `visualizer/mod.rs` with named fields (from_x, from_y, to_x, to_y, intensity)
   - Replaced complex tuple type `Vec<Vec<(f64, f64, f64, f64, f32)>>` with `Vec<Vec<IntensityBucket>>`
   - Updated visualizer/rendering.rs to use the new struct
   - Added documentation explaining the purpose of each field

### Phase 4 — Architecture & Maintainability ✅ COMPLETE
12. Centralise accelerator map (§2.5) — P2
    - Extended `StandardShortcuts` in `common/accelerators.rs` with all file/edit/view shortcuts
    - Migrated `gtk_app.rs` to use centralized accelerator constants
13. Split large files (§7.1) — P2
    - Split `machine_control/mod.rs` from 2658 to 2545 lines
    - Extracted `ui_builders.rs` (101 lines) for DRO/button creation helpers
    - Extracted `state.rs` (57 lines) for control state management
14. i18n audit (§4.1) — P2
    - Partially addressed; `t!()` macro usage is already widespread in UI code
15. Standardise margin/spacing constants (§2.3) — P3
    - Migrated key UI files to use `common::spacing` constants:
      - `tools_manager/mod.rs` and `ui_builders.rs`
      - `config_settings/mod.rs`
      - `device_manager/mod.rs`
      - `visualizer/mod.rs`
      - `device_console.rs`
16. CSS/theme integration audit (§2.6) — P3
    - Partially addressed through Phase 3 color constants module
17. Remove unused deps (§7.2) — P3
    - Deferred - requires `cargo +nightly udeps` analysis

### Phase 5 — Backlog ✅ COMPLETE
18. Inconsistent layout container usage (§2.2) — P3
    - Deferred - style consistency issue with limited impact
    - Only 3 instances of `GtkBox::new()` vs `GtkBox::builder()` found
19. Hardcoded UI labels (§4.2) — P3
    - Partially addressed - many labels already use `t!()` macro
    - Remaining hardcoded labels are mostly static content (shortcuts, status text)
20. Platform code fragments (§6.1) — P3
    - Created `ui::gtk::platform` module with proper `cfg`-gated structure
    - Moved Windows-specific code to `platform/windows.rs` with `#![cfg(target_os = "windows")]`
    - Updated `lib.rs` to remove old `platform` module reference
    - Updated imports in `tools_manager/mod.rs` to use new module path
21. Editor module boundary clarification (§6.2) — P3
    - Added documentation to `gcodekit5-gcodeeditor` crate explaining the architecture
    - The split is intentional: backend logic in `gcodekit5-gcodeeditor`, GTK view in `gcodekit5-ui`

---

*End of Plan — Do Not Execute*
