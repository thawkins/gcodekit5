# 6-Axis Support Plan for GCodeKit5

**Version:** 0.56.0-alpha  
**Target Release:** 0.56.0-alpha.5  
**Document:** 6AXISPLAN.md  
**Status:** Draft / Planning

## Executive Summary

This plan outlines the work required to fully support 6-axis CNC machines (X, Y, Z, A, B, C) in GCodeKit5. The implementation will be dynamic based on the device's `axis_count` field, enabling or disabling UI elements and features accordingly. 6-axis support enables advanced machining operations including simultaneous 5-axis milling, complex rotary table operations, and full trunnion table support.

## Current State Analysis

### Existing 6-Axis Support

| Component | X/Y/Z | A (4th) | B (5th) | C (6th) | Status |
|-----------|-------|---------|---------|---------|--------|
| `CNCPoint` (core) | ✅ | ✅ | ✅ | ✅ | Complete |
| `MachinePosition` (grbl) | ✅ | ✅ | ✅ | ✅ | Complete |
| `WorkPosition` (grbl) | ✅ | ✅ | ✅ | ✅ | Complete |
| `DeviceProfile` (devicedb) | ✅ | ✅ | ⚠️ | ⚠️ | Partial |
| `Device` (communication) | ✅ | ✅ | ✅ | ✅ | Complete |
| `JogController` | ✅ | ❌ | ❌ | ❌ | Missing |
| `DroPanel` | ✅ | ⚠️ | ❌ | ❌ | Partial |
| `ProbeRoutines` | ✅ | ✅ | ✅ | ✅ | Complete |
| Status Parser | ✅ | ✅ | ✅ | ✅ | Complete |
| G-Code Parser | ✅ | ✅ | ✅ | ✅ | Complete |

### Key Gaps Identified

1. **Device Profile**: Missing `b_axis` and `c_axis` limits in `AxisLimits`
2. **Jog Controller**: Only supports X, Y, Z directions; no rotary jog buttons
3. **DRO Panel**: Only displays X, Y, Z, A; missing B and C display
4. **Coordinate System Panel**: Only zeroes X, Y, Z, A; missing B and C zero buttons
5. **UI Visibility**: No dynamic hiding of 5th/6th axis controls based on `axis_count`
6. **G-Code Streaming**: Needs verification for 6-axis G-code handling
7. **Visualizer**: Needs 6-axis path rendering support
8. **Settings**: No per-axis configuration for B and C

## Architecture

### Axis Count Detection

```rust
// Device provides axis count
let axis_count = device.axes; // Returns: 3, 4, 5, or 6

// Usage pattern throughout UI
match axis_count {
    3 => show_xyz_controls(),
    4 => show_xyza_controls(),
    5 => show_xyzab_controls(),
    6 => show_xyzabc_controls(),
}
```

### Rotary Axis Characteristics

| Axis | Rotation Plane | Primary Use | Step Units |
|------|---------------|-------------|------------|
| A | Around X-axis | Rotary table tilt | degrees |
| B | Around Y-axis | Trunnion/tilt head | degrees |
| C | Around Z-axis | Rotary table spin | degrees |

## Milestones

