# GCodeKit5 - Codebase Improvements and Recommendations

**Document Version**: 2.0  
**Date**: February 2026  
**Analysis Scope**: 130,212 lines of Rust code across 9 crates, 430 source files

---

## Executive Summary

GCodeKit5 is a well-structured, modular Rust project with solid architectural foundations. As the codebase has grown to 130K+ lines across 430 files, there are opportunities to improve code quality, maintainability, and robustness. This document identifies 50+ actionable improvements across 9 categories, prioritized by impact and effort.

### Key Statistics
- **Total Lines**: 130,212 (Rust)
- **Crates**: 9 modular crates with clear responsibilities
- **Source Files**: 430 `.rs` files
- **Test Functions**: ~791 across all crates (116 core, 205 communication, 274 designer, 56 camtools, 96 visualizer, 44 ui)
- **Clippy Warnings**: 137+ active warnings (92 in designer alone, 26 in camtools, 13 in core)
- **Clippy Errors**: 1 hard error in gcodekit5-designer (loop condition mutation)
- **Largest Files**: 3,907 lines (designer_canvas.rs), 3,836 lines (visualizer.rs), 2,720 lines (machine_control.rs)
- **Files >1000 LOC**: 20+
- **`#[allow(...)]` suppressions**: ~156 across 21 files
- **`unsafe` blocks**: ~65 across 11 files (mostly OpenGL rendering)
- **Public APIs**: 165+ in core crate alone, ~20-30% undocumented

---

## 1. Error Handling & Robustness (CRITICAL)

### 1.1 Reduce Unsafe Unwrap/Expect Calls ✅ COMPLETED (Feb 2026)
**Previous State**: Heavy `.expect()` usage in UI layer and active code; `.unwrap()` in legacy and active code  
**Current State**: All production-code runtime `expect()`/`unwrap()` calls that could panic have been replaced with safe alternatives  
**Impact**: High - Eliminated runtime panic risk in production  
**Effort**: Medium

**What was done**:
- Replaced 23 production `expect()` calls across 14 files with safe error handling
- Used `if let Some(...)`, `unwrap_or()`, `unwrap_or_else()`, early returns, and `let ... else` patterns
- Critical fixes in: pocket_operations.rs, parametric_shapes.rs, toolpath.rs (empty collection guards)
- Critical fixes in: gtk_app.rs (init/resource/display failure handling)
- Critical fixes in: visualizer.rs (CString null-byte safety, Cairo error handling)
- Critical fixes in: response parsers (JSON key iteration safety)
- Critical fixes in: buffered.rs, advanced_features.rs (queue/map access safety)

**Remaining `expect()` calls (all safe)**:
- Regex::new() on hardcoded literal patterns (7 calls across stats.rs, vector_engraver.rs, gerber.rs, gcode/mod.rs)
- build.rs (5 calls — build-time only, not runtime)
- Test code only (~70 calls — appropriate for test assertions)
- Logically guarded (undo_manager.rs — length checked before `.next()`)

---

### 1.2 Implement Comprehensive Error Types ✅ COMPLETED (Feb 2026)
**Previous State**: 5 of 9 crates had `thiserror` error types; 4 crates (camtools, devicedb, gcodeeditor, settings) lacked structured errors  
**Current State**: All 9 crates now have `error.rs` with domain-specific `thiserror` error types  
**Impact**: Medium - Enables typed error handling and better error context  
**Effort**: Medium

**What was done**:
- Created `error.rs` for 4 crates following established patterns from designer/communication crates
- **camtools**: `CamToolError` (11 variants), `ParameterError` (5 variants), `FileFormatError` (6 variants)
- **devicedb**: `DeviceError` (8 variants), `ProfileError` (6 variants)
- **gcodeeditor**: `EditorError` (7 variants), `BufferError` (5 variants)
- **settings**: `SettingsError` (9 variants), `ConfigError` (5 variants), `PersistenceError` (5 variants)
- Added `thiserror` dependency to all 4 crate Cargo.toml files
- Registered modules in lib.rs with public re-exports and Result type aliases
- All error types include: `#[derive(Error, Debug)]`, doc comments, `#[from]` conversions, unit tests

**Remaining work**: Gradually migrate existing `Result<T, String>` and `anyhow::Result` return types to use the new typed errors

---

### 1.3 Add Null/Invalid State Guards ✅ COMPLETED (Feb 2026)
**Previous State**: No validation of state transitions or CNC parameters  
**Current State**: `debug_assert!` guards on all critical CNC parameters; state transition validation methods; runtime clamping for division-by-zero protection  
**Impact**: Medium - Prevents subtle bugs and NaN propagation  
**Effort**: Low

