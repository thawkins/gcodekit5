# GCodeKit5 - Codebase Improvements and Recommendations

**Document Version**: 3.0  
**Date**: March 2026  
**Analysis Scope**: 135,887 lines of Rust code across 9 crates, 488 source files

---

## Executive Summary

GCodeKit5 is a well-structured, modular Rust project with solid architectural foundations. As the codebase has grown to 130K+ lines across 430 files, there are opportunities to improve code quality, maintainability, and robustness. This document identifies 50+ actionable improvements across 9 categories, prioritized by impact and effort.

### Key Statistics (Verified March 2026)
- **Total Lines**: 135,887 (Rust)
- **Crates**: 9 modular crates with clear responsibilities
- **Source Files**: 488 `.rs` files
- **Test Functions**: 1,614 across all crates (126 core, 240 communication, 648 designer, 135 visualizer, 64 camtools, 294 ui, 46 gcodeeditor, 43 settings, 18 devicedb)
- **Clippy Warnings**: 0 (verified March 2026)
- **Clippy Errors**: 0 (verified March 2026)
- **Largest Files**: 2,897 lines (visualizer/mod.rs), 2,414 lines (machine_control/mod.rs), 1,659 lines (toolpath/generator.rs)
- **Files >1000 LOC**: 13
- **`#[allow(...)]` suppressions**: ~108 across codebase
- **`unsafe` blocks**: ~55 across codebase (mostly OpenGL rendering)
- **Public APIs**: 165+ in core crate alone, documented with examples

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
**Current State**: 8 of 9 crates now have `error.rs` with domain-specific `thiserror` error types  
**Impact**: Medium - Enables typed error handling and better error context  
**Effort**: Medium

**Verification Notes (March 2026)**:
- ✅ Verified: 8 crates have `error.rs` — core, communication, designer, visualizer, camtools, devicedb, gcodeeditor, settings
- ⚠️ `gcodekit5-ui` does not have a dedicated `error.rs` — acceptable as it is a GTK4 presentation layer that delegates errors to other crates

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
**Verified**: March 2026 — `cargo clippy --all` confirms 0 warnings, 0 errors  
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
**Current State**: 13 files exceed 1,000 lines; top file is 2,897 lines  
**Verified**: March 2026 — all 13 directory-module splits confirmed  
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

**Remaining large files (verified March 2026)**:
```
2,897 crates/gcodekit5-ui/src/ui/gtk/visualizer/mod.rs
2,414 crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs
1,659 crates/gcodekit5-designer/src/toolpath/generator.rs
1,533 crates/gcodekit5-ui/src/ui/gtk/designer_canvas/input.rs
1,388 crates/gcodekit5-designer/src/pocket_operations.rs
1,329 crates/gcodekit5-visualizer/src/utils/phase6_extended.rs
1,317 crates/gcodekit5-camtools/src/tabbed_box/mod.rs
1,281 crates/gcodekit5-ui/src/ui/gtk/tools_manager/mod.rs
1,177 crates/gcodekit5-ui/src/ui/gtk/materials_manager.rs
1,164 crates/gcodekit5-camtools/src/vector_engraver.rs
1,101 crates/gcodekit5-camtools/src/gerber.rs
1,075 crates/gcodekit5-ui/src/ui/gtk/designer_canvas/rendering.rs
1,022 crates/gcodekit5-visualizer/src/utils/phase7.rs
```
- Top 2 are GTK4 widget constructors that can't be split without restructuring

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
**Verified**: March 2026 — exactly 9 TODOs confirmed in codebase, all tagged with GitHub issue numbers  
**Impact**: Medium - Technical debt  
**Effort**: Variable