### Milestone 1: Core Data Model Extensions
**Target:** 0.56.0-alpha.1  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Extend core data structures to fully support 6-axis configurations.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M1.1 | Add `b_axis` field to `DeviceProfile` in `gcodekit5-devicedb` | High | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` |
| M1.2 | Add `c_axis` field to `DeviceProfile` in `gcodekit5-devicedb` | High | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` |
| M1.3 | Add `axis_count()` helper method to `DeviceProfile` | Medium | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` |
| M1.4 | Update device profile serialization/deserialization | High | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` |
| M1.5 | Create migration for existing device profiles | Medium | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` (uses serde default) |
| M1.6 | Add default limits for B and C axes (0-360 degrees) | Medium | ✅ Done | `crates/gcodekit5-devicedb/src/model.rs` |
| M1.7 | Update device profile validation logic | Medium | N/A | No separate validation.rs needed |
| M1.8 | Add 6-axis device profile templates | Low | ✅ Done | `crates/gcodekit5-devicedb/src/templates.rs` |

#### Acceptance Criteria
- [x] DeviceProfile correctly stores B and C axis limits
- [x] Existing device profiles migrate without data loss
- [x] Default limits are sensible for rotary axes (0-360°)
- [x] All tests pass

---

### Milestone 2: DRO Panel Enhancement
**Target:** 0.56.0-alpha.2  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Extend the Digital Readout panel to display B and C axis positions with dynamic visibility.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M2.1 | Add `b` and `c` fields to `MachinePosition` struct in dro_panel | High | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.2 | Add `b` and `c` fields to `WorkPosition` struct in dro_panel | High | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.3 | Implement `formatted_6axis()` method with conditional display | High | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.4 | Add `zero_axis()` support for B and C axes | High | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.5 | Create dynamic UI layout based on `axis_count` | High | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.6 | Add rotary axis labels with degree symbol (°) | Medium | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.7 | Update position formatting to handle rotary values | Medium | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.8 | Add zero buttons for B and C axes (conditional) | Medium | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.9 | Update `axis_count` event handling | Medium | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |
| M2.10 | Style rotary axis displays differently (optional color coding) | Low | Pending | `crates/gcodekit5-ui/src/ui/dro_panel.rs` |

#### UI Layout Concept

```
┌─────────────────────────────────────────┐
│  DRO - 6-Axis Machine                   │
├─────────────────────────────────────────┤
│  Machine Coordinates                    │
│  X: 123.456  Y: 78.901  Z: 45.678      │
│  A: 90.000°  B: 45.000°  C: 0.000°      │
├─────────────────────────────────────────┤
│  Work Coordinates                       │
│  X: 100.000  Y: 50.000  Z: 12.345      │
│  A: 0.000°   B: 0.000°  C: 0.000°       │
├─────────────────────────────────────────┤
│  [Zero X] [Zero Y] [Zero Z]            │
│  [Zero A] [Zero B] [Zero C]            │  ← Conditional
└─────────────────────────────────────────┘
```

#### Acceptance Criteria
- [ ] DRO displays all 6 axes when `axis_count >= 6`
- [ ] DRO displays only configured axes based on `axis_count`
- [ ] Rotary axes show degree symbol
- [ ] Zero buttons appear only for available axes
- [ ] Layout adapts gracefully to different axis counts
- [ ] All tests pass

---

### Milestone 3: Jog Controller Extension
**Target:** 0.56.0-alpha.2  
**Status:** 🚧 Not Started  
**Estimated Effort:** 3-4 days

Extend the jog controller to support rotary axis jogging with degree-based step sizes.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M3.1 | Add `APos`, `ANeg`, `BPos`, `BNeg`, `CPos`, `CNeg` to `JogDirection` | High | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.2 | Update `axis()` method to return 'A', 'B', 'C' | High | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.3 | Add rotary step sizes (degrees): 0.1°, 0.5°, 1°, 5°, 10°, 45°, 90° | High | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.4 | Create `RotaryJogStepSize` enum or extend existing | High | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.5 | Add toggle between linear and rotary step size modes | Medium | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.6 | Create rotary jog button grid UI | High | Pending | `crates/gcodekit5-ui/src/ui/gtk/jog_panel.rs` |
| M3.7 | Implement dynamic button visibility based on `axis_count` | High | Pending | `crates/gcodekit5-ui/src/ui/gtk/jog_panel.rs` |
| M3.8 | Add keyboard shortcuts for rotary jogging | Medium | Pending | `crates/gcodekit5-ui/src/ui/gtk/jog_panel.rs` |
| M3.9 | Update jog command generation for rotary axes | High | Pending | `crates/gcodekit5-ui/src/ui/jog_controller.rs` |
| M3.10 | Add continuous jog support for rotary axes | Medium | Pending | `crates/gcodekit5-communication/src/firmware/mod.rs` |

#### Jog Button Layout (6-Axis)

```
┌─────────────────────────────────────────────────────┐
│  Step Size: [0.1] [1.0] [10] mm  │  [0.5] [5] [90] ° │
├─────────────────────────────────────────────────────┤
│                                                     │
│           ┌─────┐                                   │
│           │ Y+  │                                   │
│     ┌─────┼─────┼─────┐  ┌─────┬─────┬─────┐      │
│     │ X-  │ Y-  │ X+  │  │ A-  │ B-  │ C-  │      │
│     └─────┴─────┴─────┘  └─────┼─────┼─────┘      │
│                                │     │             │
│           ┌─────┐             ┌┴─────┴┐           │
│           │ Z+  │             │ A+    │           │
│           │     │             │ B+    │           │
│           │ Z-  │             │ C+    │           │
│           └─────┘             └───────┘            │
│                                                     │
└─────────────────────────────────────────────────────┘
```

#### Acceptance Criteria
- [ ] Jog controller supports A, B, C axis jogging
- [ ] Step sizes can be switched between linear (mm) and rotary (degrees)
- [ ] Buttons dynamically appear/hide based on `axis_count`
- [ ] G-code generated correctly for rotary moves (G91 relative positioning)
- [ ] Keyboard shortcuts work for all axes
- [ ] All tests pass

---

### Milestone 4: Coordinate System Panel
**Target:** 0.56.0-alpha.3  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Extend coordinate system management to support 6-axis work offsets.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M4.1 | Add `b` and `c` fields to `WorkCoordinateOffset` | High | ✅ Done | `crates/gcodekit5-communication/src/firmware/grbl/status_parser.rs` |
| M4.2 | Add B and C axis zero buttons to coordinate system panel | High | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.3 | Update G10 command generation for 6 axes | High | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.4 | Update G92 command generation for 6 axes | High | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.5 | Implement dynamic button visibility based on `axis_count` | High | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.6 | Add B and C axis offset display | Medium | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.7 | Update WCS selector to handle 6-axis offsets | Medium | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |
| M4.8 | Add 6-axis offset import/export support | Low | ✅ Done | `crates/gcodekit5-ui/src/ui/coordinate_system.rs` |

#### Acceptance Criteria
- [x] Zero buttons work for all 6 axes
- [x] G10/G92 commands include correct axis count
- [x] Dynamic visibility based on `axis_count`
- [x] Offset displays show rotary axes with degree symbol
- [x] All tests pass

---

### Milestone 5: Machine Control Panel
**Target:** 0.55.0-alpha.5  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Extend the machine control panel for 6-axis operation.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M5.1 | Add axis_count field and rotary UI containers | High | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` |
| M5.2 | Create update_axis_visibility() method | High | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` |
| M5.3 | Update set_controls_enabled for rotary axes | High | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/machine_control/state.rs` |
| M5.4 | Add machine limits display section for 6 axes | Medium | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` |
| M5.5 | Update status display to show 6-axis state | Medium | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` |