**What was done**:
- **CNC parameter guards** (debug_assert + runtime clamp where needed):
  - multipass.rs: `max_depth_per_pass` division-by-zero guard, constructor + setter validation
  - toolpath.rs: `Toolpath::new()`, `set_feed_rate()`, `set_tool_diameter()`, `set_cut_depth()`, `set_step_in()` (with safe clamp)
  - pocket_operations.rs: Constructor, `set_parameters()`, `set_ramp_angle()` (clamped 0-90°)
  - drilling_patterns.rs: Constructor, `set_parameters()`, `set_peck_drilling()`
  - vcarve.rs: `VBitTool::new()` — tip_angle, diameter, cutting_length
  - arrays.rs: Linear, Circular, Grid constructors — spacing, radius, count validation
  - designer_state/mod.rs: `set_feed_rate()`, `set_tool_diameter()`, `set_cut_depth()`, `set_step_down()`
- **NaN/Infinity guards**: `CNCPoint::with_axes()` and `set_axes()` — all 6 axes validated
- **State machine validation**:
  - `ControllerState::can_transition_to()` — validates CNC state machine transitions (11 states)
  - `CommunicatorState::can_transition_to()` — validates communication layer transitions (5 states)
  - `CommandState` mark_* methods — debug_assert on expected source states
- **Silent fallback warnings**:
  - GRBL controller: `tracing::warn!` on unknown machine state (was silent Idle fallback)
  - DesignerState: `tracing::warn!` on unknown drawing mode (was silent Select fallback)

---

## 2. Code Quality & Maintenance (HIGH PRIORITY)

### 2.1 Address Clippy Warnings ✅ COMPLETED
**Previous State**: 155+ active Clippy warnings across 9 crates, plus 1 hard error in designer  
**Current State**: **0 warnings, 0 errors** across all crates  
**Impact**: Code health, CI readiness  

**What Was Fixed**:
- 1 hard error: `while_immutable_condition` in pocket_operations.rs (while true → loop)
- 155+ warnings across 9 crates, including:
  - gcodekit5-designer: 93 warnings (auto-fix + manual)
  - gcodekit5-ui: 150 warnings (auto-fix + manual: type_complexity, too_many_arguments, unnecessary_cast, etc.)
  - gcodekit5-camtools: 26 warnings
  - gcodekit5-communication: 19 warnings (strip_prefix, clamp, io::Error::other, derivable_impls)
  - gcodekit5-core: 14 warnings (needless_update, match_like_matches)
  - gcodekit5-visualizer: 16 warnings (redundant closures, Default impls, collapsible blocks)
  - gcodekit5-gcodeeditor: 3 warnings
  - gcodekit5-settings: 2 warnings
  - gcodekit5-ui build.rs: 1 warning

**Strategy Used**:
- `#[allow(clippy::type_complexity)]` for Rc<RefCell<Option<Box<dyn Fn(...)>>>> patterns (47 instances)
- `#[allow(clippy::too_many_arguments)]` for GTK callback functions (10 instances)
- `.clamp()`, `.strip_prefix()`, `io::Error::other()` for idiomatic Rust
- `#[derive(Default)]` replacing manual impls
- `Display` trait replacing inherent `to_string()`
- `FromStr` trait replacing inherent `from_str()`

---

### 2.2 Reduce Cognitive Complexity — Split Large Files ✅ COMPLETED
**Previous State**: 20 files exceeded 1,000 lines; top file was 3,907 lines  
**Current State**: 13 files exceed 1,000 lines; top file is 2,898 lines  
**Impact**: High - Improved maintainability  

**Files Split** (13 monolithic files → 40+ focused modules):

| Original File | Before | After (mod.rs) | Sub-modules |
|---|---|---|---|
| designer_canvas.rs | 3,907 | 529 | input.rs, editing.rs, rendering.rs, toolpath_preview.rs |
| visualizer.rs | 3,836 | 2,898 | gl_loader.rs, rendering.rs, interaction.rs |
| machine_control.rs | 2,720 | 2,422 | overrides.rs, operations.rs |
| model.rs | 2,524 | 302 | 10 shape files (rectangle, circle, path, etc.) |
| designer.rs | 2,008 | 655 | ui_builders.rs, file_ops.rs |
| tools_manager.rs | 1,938 | 1,299 | ui_builders.rs, event_handlers.rs |
| gcode/mod.rs | 1,798 | 21 | command.rs, parser.rs, pipeline.rs, processors.rs |
| toolpath.rs | 1,759 | 52 | segment.rs, generator.rs |
| config_settings.rs | 1,574 | 456 | grbl_settings.rs, operations.rs |
| canvas.rs | 1,564 | 370 | types.rs, operations.rs |
| designer_properties/mod.rs | 1,536 | 596 | builders.rs, update.rs |
| tabbed_box.rs | 1,485 | 1,313 | types.rs |
| device_manager.rs | 1,476 | 422 | tabs.rs, operations.rs |