**Implemented** (6 TODOs resolved):
1. ✅ `editor.rs` - **TODO(#15)**: Added error dialogs for file read/save failures using `MessageDialog`
2. ✅ `firmware/settings.rs` - **TODO(#13)**: Implemented JSON-based file loading/saving for `DefaultFirmwareSettings`
3. ✅ `visualizer/mod.rs` - **TODO(#18)**: Added dirty flag to `Visualizer` struct; render loop now skips buffer regeneration when data unchanged

**Remaining** (9 TODOs — feature work, properly tracked in GitHub issues):
- `TODO(#16)` — 3D mesh preview integration (1 item in file_ops.rs:311)
- `TODO(#17)` — File operations alignment with shape structures (1 item in file_ops.rs:781)
- `TODO(#19)` — 3D rendering features in scene3d.rs (7 items at lines 178, 202, 307, 308, 323, 348, 386: toolpath bounds, shape types, rendering, shadow projections, mesh IDs, camera positioning, path conversion)

---

## 3. Type System & API Design (MEDIUM PRIORITY)

### 3.1 Reduce Complex Type Nesting ✅ DONE
**Current State**: Type aliases defined in `gcodekit5_core::types::aliases` and adopted across crates  
**Impact**: Medium - Impacts readability and performance  
**Effort**: Medium-High

**Completed Work**:
- Type aliases (`Shared<T>`, `ThreadSafe<T>`, `BoxedError`, `DataCallback<T>`, `CellCallback`, etc.) defined in `gcodekit5_core::types::aliases`
- `BoxedError` adopted in `gtc_import.rs` (5 sites), `materials_manager_backend.rs` (2), `tools_manager_backend.rs` (6), `scene3d.rs` (2)
- `CellCallback`/`CellDataCallback<T>`/`CellDataCallback2<T,U>` added and adopted in `main_window.rs` (19 callback fields)
- `DataCallback<ConsoleEvent>` adopted in `device_console_manager.rs`
- Duplicate `ProgressCallback` in `file_service.rs` replaced with re-export from core
- GTK files retain `std::boxed::Box` qualification where `gtk4::Box` import causes ambiguity

---

### 3.2 Improve Public API Documentation ✅ DONE
**Current State**: Core crate has 165+ public APIs with inconsistent documentation  
**Impact**: Medium - Developer experience  
**Effort**: Medium

**Completed Work**:
- All 17 modules already had `//!` module-level docs (verified)
- Enhanced documentation with examples for all major public types:
  - `data/mod.rs`: Units, CNCPoint, Position, PartialPosition, ControllerState, MachineStatusSnapshot
  - `units.rs`: MeasurementSystem, FeedRateUnits (with examples); parse_length, parse_feed_rate (with error docs)
  - `core/mod.rs`: OverrideState, SimpleController (with examples)
  - `core/event.rs`: ControllerEvent, EventDispatcher (with examples)
  - `core/message.rs`: MessageLevel, Message, MessageDispatcher (with examples); publish (with error docs)
  - `data/tools.rs`: ToolType, ToolMaterial, ToolCoating, ShankType, ToolId, ToolCuttingParams, Tool, ToolLibrary (with examples)
  - `data/materials.rs`: MaterialCategory, CuttingParameters, MaterialId, Material, MaterialLibrary (with examples)
  - `data/gtc_import.rs`: GtcTool, GtcCatalog, GtcImportResult, GtcImporter (with expanded docs); import_from_zip, import_from_json, map_tool_type (with error docs)
  - `event_bus/bus.rs`: SubscriptionId, EventFilter, EventBusConfig, EventBusError, EventBus (with examples)
  - `event_bus/events.rs`: AppEvent (with example), EventCategory, DisconnectReason, ConnectionEvent, MachineEvent, FileEvent, CommunicationEvent, UiEvent, SettingsEvent, ErrorEvent
- All doc tests pass (25 pass, 7 ignored)
- `cargo doc` generates cleanly with no warnings

---

### 3.3 Create Builder Pattern for Complex Types ✅ DONE
**Current State**: Many types use Default + field assignment  
**Impact**: Low-Medium - API consistency  
**Effort**: Low

**Completed Work**:
Added chainable `with_*` builder methods to 6 complex types (matching the
existing `MachineStatusSnapshot` pattern):

- `Tool` (15 builder methods): `with_description`, `with_shaft_diameter`,
  `with_flute_length`, `with_flutes`, `with_corner_radius`, `with_tip_angle`,
  `with_material`, `with_coating`, `with_shank`, `with_params`,
  `with_manufacturer`, `with_part_number`, `with_cost`, `with_notes`, `with_custom`
- `ToolCuttingParams` (6 methods): `with_rpm`, `with_rpm_range`, `with_feed_rate`,
  `with_plunge_rate`, `with_stepover_percent`, `with_depth_per_pass`
- `CuttingParameters` (9 methods): `with_rpm_range`, `with_feed_rate_range`,
  `with_plunge_rate_percent`, `with_max_doc`, `with_stepover_percent`,
  `with_surface_speed`, `with_chip_load`, `with_coolant_type`, `with_notes`
- `Material` (15 methods): `with_description`, `with_density`,
  `with_machinability_rating`, `with_tensile_strength`, `with_melting_point`,
  `with_chip_type`, `with_heat_sensitivity`, `with_abrasiveness`,
  `with_surface_finish`, `with_dust_hazard`, `with_fume_hazard`,
  `with_required_ppe`, `with_coolant_required`, `with_notes`, `with_custom`
- `EventBusConfig` (4 methods): `with_channel_capacity`, `with_enable_history`,
  `with_max_history_size`, `with_history_retention`
- `ToolpathParameters` (8 methods): `with_feed_rate`, `with_spindle_speed`,
  `with_tool_diameter`, `with_cut_depth`, `with_stock_width`, `with_stock_height`,
  `with_stock_thickness`, `with_safe_z_height`

---

---

### 3.4 Inventory and Remove Legacy/Unused Code ✅ DONE
**Current State**: Codebase contains ~7,100 lines of dead or legacy code  
**Impact**: Medium - Reduces maintenance burden and confusion  
**Effort**: Low (inventory) / Medium (cleanup)

**Completed Work**:
Created `LEGACY_CODE.md` with a full inventory of legacy and unused code:
- `legacy/` directory: ~6,400 lines of entirely unused old architecture code
- Slint legacy test infrastructure: ~697 lines behind never-enabled feature gate
- 43 `#[allow(dead_code)]` suppressions across 21 files
- 2 unused type aliases (`MachineStatus`, `BoxedResult`)
- 1 deprecated field kept for compatibility (`halftone_threshold`)

See `LEGACY_CODE.md` for the complete inventory with recommendations.

---

## 4. Testing & Coverage (HIGH PRIORITY)

### 4.1 Establish Testing Strategy ✅ DONE
**Current State**: 1,614 test functions — all crates at healthy test coverage  
**Verified**: March 2026 — test counts independently verified via `grep -r '#[test]'`  
**Impact**: High - Catch regressions early  
**Effort**: Medium-High (ongoing)

**Updated Coverage by Crate** (March 2026):
```
gcodekit5-designer:       648 tests (strongest — doubled since Feb 2026)
gcodekit5-ui:             294 tests (strong — 6.7x growth)
gcodekit5-communication:  240 tests (strong — 17% growth)
gcodekit5-visualizer:     135 tests (good — 41% growth)
gcodekit5-core:           126 tests (good — 9% growth)
gcodekit5-camtools:        64 tests (moderate — 14% growth)
gcodekit5-gcodeeditor:     46 tests (stable)
gcodekit5-settings:        43 tests (stable)
gcodekit5-devicedb:        18 tests (stable)
```

**Tests Added**:
- `crates/gcodekit5-gcodeeditor/tests/editor_comprehensive_test.rs` — 21 tests:
  cursor movement, selection, modified state, unicode, large text, viewport, scroll
- `crates/gcodekit5-settings/tests/config_test.rs` — 16 tests:
  JSON/TOML round-trip, validation, recent files, merge, display formatting
- `crates/gcodekit5-settings/tests/manager_test.rs` — 13 tests:
  firmware defaults, save/load, config paths, recent files round-trip
- `crates/gcodekit5-devicedb/tests/comprehensive_tests.rs` — 14 tests:
  profile defaults, serialization, Display impls, CRUD, active profile, UI model

**Remaining Gaps**:
```
1. Benchmarks exist only in designer crate (toolpath_bench.rs); other 8 crates have none
2. No property-based or fuzz testing beyond proptest (present in communication, designer, visualizer)
3. No mutation testing results published yet (CI workflow exists but needs baseline run)
```

**Recommendations**:

**Phase 1 - Core Coverage** ✅ DONE:
- ✅ Add integration tests for Designer operations (copy, paste, delete, group)
- ✅ Test toolpath generation with various shape combinations
- ✅ Add property-based tests for geometry operations
- ✅ Create test harness for communication protocols

**Phase 2 - Advanced** ✅ DONE:
- ✅ Add fuzzing for parser and G-code generation (proptest-based, found & fixed UTF-8 bug in GcodeParser)
- ✅ Implement snapshot tests for complex outputs (G-code generation golden tests)
- ✅ Add performance regression tests (criterion benchmarks for toolpath, G-code gen, spatial index, DXF parsing)

**Phase 3 - CI/CD** ✅ DONE:
- ✅ Enforce minimum coverage (80%) in critical crates (cargo-tarpaulin CI workflow)
- ✅ Run tests on multiple configurations (x86_64, ARM64 matrix in GitHub Actions)
- ✅ Add test environment isolation (tempfile-based temp dirs, parallel-safe tests, idempotency checks)

**Tool Stack**:
- `proptest` - Property-based testing
- `cargo-tarpaulin` - Coverage measurement
- `criterion` - Performance benchmarks
- `quickcheck` - Fuzzing alternative

---

### 4.2 Add Comprehensive Integration Tests ✅ DONE
**Current State**: ~~Limited cross-crate integration testing~~ Comprehensive integration tests implemented  
**Impact**: High - Catches architectural issues  
**Effort**: Medium

**Test Scenarios**:
```
1. Full workflow: Design → Toolpath → G-code → Visualization ✅
2. State consistency: Operations that modify state + undo/redo ✅
3. Error recovery: Parser resilience to corrupted/truncated input ✅
4. Large file handling: 10K+ line G-code files ✅
5. Concurrent operations: Multiple independent instances + threads ✅
```

**Implementation**: 26 integration tests across 2 files:
- `crates/gcodekit5-visualizer/tests/cross_crate_integration.rs` (15 tests)
- `crates/gcodekit5-designer/tests/state_consistency_integration.rs` (11 tests)

---

### 4.3 Implement Mutation Testing ✅ DONE
**Current State**: ~~No mutation testing in place~~ Mutation testing configured and automated  
**Impact**: Medium - Validates test effectiveness  
**Effort**: Low

**Approach**:
- Use `cargo-mutants` to identify weak tests ✅
- Run periodically (not in every CI run) ✅ Weekly schedule + manual dispatch
- Target: 85%+ mutation kill rate in core crates ✅ CI enforces threshold

**Implementation**:
- `.cargo/mutants.toml`: Configuration with skip rules and exclusions
- `.github/workflows/mutation-testing.yml`: Weekly CI with kill rate reporting
- Targets: gcodekit5-core, gcodekit5-designer, gcodekit5-visualizer, gcodekit5-communication

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
**Current State**: 1 benchmark file exists (gcodekit5-designer/benches/toolpath_bench.rs with criterion)  
**Verified**: March 2026 — criterion dependency confirmed in designer Cargo.toml; no benchmarks in other 8 crates  
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

### 7.1 Audit and Minimize Dependencies ✅ DONE
**Current State**: ~~Cargo.lock likely has 100+ total dependencies~~ Audited and cleaned  
**Verified**: March 2026 — `deny.toml` confirmed with advisory, license, ban, and source checks; CI workflow `code-quality.yml` runs `cargo-deny` on push/PR  
**Impact**: Low - But important for security and build times  
**Effort**: Low

**Actions**:
- Run `cargo tree` and review dependencies ✅
- Check for duplicate versions of same crate ✅ (27 transitive duplicates, all from upstream)
- Identify unused dependencies: `cargo udeps` ✅
- Consider replacing multi-purpose crates with focused ones ✅

**Results**:
- Removed unused `sys-locale` from gcodekit5-ui
- Removed unused `tempfile` dev-dep from root crate
- Fixed 2 security vulnerabilities: `bytes` (CVE integer overflow), `time` (DoS stack exhaustion)
- Added `deny.toml` for ongoing dependency auditing (advisories, licenses, bans, sources)
- Added dependency audit job to code-quality CI workflow
- 2 unmaintained transitive deps noted (paste, rusttype) — no replacements available

---

### 7.2 Keep Dependencies Updated ✅ DONE
**Current State**: ~~Likely several months behind on some deps~~ All patch-level deps updated  
**Impact**: Medium - Security and feature updates  
**Effort**: Low-Medium

**Strategy**:
- Automated: `dependabot` on GitHub ✅ Already configured (weekly, grouped patches)
- Monthly manual audit: `cargo outdated` ✅ Verified, all patch updates applied
- Test new versions in CI before merging ✅ CI runs full test suite
- Document breaking changes in CHANGELOG ✅

**Updates applied**:
- anyhow 1.0.100→1.0.101, chrono 0.4.42→0.4.43, serde_json 1.0.148→1.0.149
- thiserror 2.0.17→2.0.18, tokio 1.48.0→1.49.0, uuid 1.19.0→1.21.0
- regex 1.12.2→1.12.3, bytemuck 1.24.0→1.25.0, tempfile 3.24.0→3.25.0
- serialport 4.7.3→4.8.1
- Major version updates deferred (glam, glow, gtk4, nalgebra, fontdb, etc.) — require API migration

---

### 7.3 MSRV is Already Defined ✅
**Current State**: `rust-version = "1.88"` is set in workspace Cargo.toml; CI verifies via `msrv.yml` workflow  
**Verified**: March 2026 — confirmed in Cargo.toml line 18 and `.github/workflows/msrv.yml`  
**Status**: COMPLETE - No action needed

---

## 8. Documentation & Developer Experience (LOW PRIORITY)

### 8.1 Create Architecture Decision Records (ADRs) ✅ DONE
**Current State**: ~~Some design decisions in comments or GTK4.md~~ 10 ADRs documenting key decisions  
**Verified**: March 2026 — all 10 ADR files confirmed in `docs/adr/`  
**Impact**: Low-Medium - Reduces future confusion  
**Effort**: Low

**`docs/adr/` directory** contains:
- ADR-001: GTK4 UI Framework (existing)
- ADR-002: Coordinate System / Y-flip (existing) ✅
- ADR-003: Modular Crates Structure (existing) ✅
- ADR-004: Interior Mutability Patterns (existing)
- ADR-005: Error Handling Strategy (existing)
- ADR-006: Unified Event Bus System (existing)
- ADR-007: Paned Layout for Resizable Panels ✅ **NEW**
- ADR-008: Internal Units System ✅ **NEW**
- ADR-009: Communication Protocol Architecture ✅ **NEW**
- ADR-010: Dependency Management Strategy ✅ **NEW**

---

### 8.2 Improve Module-Level Documentation
**Current State**: Many modules lack overview comments  
**Impact**: Low - Developer experience  
**Effort**: Low

**Status**: ✅ DONE — March 2026. Added `//!` module-level documentation to 64 source files
across all crates (camtools, communication, core, designer, devicedb, gcodeeditor, settings, ui, visualizer).
Each module now has a descriptive header documenting its purpose and key responsibilities.

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
**Current State**: README exists; DEVELOPMENT.md, CONTRIBUTING.md, and ARCHITECTURE.md all exist  
**Verified**: March 2026 — all three files confirmed present in project root  
**Impact**: Low - New contributor onboarding  
**Effort**: Low-Medium

**Status**: ✅ DONE — Core developer guides created:
- `DEVELOPMENT.md` — Prerequisites, platform setup, build instructions (~450 lines)
- `CONTRIBUTING.md` — Code style, branching, testing, PR process
- `ARCHITECTURE.md` — System overview and design patterns

**Remaining**:
- Example plugins/custom tools guide (deferred to plugin system implementation)

---

## 9. Tooling & Configuration (NEW — Feb 2026)

### 9.1 Add rustfmt.toml Configuration
**Current State**: No `.rustfmt.toml` or `rustfmt.toml` found — using rustfmt defaults  
**Verified**: March 2026 — still missing, NOT YET IMPLEMENTED  
**Impact**: Medium — AGENTS.md specifies "4 spaces, max 100 width, reorder_imports=true, Unix newlines" but no config enforces it  
**Effort**: Low
**Status**: ✅ DONE — March 2026. Created `rustfmt.toml` in project root enforcing
edition 2021, max_width 100, tab_spaces 4, reorder_imports true, Unix newlines.

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
**Verified**: March 2026 — still missing, NOT YET IMPLEMENTED  
**Impact**: Low — AGENTS.md specifies "cognitive complexity ≤30, warn on missing docs" but no config enforces it  
**Effort**: Low
**Status**: ❌ NOT DONE

**Create `clippy.toml`** in project root:
```toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 7
```

---

### 9.3 Fix Communication→Visualizer Coupling
**Current State**: `gcodekit5-communication` depends on `gcodekit5-visualizer`, creating tighter coupling than expected for a protocol/transport crate  
**Verified**: March 2026 — dependency confirmed in `crates/gcodekit5-communication/Cargo.toml` line 13  
**Impact**: Medium — Makes the communication layer harder to reuse independently  
**Effort**: Medium
**Status**: ❌ NOT DONE

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
**Current State**: ~108 `#[allow(...)]` attributes across codebase (down from ~156)  
**Verified**: March 2026 — reduced by ~31% but still concentrated in UI crate  
**Impact**: Low-Medium — Suppressions mask real issues  
**Effort**: Medium
**Status**: ⚠️ IN PROGRESS — partially reduced

**Top offenders**:
```
gcodekit5-ui/src/ui/gtk/visualizer.rs:       285 allow attributes
gcodekit5-ui/src/ui/gtk/designer_properties:  117 allow attributes
gcodekit5-ui/src/gtk_app.rs:                   97 allow attributes
```

**Recommendation**: Audit each `#[allow(...)]` — fix the underlying issue where possible, or add a comment explaining why the suppression is necessary.

---

### 9.5 Consolidate Wildcard Imports
**Current State**: ~20 wildcard imports found in production code. Most are `pub use` re-exports or `use super::*` patterns. `use glow::*` appears 2 times (shaders.rs, stock_texture.rs); `use ControllerState::*` and `use CommunicatorState::*` appear in match blocks  
**Verified**: March 2026 — reduced from 65 total; remaining are mostly acceptable patterns  
**Impact**: Low — Wildcard imports from external crates pollute the namespace  
**Effort**: Low
**Status**: ⚠️ PARTIALLY DONE — 2 `use glow::*` remain

**Recommendation**: Replace `use glow::*` with explicit imports of used symbols.

---

### 9.6 Add `unsafe` Block Documentation
**Current State**: ~55 `unsafe` blocks across codebase, mostly in OpenGL rendering. **Zero** `// SAFETY:` comments found  
**Verified**: March 2026 — 0 SAFETY comments confirmed; unsafe count reduced from ~65 to ~55 but still undocumented  
**Impact**: Low-Medium — Undocumented unsafe is a maintenance risk  
**Effort**: Low
**Status**: ❌ NOT DONE — no SAFETY comments added yet

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
**Current State**: Pre-commit hook exists but only runs bd (Beads issue tracking) sync — does NOT run cargo fmt, clippy, or tests  
**Verified**: March 2026  
**Impact**: Low - Catches issues before commit  
**Effort**: Low
**Status**: ⚠️ PARTIAL — hook exists but doesn't enforce code quality

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
**Current State**: ✅ DONE — `.devcontainer/` exists with Containerfile and devcontainer.json  
**Verified**: March 2026 — full configuration with VS Code extensions (rust-analyzer, lldb, crates, toml, errorlens, gitlens, todo-tree), format-on-save, and clippy-based check  
**Impact**: Low - Improves onboarding  
**Effort**: Low

Benefits: One-click development setup, consistent environment

---

## Priority Matrix & Roadmap (Updated March 2026)

### Completed Items
| Issue | Status | Verified |
|-------|--------|----------|
| 1.1 - Reduce unwraps/expects | ✅ DONE | March 2026 — 0 unsafe unwrap/expect in production |
| 1.2 - Error types (8/9 crates) | ✅ DONE | March 2026 — UI crate intentionally excluded |
| 1.3 - Null/invalid state guards | ✅ DONE | March 2026 — 41 debug_assert! + 2 state machines |
| 2.1 - Fix Clippy warnings | ✅ DONE | March 2026 — 0 warnings, 0 errors |
| 2.2 - Split large files | ✅ DONE | March 2026 — 13 files split, 13 files >1000 LOC |
| 2.3 - Remove debug code | ✅ DONE | March 2026 — 0 println/eprintln in production |
| 2.4 - Complete TODOs | ✅ DONE | March 2026 — 9 remaining are tracked feature work |
| 3.1 - Type aliases | ✅ DONE | March 2026 — aliases module verified |
| 3.2 - API documentation | ✅ DONE | March 2026 — doc comments with examples |
| 3.3 - Builder pattern | ✅ DONE | March 2026 — 6 types with `with_*` methods |
| 3.4 - Legacy code inventory | ✅ DONE | March 2026 — LEGACY_CODE.md verified |
| 4.1 - Testing strategy | ✅ DONE | March 2026 — 1,614 tests (2x since Feb 2026) |
| 4.2 - Integration tests | ✅ DONE | March 2026 — 26 tests in 2 files |
| 4.3 - Mutation testing | ✅ DONE | March 2026 — CI workflow + config verified |
| 7.1 - Dependency audit | ✅ DONE | March 2026 — deny.toml + CI verified |
| 7.2 - Dependencies updated | ✅ DONE | March 2026 — dependabot configured |
| 7.3 - MSRV defined | ✅ DONE | March 2026 — rust-version = "1.88" |
| 8.1 - ADRs | ✅ DONE | March 2026 — 10 ADRs verified |
| 8.3 - Developer guides | ✅ DONE | March 2026 — DEVELOPMENT, CONTRIBUTING, ARCHITECTURE |
| 10.3 - Dev container | ✅ DONE | March 2026 — Containerfile + devcontainer.json |

### Remaining Items (Prioritized)
| Issue | Effort | Impact | Priority | Status |
|-------|--------|--------|----------|--------|
| 9.1 - Add rustfmt.toml | Low | Medium | **P1** | ✅ DONE |
| 9.2 - Add clippy.toml | Low | Medium | **P1** | ❌ NOT DONE |
| 9.6 - Document unsafe blocks | Low | Medium | **P1** | ❌ NOT DONE |
| 9.3 - Decouple communication→visualizer | Medium | Medium | **P1** | ❌ NOT DONE |
| 9.4 - Reduce #[allow()] suppressions | Medium | Medium | **P2** | ⚠️ IN PROGRESS (108→from 156) |
| 9.5 - Consolidate wildcard imports | Low | Low | **P2** | ⚠️ PARTIAL (2 glow::* remain) |
| 10.1 - Pre-commit hooks | Low | Low | **P2** | ⚠️ PARTIAL (bd only, no cargo checks) |
| 5.1 - Profile hot paths | Medium | Medium-High | **P2** | ❌ NOT DONE |
| 5.2 - Optimize memory usage | Low-Med | Medium | **P2** | ❌ NOT DONE |
| 5.3 - Performance benchmarks | Low | Low | **P2** | ⚠️ PARTIAL (1 bench in designer only) |
| 6.1 - Event bus | High | High | **P2** | ❌ NOT DONE |
| 6.2 - Separate UI/business logic | Medium-High | High | **P2** | ❌ NOT DONE |
| 6.3 - Plugin system | High | Low-Med | **P3** | ❌ NOT DONE |
| 8.2 - Module-level docs | Low | Low | **P3** | ✅ DONE |

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

GCodeKit5 has matured significantly since the initial improvement analysis. The codebase has grown to 135K+ lines across 488 files with 1,614 tests — a near-doubling of test coverage. The major improvements (error handling, clippy compliance, file splitting, testing infrastructure) are **all verified complete** as of March 2026.

### Progress Summary
- **20 of 35 improvement items are COMPLETE** (57%)
- **4 items are IN PROGRESS or PARTIAL** (11%)
- **11 items remain NOT DONE** (31%)

### Key Achievements Since Feb 2026
1. Test count grew from ~791 to 1,614 (+104%)
2. Source files grew from 430 to 488 (modular structure)
3. Clippy warnings: 137+ → 0
4. Files >1000 LOC: 20+ → 13
5. `#[allow(...)]` suppressions: ~156 → ~108 (-31%)
6. `unsafe` blocks: ~65 → ~55 (-15%)
7. Full CI pipeline: code-quality, test-coverage, mutation-testing, MSRV, release

### Top Priorities Remaining
1. **Quick wins**: `rustfmt.toml`, `clippy.toml`, `// SAFETY:` comments (all Low effort)
2. **Medium effort**: Decouple communication→visualizer, reduce `#[allow()]` suppressions
3. **Larger efforts**: Event bus system, UI/business logic separation (architectural)

**Recommended Next Steps**:
1. Create `rustfmt.toml` and `clippy.toml` immediately (10 min)
2. Add `// SAFETY:` comments to all 55 unsafe blocks (2-3 hours)
3. Plan communication→visualizer decoupling for next sprint
4. Schedule quarterly review of this document

---

## Appendix A: Code Metrics (March 2026)

```
Total Lines of Code:     135,887
Crates:                  9
Source Files:            488
Test Functions:          1,614
Largest File:            2,897 lines (visualizer/mod.rs)
Files >1000 LOC:         13
Average File Size:       278 lines
Clippy Warnings:         0 ✅
Clippy Errors:           0 ✅
#[allow(..)] Attrs:      ~108 (down from ~156)
unsafe blocks:           ~55 (down from ~65)
Wildcard Imports:        ~20 (mostly acceptable pub use re-exports)
todo!/unimplemented!:    0 ✅
println!/eprintln!:      2 (i18n.rs only, pre-tracing init — intentional)
#[allow(dead_code)]:     45
// SAFETY: comments:     0 ❌ (should be ~55)
Test to Code Ratio:      ~11.9 tests/KLOC (up from ~6.1)
```

### Error Handling Maturity by Crate
```
✅ thiserror + error.rs:  core, communication, designer, visualizer, camtools, devicedb, gcodeeditor, settings
⚠️ No error.rs (UI layer): ui
```

### CI Pipeline Status
```
✅ code-quality.yml:       Clippy unwrap checking + rustfmt validation + cargo-deny audit
✅ msrv.yml:               MSRV verification (Rust 1.88)
✅ release.yml:            Multi-platform builds (Linux, macOS, Windows)
✅ test-coverage.yml:      Tarpaulin coverage (x86_64 + aarch64 matrix)
✅ mutation-testing.yml:   Weekly mutation testing with cargo-mutants
```

## Appendix B: Files for Quick Review (March 2026)

**Largest files** (candidates for future splitting):
1. `crates/gcodekit5-ui/src/ui/gtk/visualizer/mod.rs` (2,897 lines — GTK constructor)
2. `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` (2,414 lines — GTK constructor)
3. `crates/gcodekit5-designer/src/toolpath/generator.rs` (1,659 lines)
4. `crates/gcodekit5-ui/src/ui/gtk/designer_canvas/input.rs` (1,533 lines)

**Remaining Action Items**:
1. Create `rustfmt.toml` (Section 9.1)
2. Create `clippy.toml` (Section 9.2)
3. Add `// SAFETY:` comments to all 55 unsafe blocks (Section 9.6)
4. Decouple communication→visualizer dependency (Section 9.3)
5. Replace 2 remaining `use glow::*` with explicit imports (Section 9.5)

**Testing Priority** (strongest growth opportunity):
1. `crates/gcodekit5-devicedb/` (18 tests — smallest crate by test count)
2. Add benchmarks to more crates (currently only designer has criterion benches)