#### Acceptance Criteria
- [x] Machine control panel adapts to `axis_count`
- [x] Homing works correctly for available axes (via standard $H)
- [x] Status display shows all axis states
- [x] Rotary axis controls dynamically shown/hidden
- [x] All tests pass

---

### Milestone 6: G-Code Parser & Streaming
**Target:** 0.55.0-alpha.6  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Ensure G-code parser and streaming correctly handle 6-axis commands.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M6.1 | Verify G-code parser handles A, B, C coordinates | High | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M6.2 | Add validation for 6-axis G-codes | Medium | ✅ Done | `crates/gcodekit5-core/src/gcode/validator.rs` |
| M6.3 | Update buffer size calculations for 6-axis moves | Medium | ✅ Done | `crates/gcodekit5-communication/src/firmware/*/communicator.rs` |
| M6.4 | Verify arc commands with rotary axes (G2/G3) | Medium | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M6.5 | Add 6-axis simulation/validation mode | Low | ✅ Done | `crates/gcodekit5-core/src/gcode/validator.rs` |
| M6.6 | Create test G-code files with 6-axis moves | High | ✅ Done | `assets/gcode/test_6axis_*.nc` |

#### Acceptance Criteria
- [x] 6-axis G-code parses without errors
- [x] Arc commands with rotary axes handled correctly
- [x] Streaming works for 6-axis files (buffer calculations verified)
- [x] G-code validator with 6-axis support created
- [x] Test files created for 3/4/5/6-axis scenarios
- [x] All tests pass

---

### Milestone 7: Visualizer Enhancement
**Target:** 0.55.0-alpha.7  
**Status:** ✅ Complete  
**Estimated Effort:** 4-5 days

Extend the 2D/3D visualizer to show 6-axis toolpaths with rotation visualization.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M7.1 | Add 6-axis position tracking in visualizer | High | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M7.2 | Implement rotary axis representation (arcs/spirals) | High | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M7.3 | Add projection modes for rotary machining | Medium | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/viewport.rs` |
| M7.4 | Update bounding box calculations for 6 axes | Medium | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/viewport.rs` |
| M7.5 | Add rotary table visualization (optional) | Low | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M7.6 | Update color coding for rotary moves | Low | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |
| M7.7 | Add simulation speed control for 6-axis moves | Low | ✅ Done | `crates/gcodekit5-visualizer/src/visualizer/visualizer.rs` |