**Strategy Used**:
- Directory modules (`foo.rs` → `foo/mod.rs` + sub-files)
- `pub(crate)` for struct fields accessed by sub-module impl blocks
- `use super::*;` in sub-modules for parent module access
- `pub use` re-exports to preserve public API
- No external import changes required

**Remaining large files** (not split — single-purpose or mostly constructor):
- `visualizer/mod.rs` (2,898) — ~1,145-line `new()` constructor
- `machine_control/mod.rs` (2,422) — ~2,260-line `new()` constructor
- These are GTK4 widget constructors that can't be split without restructuring

---

### 2.3 Eliminate Temporary Debug Code ✅ COMPLETED
**Previous State**: `println!`/`eprintln!` found in 17 files including build scripts and tests  
**Current State**: **0 debug println/eprintln** in production or test code  
**Impact**: Clean logs, AGENTS.md compliance  

**What Was Fixed**:
- `hatch_test.rs` — Removed 2 println! calls (error info now in panic message)
- `test_virtual_ports.rs` — Removed 4 println! calls (test relies on assertions only)
- `model_verification.rs` — Removed 1 println! call (assert messages show values on failure)
- `i18n.rs` — Kept 2 eprintln! calls (documented: runs before tracing is initialized)
- `build.rs` — Kept 5 println! calls (cargo build protocol requirement)
- No `dbg!()` macros found in codebase
- 46 `debug!()` calls reviewed — all intentional (STL import, G-code parsing, etc.)

---

### 2.4 Complete TODO/FIXME Items ✅ COMPLETED
**Current State**: Reduced from 15 to 9 TODOs (all remaining are feature work tracked in GitHub issues)  
**Impact**: Medium - Technical debt  
**Effort**: Variable