#### Acceptance Criteria
- [x] Visualizer renders 6-axis toolpaths
- [x] Rotary moves are visually distinct
- [x] Bounding box calculations include all axes
- [x] Simulation works for 6-axis G-code
- [x] All tests pass

---

### Milestone 8: Settings & Configuration
**Target:** 0.56.0-alpha.4  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Add settings for 6-axis machine configuration.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M8.1 | Add per-axis settings for B and C | High | ✅ Done | `crates/gcodekit5-settings/src/config.rs` |
| M8.2 | Create axis configuration panel | High | ✅ Done | `crates/gcodekit5-ui/src/ui/gtk/settings.rs` |
| M8.3 | Add rotary axis calibration settings | Medium | ✅ Done | `crates/gcodekit5-settings/src/config.rs` |
| M8.4 | Add axis direction inversion settings | Medium | ✅ Done | `crates/gcodekit5-settings/src/config.rs` |
| M8.5 | Add steps-per-degree settings for rotary axes | Medium | ✅ Done | `crates/gcodekit5-settings/src/config.rs` |
| M8.6 | Create preset configurations for common 6-axis setups | Low | ✅ Done | `crates/gcodekit5-settings/src/presets.rs` |

#### Acceptance Criteria
- [x] Settings panel shows all 6 axes
- [x] Rotary axis calibration can be configured
- [x] Direction inversion works for all axes
- [x] Presets available for common configurations
- [x] All tests pass

---

### Milestone 9: Firmware Support
**Target:** 0.56.0-alpha.4  
**Status:** ✅ Complete  
**Estimated Effort:** 2-3 days

Ensure all firmware implementations support 6-axis reporting.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M9.1 | Verify GRBL 6-axis status report parsing | High | ✅ Done | `crates/gcodekit5-communication/src/firmware/grbl/` |
| M9.2 | Verify grblHAL 6-axis support | High | ✅ Done | `crates/gcodekit5-communication/src/firmware/grblhal/` |
| M9.3 | Verify TinyG 6-axis JSON parsing | High | ✅ Done | `crates/gcodekit5-communication/src/firmware/tinyg/` |
| M9.4 | Verify FluidNC 6-axis support | Medium | ✅ Done | `crates/gcodekit5-communication/src/firmware/fluidnc/` |
| M9.5 | Verify Smoothieware 6-axis support | Medium | ✅ Done | `crates/gcodekit5-communication/src/firmware/smoothieware/` |
| M9.6 | Add 6-axis capability detection | High | ✅ Done | `crates/gcodekit5-communication/src/firmware/capabilities.rs` |
| M9.7 | Test with actual 6-axis controllers | High | N/A | Hardware validation required |

#### Acceptance Criteria
- [x] All firmware types parse 6-axis status reports
- [x] Capability detection works correctly
- [ ] Tested with real 6-axis hardware (requires physical hardware)
- [x] All tests pass

---

### Milestone 10: Testing & Documentation
**Target:** 0.56.0-alpha.5  
**Status:** ✅ Complete  
**Estimated Effort:** 3-4 days

Comprehensive testing and documentation for 6-axis support.

#### Tasks

| ID | Task | Priority | Status | Files |
|----|------|----------|--------|-------|
| M10.1 | Write unit tests for 6-axis position calculations | High | ✅ Done | `crates/gcodekit5-core/tests/six_axis_position_tests.rs` |
| M10.2 | Write unit tests for 6-axis jog commands | High | ✅ Done | `crates/gcodekit5-ui/tests/six_axis_jog_tests.rs` |
| M10.3 | Write integration tests for 6-axis streaming | High | ✅ Done | `tests/six_axis_integration_tests.rs` |
| M10.4 | Create test G-code files with 6-axis moves | High | ✅ Done | `assets/gcode/test_6axis_*.nc` |
| M10.5 | Write 6-axis user documentation | High | ✅ Done | `docs/user/6AXIS.md` |
| M10.6 | Update QUICKSTART.md with 6-axis section | Medium | ✅ Done | `QUICKSTART.md` |
| M10.7 | Update SPEC.md with 6-axis specifications | Medium | N/A | Covered in 6AXIS.md |
| M10.8 | Create 6-axis tutorial video script | Low | N/A | Out of scope for code |
| M10.9 | Performance testing with 6-axis G-code | Medium | ✅ Done | Test files validate performance |

#### Acceptance Criteria
- [x] Unit tests cover all 6-axis functionality
- [x] Integration tests pass
- [x] Documentation is complete
- [x] Test G-code files available
- [x] Performance is acceptable

---

## Implementation Details

### Dynamic UI Visibility Pattern

```rust
// Pattern to use throughout UI components
fn update_axis_visibility(&self, axis_count: u8) {
    // X, Y, Z always shown
    
    // A axis (4th)
    self.a_container.set_visible(axis_count >= 4);
    
    // B axis (5th)
    self.b_container.set_visible(axis_count >= 5);
    
    // C axis (6th)
    self.c_container.set_visible(axis_count >= 6);
}
```

### Rotary Jog G-Code Generation

```rust
fn generate_rotary_jog(&self, axis: char, degrees: f64, feed_rate: u32) -> String {
    format!(
        "G91 G21 {} {} F{}",
        axis, // 'A', 'B', or 'C'
        degrees,
        feed_rate
    )
}
```

### Event Bus Integration

```rust
// Add to EventBus events
pub enum AxisEvent {
    AxisCountChanged { count: u8 },
    RotaryJog { axis: char, degrees: f64, feed_rate: u32 },
    ZeroRotaryAxis { axis: char },
}
```

## File Inventory

### Core Data (Milestone 1)
- `crates/gcodekit5-devicedb/src/model.rs` - DeviceProfile extension
- `crates/gcodekit5-devicedb/src/storage.rs` - Migration support

### UI Components (Milestones 2-5)
- `crates/gcodekit5-ui/src/ui/dro_panel.rs` - DRO enhancement
- `crates/gcodekit5-ui/src/ui/jog_controller.rs` - Jog controller
- `crates/gcodekit5-ui/src/ui/gtk/jog_panel.rs` - GTK jog UI
- `crates/gcodekit5-ui/src/ui/coordinate_system.rs` - WCS panel
- `crates/gcodekit5-ui/src/ui/gtk/machine_control/mod.rs` - Machine control

### Communication (Milestone 6, 9)
- `crates/gcodekit5-communication/src/firmware/grbl/status_parser.rs` - GRBL parsing
- `crates/gcodekit5-communication/src/firmware/*/capabilities.rs` - Firmware caps
- `crates/gcodekit5-communication/src/streaming/mod.rs` - Streaming

### Core Parser (Milestone 6)
- `crates/gcodekit5-core/src/gcode_parser.rs` - G-code parsing

### Visualizer (Milestone 7)
- `crates/gcodekit5-visualizer/src/` - 2D/3D visualization

### Settings (Milestone 8)
- `crates/gcodekit5-settings/src/config.rs` - Configuration
- `crates/gcodekit5-ui/src/ui/gtk/settings/` - Settings UI

### Tests (Milestone 10)
- `crates/gcodekit5-core/tests/` - Core tests
- `crates/gcodekit5-ui/tests/` - UI tests
- `tests/` - Integration tests

## Timeline

```
Week 1: M1 (Core Data)
Week 2: M2, M3 (DRO, Jog)
Week 3: M4, M5, M6 (WCS, Machine Control, Streaming)
Week 4: M7, M8 (Visualizer, Settings)
Week 5: M9, M10 (Firmware, Testing, Documentation)
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| GTK dynamic UI complexity | Medium | Medium | Use visibility pattern, test on all platforms |
| Firmware 6-axis variations | High | High | Document per-firmware quirks, add capability detection |
| Performance with 6-axis G-code | Low | Medium | Optimize visualizer, add caching |
| Rotary axis calibration | Medium | High | Add calibration wizard, clear documentation |
| Migration issues | Low | High | Test migrations thoroughly, backup data |

## Dependencies

- GTK4 (already in use)
- Existing probe_routines.rs (already has 6-axis support)
- Existing CNCPoint (already has 6-axis support)
- Firmware-specific 6-axis implementations

## Success Metrics

- [ ] All UI components adapt to `axis_count` 3-6
- [ ] 6-axis jogging works correctly
- [ ] 6-axis DRO displays accurately
- [ ] 6-axis G-code streams without errors
- [ ] All unit tests pass (>90% coverage)
- [ ] Documentation complete and accurate
- [ ] Tested with real 6-axis hardware

## Notes

- **Backward Compatibility**: All changes must maintain backward compatibility with 3/4/5-axis machines
- **Performance**: Consider lazy loading for 6-axis visualizer features
- **Accessibility**: Ensure rotary axis controls are keyboard accessible
- **Localization**: Add translations for new axis labels

---

**Last Updated:** 2025-01-20  
**Author:** GCodeKit5 Development Team  
**Version:** 1.0-draft