**Implemented** (6 TODOs resolved):
1. ✅ `editor.rs` - **TODO(#15)**: Added error dialogs for file read/save failures using `MessageDialog`
2. ✅ `firmware/settings.rs` - **TODO(#13)**: Implemented JSON-based file loading/saving for `DefaultFirmwareSettings`
3. ✅ `visualizer/mod.rs` - **TODO(#18)**: Added dirty flag to `Visualizer` struct; render loop now skips buffer regeneration when data unchanged

**Remaining** (9 TODOs — feature work, properly tracked in GitHub issues):
- `TODO(#16)` — 3D mesh preview integration (1 item in file_ops.rs)
- `TODO(#17)` — File operations alignment with shape structures (1 item in file_ops.rs)
- `TODO(#19)` — 3D rendering features in scene3d.rs (7 items: toolpath bounds, shape types, rendering, shadow projections, mesh IDs, camera positioning, path conversion)

---

## 3. Type System & API Design (MEDIUM PRIORITY)

### 3.1 Reduce Complex Type Nesting
**Current State**: 192 instances of `Box<dyn>`, `Rc<RefCell>`, `Arc<Mutex>` patterns  
**Impact**: Medium - Impacts readability and performance  
**Effort**: Medium-High

**Problem Pattern**:
```rust
// Hard to read and reason about
Rc<RefCell<Box<dyn Component>>>

// Common in:
// - UI callback chains (need flexibility)
// - State management (multiple owners)
// - Channel types
```

**Solutions**:
1. **Type Aliases**: Create readable aliases
```rust
type ComponentRef = Rc<RefCell<Box<dyn Component>>>;
```

2. **Owned vs Borrowed**: Reconsider ownership in specific contexts
```rust
// Instead of Arc<Mutex<State>>, use Arc<State> with interior mutability where needed
```

3. **Trait Objects**: Use concrete types when possible
```rust
// If only one implementation, avoid Box<dyn T>
struct Canvas { /* ... */ }  // Better than Box<dyn Drawable>
```

**Priority Areas**:
- UI callback chains (use Box<dyn Fn()> → consider event system)
- State management (use dedicated patterns like signals/events)
- Generator/visitor patterns

---

### 3.2 Improve Public API Documentation
**Current State**: Core crate has 165+ public APIs with inconsistent documentation  
**Impact**: Medium - Developer experience  
**Effort**: Medium

**Current Coverage**:
- `//!` crate docs: Present
- `///` function docs: ~60% coverage
- Examples in docs: ~10% coverage

**Targets**:
```
1. Document all public types and methods
2. Add examples for major types (State, Commands, Traits)
3. Add "Error cases" section to fallible functions
4. Create module-level overview docs
```

**Example**:
```rust
/// Represents a CNC machine position in work coordinates.
///
/// # Fields
/// - `x`, `y`, `z`: Linear axes in millimeters
/// - `a`, `b`, `c`: Rotary axes in degrees
///
/// # Example
/// ```
/// let pos = Position::new(10.0, 20.0, 0.0, 0.0, 0.0, 0.0);
/// assert_eq!(pos.x, 10.0);
/// ```
pub struct Position {
    // ...
}
```

---

### 3.3 Create Builder Pattern for Complex Types
**Current State**: Many types use Default + field assignment  
**Impact**: Low-Medium - API consistency  
**Effort**: Low

**Examples**:
- `ControllerStatus` - 12+ optional fields
- `ToolpathSettings` - 8+ configuration parameters
- `CAMToolParameters` - Multiple tool-specific configs

**Pattern**:
```rust
let settings = ToolpathSettingsBuilder::new()
    .with_feed_rate(100.0)
    .with_spindle_speed(5000)
    .with_depth_of_cut(2.0)
    .build()?;
```

---

## 4. Testing & Coverage (HIGH PRIORITY)

### 4.1 Establish Testing Strategy
**Current State**: 791+ test functions with good overall coverage but gaps in some crates  
**Impact**: High - Catch regressions early  
**Effort**: Medium-High (ongoing)

**Current Coverage by Crate** (Feb 2026):
```
gcodekit5-designer:       274 tests (strongest)
gcodekit5-communication:  205 tests (strong)
gcodekit5-core:           116 tests (good)
gcodekit5-visualizer:      96 tests (moderate)
gcodekit5-camtools:        56 tests (needs more)
gcodekit5-ui:              44 tests (needs more)
gcodekit5-gcodeeditor:      5 tests (WEAK)
gcodekit5-settings:         2 test files (WEAK)
gcodekit5-devicedb:         1 integration test (WEAK)
```

**Current Gaps**:
```
1. gcodekit5-gcodeeditor - Only 5 tests for rope-based text editor (critical component)
2. gcodekit5-settings - Only 2 test files for persistence/view model
3. gcodekit5-devicedb - Only 1 integration test for device database
4. No benchmarks in any crate (benches/ directories are empty)
5. No property-based or fuzz testing
6. No mutation testing
```

**Recommendations**:

**Phase 1 - Core Coverage**:
- Add integration tests for Designer operations (copy, paste, delete, group)
- Test toolpath generation with various shape combinations
- Add property-based tests for geometry operations
- Create test harness for communication protocols

**Phase 2 - Advanced**:
- Add fuzzing for parser and G-code generation
- Implement snapshot tests for complex outputs
- Add performance regression tests

**Phase 3 - CI/CD**:
- Enforce minimum coverage (80%) in critical crates
- Run tests on multiple configurations (x86_64, ARM64)
- Add test environment isolation

**Tool Stack**:
- `proptest` - Property-based testing
- `cargo-tarpaulin` - Coverage measurement
- `criterion` - Performance benchmarks
- `quickcheck` - Fuzzing alternative

---

### 4.2 Add Comprehensive Integration Tests
**Current State**: Limited cross-crate integration testing  
**Impact**: High - Catches architectural issues  
**Effort**: Medium

**Test Scenarios**:
```
1. Full workflow: Design → Toolpath → G-code → Visualization
2. State consistency: Operations that modify state + undo/redo
3. Error recovery: Network interruption during file send
4. Large file handling: 10K+ line G-code files
5. Concurrent operations: Multiple machine interactions
```

---

### 4.3 Implement Mutation Testing
**Current State**: No mutation testing in place  
**Impact**: Medium - Validates test effectiveness  
**Effort**: Low

**Approach**:
- Use `cargo-mutants` to identify weak tests
- Run periodically (not in every CI run)
- Target: 85%+ mutation kill rate in core crates

---

## 5. Performance & Optimization (MEDIUM PRIORITY)

### 5.1 Profile and Optimize Hot Paths
**Current State**: Some profiling done, but not systematic  
**Impact**: Medium-High - Improves responsiveness  
**Effort**: Medium

**Known Hot Paths**:
1. **Toolpath Generation**: Can be slow for complex shapes
2. **Visualizer Rendering**: Large G-code file rendering
3. **Designer Hit Testing**: Shape selection with 1000+ shapes
4. **Settings Serialization**: Happens on every config change

**Profiling Strategy**:
```bash
# Use perf on Linux, Instruments on macOS
cargo build --release
perf record ./target/release/gcodekit5
perf report

# Or use flamegraph
cargo install flamegraph
cargo flamegraph --bin gcodekit5
```

**Quick Wins**:
- Cache hit-test results (invalidate on shape changes)
- Batch settings updates (don't serialize after each field change)
- Use SIMD for geometry calculations where applicable
- Consider memory pool for frequently allocated objects

---

### 5.2 Optimize Memory Usage
**Current State**: No explicit memory profiling  
**Impact**: Medium - Improves performance on constrained systems  
**Effort**: Low-Medium

**Opportunities**:
1. **String Allocations**: Intern frequently-used strings
2. **Geometry Vectors**: Use `SmallVec<[T; 16]>` for points that rarely exceed 16
3. **Arc vs Rc**: Audit for unnecessary Arcs (single-threaded context)
4. **Clone Overhead**: Profile and reduce clones in hot paths

---

### 5.3 Add Performance Benchmarks
**Current State**: No benchmarks in repo  
**Impact**: Low (quality metric) but important for tracking  
**Effort**: Low

**Add** `benches/` directory with criterion benchmarks for:
- Toolpath generation with varying complexity
- G-code parsing with different file sizes
- Geometry operation performance
- State update speed

---

## 6. Architecture & Design Patterns (MEDIUM PRIORITY)

### 6.1 Implement Event Bus/Signal System
**Current State**: Callback chains and direct coupling in many places  
**Impact**: High - Improves decoupling  
**Effort**: Medium-High

**Problem**:
```rust
// Current: Direct coupling
designer.on_gcode_generated(|gcode| { visualizer.load(gcode); });
designer.on_selection_changed(|shapes| { properties.update(shapes); });
// Multiple handlers per event → hard to manage
```

**Solution**:
```rust
// Event bus pattern
pub enum DesignerEvent {
    GcodeGenerated(String),
    SelectionChanged(Vec<ShapeId>),
    ShapeCreated(ShapeId),
    StateChanged(DesignerStateChange),
}

pub trait EventSubscriber {
    fn on_event(&mut self, event: DesignerEvent);
}

pub struct EventBus {
    subscribers: Vec<Box<dyn EventSubscriber>>,
}
```

**Benefits**:
- Easier to add new subscribers
- Decoupled components
- Simpler testing (mock event bus)
- Better for undo/redo logging

---

### 6.2 Separate UI Logic from Business Logic
**Current State**: Some business logic embedded in UI callbacks  
**Impact**: High - Improves testability  
**Effort**: Medium-High

**Problem Areas**:
- Designer operations mixed with GTK code
- Visualizer rendering mixed with state updates
- Settings changes with immediate UI updates

**Solution**:
- Create `business_logic` modules for each feature
- UI layer becomes thin: receive events, call business logic, update display
- Example:
```rust
// business_logic/designer_operations.rs
pub fn copy_selected_shapes(
    state: &DesignerState,
    selection: &[ShapeId],
) -> Result<Vec<DesignerShape>> {
    // Pure business logic, no GTK code
}

// ui/gtk/designer.rs
copy_button.connect_clicked({
    let state = state.clone();
    move |_| {
        match copy_selected_shapes(&state, &selection) {
            Ok(shapes) => clipboard.set(shapes),
            Err(e) => show_error(&e),
        }
    }
});
```

---

### 6.3 Implement Plugin/Extension System
**Current State**: Hardcoded CAM tools, preprocessors  
**Impact**: Low-Medium (future extensibility)  
**Effort**: High

**Vision**:
- Load plugins from `~/.gcodekit5/plugins/`
- Plugin trait for CAM tools, preprocessors
- Currently: add new tool → edit code → recompile
- Future: User downloads plugin, drops in folder

**Phased Approach**:
1. Define plugin trait (Year 2)
2. Refactor existing tools to use trait
3. Add plugin loader (Year 3)

---

## 7. Dependency Management & Maintenance (LOW-MEDIUM PRIORITY)

### 7.1 Audit and Minimize Dependencies
**Current State**: Cargo.lock likely has 100+ total dependencies  
**Impact**: Low - But important for security and build times  
**Effort**: Low

**Actions**:
- Run `cargo tree` and review dependencies
- Check for duplicate versions of same crate
- Identify unused dependencies: `cargo udeps`
- Consider replacing multi-purpose crates with focused ones

---

### 7.2 Keep Dependencies Updated
**Current State**: Likely several months behind on some deps  
**Impact**: Medium - Security and feature updates  
**Effort**: Low-Medium

**Strategy**:
- Automated: `dependabot` on GitHub
- Monthly manual audit: `cargo outdated`
- Test new versions in CI before merging
- Document breaking changes in CHANGELOG

---

### 7.3 MSRV is Already Defined ✅
**Current State**: `rust-version = "1.88"` is set in workspace Cargo.toml; CI verifies via `msrv.yml` workflow  
**Status**: COMPLETE - No action needed

---

## 8. Documentation & Developer Experience (LOW PRIORITY)

### 8.1 Create Architecture Decision Records (ADRs)
**Current State**: Some design decisions in comments or GTK4.md  
**Impact**: Low-Medium - Reduces future confusion  
**Effort**: Low

**Create `docs/adr/` directory** with decisions like:
- `0001_gtk4_coordinate_systems.md` - Why Y-flip is needed
- `0002_paned_layout_for_resizable_panels.md` - Recent change
- `0003_modular_crates_structure.md` - Architecture rationale

**Template**:
```markdown
# ADR-###: Title

## Status: Accepted

## Context
(Why was this decision needed?)

## Decision
(What was decided?)

## Consequences
(Positive and negative impacts)

## Alternatives Considered
(Other options explored)
```

---

### 8.2 Improve Module-Level Documentation
**Current State**: Many modules lack overview comments  
**Impact**: Low - Developer experience  
**Effort**: Low

**Add to each module**:
```rust
//! # Designer State
//!
//! This module manages the state of the visual designer, including:
//! - Active shapes and selection
//! - Undo/redo history
//! - Canvas viewport (zoom, pan)
//! - Tool state (active tool, tool parameters)
//!
//! ## Thread Safety
//! DesignerState uses interior mutability (RefCell) and is not Send/Sync.
//! Access from UI thread only.
```

---

### 8.3 Create User & Developer Guides
**Current State**: README exists but limited developer docs  
**Impact**: Low - New contributor onboarding  
**Effort**: Low-Medium

**Create**:
- `DEVELOPMENT.md` - Setup, build, test guide
- `CONTRIBUTING.md` - Code style, PR process
- `ARCHITECTURE.md` - System overview (expand GTK4.md)
- Example plugins/custom tools guide

---

## 9. Tooling & Configuration (NEW — Feb 2026)

### 9.1 Add rustfmt.toml Configuration
**Current State**: No `.rustfmt.toml` or `rustfmt.toml` found — using rustfmt defaults  
**Impact**: Medium — AGENTS.md specifies "4 spaces, max 100 width, reorder_imports=true, Unix newlines" but no config enforces it  
**Effort**: Low

**Create `rustfmt.toml`** in project root:
```toml
edition = "2021"
max_width = 100
tab_spaces = 4
reorder_imports = true
newline_style = "Unix"
```

---

### 9.2 Add clippy.toml Configuration
**Current State**: No `.clippy.toml` or `clippy.toml` found — using clippy defaults  
**Impact**: Low — AGENTS.md specifies "cognitive complexity ≤30, warn on missing docs" but no config enforces it  
**Effort**: Low

**Create `clippy.toml`** in project root:
```toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 7
```

---

### 9.3 Fix Communication→Visualizer Coupling
**Current State**: `gcodekit5-communication` depends on `gcodekit5-visualizer`, creating tighter coupling than expected for a protocol/transport crate  
**Impact**: Medium — Makes the communication layer harder to reuse independently  
**Effort**: Medium

**Dependency Graph**:
```
core ← (foundation, no internal deps)
 ↑
 ├── designer ← core only
 ├── visualizer ← core, designer
 ├── communication ← core, visualizer  ← ⚠️ UNEXPECTED
 ├── camtools ← core, devicedb
 ├── settings ← core
 ├── devicedb ← (standalone)
 ├── gcodeeditor ← (standalone, only ropey)
 └── ui ← ALL crates (orchestration)
```

**Recommendation**: Extract the shared types that communication and visualizer both need into `gcodekit5-core`, then remove the communication→visualizer dependency.

---

### 9.4 Reduce `#[allow(...)]` Suppressions
**Current State**: ~156 `#[allow(...)]` attributes across 21 files, heavily concentrated in UI  
**Impact**: Low-Medium — Suppressions mask real issues  
**Effort**: Medium

**Top offenders**:
```
gcodekit5-ui/src/ui/gtk/visualizer.rs:       285 allow attributes
gcodekit5-ui/src/ui/gtk/designer_properties:  117 allow attributes
gcodekit5-ui/src/gtk_app.rs:                   97 allow attributes
```

**Recommendation**: Audit each `#[allow(...)]` — fix the underlying issue where possible, or add a comment explaining why the suppression is necessary.

---

### 9.5 Consolidate Wildcard Imports
**Current State**: 65 wildcard imports found. Most are acceptable (`use super::*` in tests, `pub use` re-exports), but `use glow::*` appears 3 times in renderer code  
**Impact**: Low — Wildcard imports from external crates pollute the namespace  
**Effort**: Low

**Recommendation**: Replace `use glow::*` with explicit imports of used symbols.

---

### 9.6 Add `unsafe` Block Documentation
**Current State**: ~65 `unsafe` blocks across 11 files, mostly in OpenGL rendering  
**Impact**: Low-Medium — Undocumented unsafe is a maintenance risk  
**Effort**: Low

**Concentrated in**:
```
gcodekit5-ui/src/ui/gtk/visualizer.rs:    12 unsafe blocks
gcodekit5-ui/src/ui/gtk/shaders.rs:       10 unsafe blocks
gcodekit5-ui/src/ui/gtk/renderer_3d.rs:    6 unsafe blocks
gcodekit5-ui/src/ui/gtk/stock_texture.rs:  4 unsafe blocks
```

**Recommendation**: Add `// SAFETY: <reason>` comments above every `unsafe` block per Rust convention.

---

## 10. Tooling & Workflow (LOW PRIORITY)

### 10.1 Pre-Commit Hooks
**Current State**: Likely no hooks configured  
**Impact**: Low - Catches issues before commit  
**Effort**: Low

**Create** `.git/hooks/pre-commit`:
```bash
#!/bin/bash
cargo fmt --check || exit 1
cargo clippy --all -- -D warnings || exit 1
cargo test --lib || exit 1
```

**Or use** `pre-commit` framework: `pre-commit install`

---

### 10.2 Add Continuous Benchmarking
**Current State**: No performance tracking  
**Impact**: Low - Catches regressions  
**Effort**: Low-Medium

**Setup**:
- Benchmark on each commit
- Track results over time
- Alert if performance degrades >5%
- Tools: `cargo-criterion`, `cargo-nextest`

---

### 10.3 Create Development Container
**Current State**: Setup documented but manual  
**Impact**: Low - Improves onboarding  
**Effort**: Low

**Add** `.devcontainer/Dockerfile` and `devcontainer.json`:
```json
{
  "image": "mcr.microsoft.com/devcontainers/rust:latest",
  "customizations": {
    "vscode": {
      "extensions": ["rust-lang.rust-analyzer", "tomoki1207.pdf"]
    }
  }
}
```

Benefits: One-click development setup, consistent environment

---

## Priority Matrix & Roadmap

### Immediate (Next 1-2 Releases)
| Issue | Effort | Impact | Priority |
|-------|--------|--------|----------|
| 2.1 - Fix Clippy error in designer | Low | High | **✅ DONE** |
| 1.1 - Reduce unwraps/expects | Medium | High | **P0** |
| 9.1 - Add rustfmt.toml | Low | Medium | **P1** |
| 9.2 - Add clippy.toml | Low | Medium | **P1** |
| 2.1 - Fix remaining Clippy warnings | Low | Low | **✅ DONE** |
| 2.3 - Remove debug code | Low | Low | **✅ DONE** |
| 4.1 - Testing strategy | Medium | High | **P0** |
| 9.6 - Document unsafe blocks | Low | Medium | **P1** |

### Short-term
| Issue | Effort | Impact | Priority |
|-------|--------|--------|----------|
| 2.2 - Split large files (20→13 files >1000 LOC) | Medium | High | **✅ DONE** |
| 1.2 - Add error types to 4 remaining crates | Medium | Medium | **P1** |
| 9.3 - Decouple communication→visualizer | Medium | Medium | **P1** |
| 9.4 - Audit #[allow()] suppressions | Medium | Medium | **P1** |
| 3.1 - Complex types | Medium | Medium | **P1** |
| 6.2 - Separate logic | Medium | High | **P1** |
| 3.2 - API docs (20-30% missing) | Medium | Medium | **P2** |

### Medium-term
| Issue | Effort | Impact | Priority |
|-------|--------|--------|----------|
| 6.1 - Event bus | High | High | **P1** |
| 4.3 - Mutation testing | Low | Medium | **P2** |
| 2.4 - Complete TODOs | Variable | Medium | ✅ COMPLETED |
| 7.2 - Dependency updates | Low | Low | **P2** |

### Long-term
| Issue | Effort | Impact | Priority |
|-------|--------|--------|----------|
| 6.3 - Plugin system | High | Low | **P3** |
| 8.1 - ADRs | Low | Low | **P2** |
| 8.3 - Developer guides | Low | Low | **P2** |

---

## Implementation Tracking

### Suggested GitHub Labels
```
- `debt:unwraps` - Unwrap/expect cleanup
- `debt:complexity` - Reduce cognitive complexity
- `debt:testing` - Improve test coverage
- `debt:docs` - Documentation improvements
- `refactor:architecture` - Major architecture changes
- `type:enhancement` - New features
- `type:bug` - Bug fixes
- `effort:small` - <2 hours
- `effort:medium` - 2-8 hours
- `effort:large` - >8 hours
- `blocked:xxx` - Blocked by issue XXX
```

### Quick Checklist for New PRs
- [ ] No new `unwrap()` calls (unless justified with comment)
- [ ] `cargo fmt` passed
- [ ] `cargo clippy` has no new warnings
- [ ] Tests added for new functionality
- [ ] Public APIs documented with `///`
- [ ] Error cases handled (no silent failures)
- [ ] No debug `eprintln!()` or `println!()`
- [ ] Changelog entry added

---

## Conclusion

GCodeKit5 has a solid foundation with modular architecture and good separation of concerns. The recommendations in this document focus on:

1. **Short-term stability**: Fix unwraps, complete TODOs, reduce complexity
2. **Mid-term quality**: Improve testing, error handling, documentation
3. **Long-term flexibility**: Decouple components, enable extensibility

Implementing these changes will make the codebase more robust, maintainable, and welcoming to new contributors.

**Recommended Next Steps**:
1. Create GitHub issues for all P0 items
2. Assign to Q1 2026 milestone
3. Add to pull request checklist immediately
4. Setup pre-commit hooks this week
5. Schedule quarterly review of this document

---

## Appendix A: Code Metrics (Feb 2026)

```
Total Lines of Code:     130,212
Crates:                  9
Source Files:            430
Test Functions:          ~791
Largest File:            3,907 lines (designer_canvas.rs)
Files >1000 LOC:         20+
Average File Size:       303 lines
Clippy Warnings:         137+ (1 hard error in designer)
Clippy Auto-fixable:     ~20
#[allow(..)] Attrs:      ~156 across 21 files
unsafe blocks:           ~65 across 11 files
Wildcard Imports:        65 (mostly acceptable)
todo!/unimplemented!:    0 ✅
println!/eprintln!:      17 files (mostly build scripts/tests)
#[allow(dead_code)]:     19 instances
Test to Code Ratio:      ~1:165 (tests/KLOC)
```

### Error Handling Maturity by Crate
```
✅ thiserror + error.rs:  core, communication, designer, visualizer
❌ anyhow only:           camtools, devicedb, gcodeeditor, settings, ui
```

### CI Pipeline Status
```
✅ code-quality.yml:  Clippy unwrap checking + rustfmt validation
✅ msrv.yml:          MSRV verification (Rust 1.88)
✅ release.yml:       Multi-platform builds (Linux, macOS, Windows)
❌ No benchmark CI
❌ No coverage CI
```

## Appendix B: Files for Quick Review (Feb 2026)

**Start Here** (Largest files, most impactful to split):
1. `crates/gcodekit5-ui/src/ui/gtk/designer_canvas.rs` (3,907 lines)
2. `crates/gcodekit5-ui/src/ui/gtk/visualizer.rs` (3,836 lines)
3. `crates/gcodekit5-ui/src/ui/gtk/machine_control.rs` (2,720 lines)
4. `crates/gcodekit5-designer/src/model.rs` (2,524 lines)

**Error Handling Review**:
1. `crates/gcodekit5-core/src/error.rs` (main error types)
2. `crates/gcodekit5-communication/src/firmware/grbl/controller.rs` (unwrap calls)
3. `crates/gcodekit5-designer/src/slice_toolpath.rs` (unwrap calls)

**Testing Priority** (weakest coverage):
1. `crates/gcodekit5-gcodeeditor/` (5 tests)
2. `crates/gcodekit5-settings/` (2 test files)
3. `crates/gcodekit5-devicedb/` (1 integration test)
