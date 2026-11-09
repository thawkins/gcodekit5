# Probe / Touch Probe Implementation Plan

## Document Information
| | |
|---|---|
| **Project** | GCodeKit5 |
| **Status** | Planning |
| **Version** | 0.55.0-alpha target |
| **Scope** | Full touch-probe support: G38.x commands, probing routines, WCS auto-update, tool-length setter, CAM-tool UI dialogs |

---

## 1. Overview

This plan details the implementation of **CNC touch-probe support** in GCodeKit5. Probing enables automated, repeatable work-coordinate setup, tool-length measurement, and in-process inspection without manual edge-finding or paper feeler gauges.

### Goals
1. **G-code layer** – Send standard `G38.x` probe cycles and parse trigger positions.
2. **Communication layer** – Detect probe-trigger status reports from GRBL / grblHAL / FluidNC / TinyG.
3. **Core types** – Model probe types (touch probe, tool-length setter), probe routines, and results.
4. **CAM tools** – Provide easy-to-use UI dialogs for the most common probing operations (edge, corner, bore/boss center, Z-touch).
5. **WCS integration** – Automatically update work-coordinate offsets (`G54`–`G59`, `G92`, `G10`) from probe results.
6. **Safety** – Robust retract / fail-safe behaviour when a probe cycle does not trigger.

### What is *not* in scope (future releases)
- Full 3-D inspection / measurement reporting
- Probed surface-to-mesh comparison
- Automatic tool-wear compensation loops
- Wireless / Renishaw-style macro integration

---

## 2. Reference: How CNC Probes Work

### 2.1 Hardware
| Type | Description | Typical I/O |
|------|-------------|-----------|
| **Touch probe** | Spring-loaded stylus; deflection closes a circuit. | Z-probe pin (normally-open or normally-closed) on controller |
| **Tool-length setter** | Fixed plate on bed; tool touches plate to set Z. | Same probe input or dedicated second pin (`G31.1` on some controllers) |
| **Edge finder / 3D tasters** | Mechanical or electrical devices for edge location. | May be manual (no controller integration) or electrical (probe input) |

### 2.2 Standard G-code Commands
| Command | Behaviour |
|---------|-----------|
| `G38.2 X<x> Y<y> Z<z> F<feed>` | Probe toward target. **Error** if probe does not trigger before target. |
| `G38.3 X<x> Y<y> Z<z> F<feed>` | Probe toward target. **No error** if probe does not trigger (graceful fail). |
| `G38.4 X<x> Y<y> Z<z> F<feed>` | Probe away from target until probe **opens** (release probe). Error if still closed at target. |
| `G38.5 X<x> Y<y> Z<z> F<feed>` | Probe away from target until probe opens. No error if still closed. |

> GRBL / grblHAL / FluidNC all implement `G38.2` and `G38.3`. `G38.4/5` are present in grblHAL and some GRBL 1.1 forks but are less universal; we will start with `G38.2/3` and add 4/5 later.

### 2.3 Probe Result Reporting
- GRBL 1.1 responds with `[PRB:0.000,10.000,0.000:1]` (X, Y, Z, success flag) after a `G38.x` cycle.
- Status poll `?` may contain `Pn:P` (probe pin active) in real-time report.
- TinyG / g2core return JSON probe results.
- FluidNC mirrors GRBL 1.1 behaviour.

### 2.4 Common CAM Probe Routines
| Routine | Description |
|---------|-------------|
| **Z-touch / surface probe** | Rapid to safe height, probe down until trigger, set WCS Z = 0 (or known offset). |
| **Edge find (single axis)** | Probe toward an edge in X or Y, back off, probe again slowly for accuracy, set WCS zero on that edge. |
| **Corner find** | Probe X-min, probe Y-min (or X-max/Y-max), compute corner intersection, set WCS origin. |
| **Bore center** | Probe 4 points on inside diameter (left/right/front/back), compute circle center. |
| **Boss / pin center** | Same 4-point routine but from outside (probe toward centre). |
| **Tool-length setter** | Rapid over setter plate, probe Z down, store tool-length offset (`G43.1` or tool table). |

---

## 3. Architecture Fit

```
┌─────────────────────────────────────────────────────────────┐
│  gcodekit5-ui   │  GTK dialogs for each probe routine       │
│  (cam_tools/)   │  • Probe Routines dashboard              │
│                 │  • Edge / Corner / Bore / Z / Tool     │
├─────────────────────────────────────────────────────────────┤
│  gcodekit5-camtools │  Core probe logic, G-code generation │
│  (probe_routines.rs)│  • Build G38.x command sequences    │
│                     │  • Compute WCS offsets from results  │
├─────────────────────────────────────────────────────────────┤
│  gcodekit5-core     │  Probe types, events, errors         │
│  (events.rs, types) │  • ProbeTriggered event              │
│                     │  • ProbeRoutine, ProbeResult structs   │
├─────────────────────────────────────────────────────────────┤
│  gcodekit5-communication │  Send G38.x, parse [PRB:…]      │
│  (grbl.rs, fluidnc.rs)   │  Real-time status with Pn:P       │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Milestones

### Milestone 1 – Foundation: G38.x Support & Core Types
**Target version:** `0.55.0-alpha.1`  
**Effort:** Medium

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 1.1 | **GCode command model** – Add `G38_2`, `G38_3`, `G38_4`, `G38_5` variants to the gcode parser / command enum in `gcodekit5-core`. | | Parser round-trip tests pass; commands serialize correctly. |
| 1.2 | **GRBL probe response parser** – Parse `[PRB:x,y,z:flag]` into a structured `ProbeResult`. | | Unit tests for `[PRB:…]` with 1/0 flag, missing flag, malformed input. |
| 1.3 | **Real-time probe pin status** – Detect `Pn:P` in GRBL status reports and emit `ProbePinActive` / `ProbePinInactive` events. | | Tests for status strings with and without `P`. |
| 1.4 | **Core probe types** – Define `ProbeType` enum (`TouchProbe`, `ToolLengthSetter`), `ProbeRoutine` descriptor, `ProbeResult` struct with trigger position and success flag. | | Types compile; documented with doc comments. |
| 1.5 | **Error types** – Extend `gcodekit5-core::Error` with `ProbeTimeout`, `ProbeUnexpectedTrigger`, `ProbeStuck`, `ProbeNotSupported`. | | Errors implement `thiserror`; translatable messages. |
| 1.6 | **Controller trait stubs** – Implement real `probe_z`, `probe_x`, `probe_y` async methods on the controller trait (previously unimplemented). | | Methods send correct `G38.x` command and await `ProbeResult`. |

---

### Milestone 2 – Communication Layer Integration
**Target version:** `0.55.0-alpha.2`  
**Effort:** Medium

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 2.1 | **GRBL probe command sender** – In `grbl.rs`, implement `send_probe_command()` that streams `G38.x` with proper feed rate and waits for `[PRB:…]` or alarm. | | Integration test against GRBL 1.1 simulator. |
| 2.2 | **GRBL probe fail-safe** – If `G38.2` does not trigger (alarm or error), issue `G0` retract to safe height; do not leave probe stuck. | | Test: probe miss → automatic retract to `Z+5`. |
| 2.3 | **grblHAL / FluidNC support** – Verify `G38.2/3` compatibility; extend parser if `[PRB:…]` format differs. | | Simulator tests pass for both firmwares. |
| 2.4 | **TinyG / g2core JSON probe results** – Parse JSON probe response fields (`r:{posx,posy,posz,prb:…}`). | | Unit tests with sample TinyG JSON. |
| 2.5 | **Event wiring** – Ensure `ProbeTriggered` event is emitted by all firmware implementations with correct `PartialPosition`. | | Event bus subscriber receives event within 50 ms of trigger. |
| 2.6 | **Async timeout handling** – Probe command should have configurable timeout (default 30 s); cancel on timeout and retract. | | Unit test: timeout fires → `ProbeTimeout` error. |

---

### Milestone 3 – Probe Routines (Core Logic)
**Target version:** `0.55.0-alpha.3`  
**Effort:** Large

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 3.1 | **Routine engine** – Create `crates/gcodekit5-camtools/src/probe_routines.rs` with a `ProbeRoutineEngine` that takes a `ProbeRoutine` descriptor and returns a G-code string + expected result slots. | | Engine generates syntactically valid G-code for all routine types. |
| 3.2 | **Z-touch routine** – Generate: `G0` to XY safe position → `G38.2 Z-<depth> F<feed>` → retract → (optional) slow re-probe for accuracy. | | G-code string matches hand-verified reference. |
| 3.3 | **Edge-find routine (single axis)** – Generate: rapid to start → probe toward edge → backoff `probe_backoff` mm → slow re-probe → compute edge position. | | Reference test against known edge position (simulator). |
| 3.4 | **Corner-find routine** – Generate: probe X-min edge, probe Y-min edge, compute corner intersection, set WCS origin. Support X-max/Y-max variants. | | G-code + math verified with 2D geometry unit tests. |
| 3.5 | **Bore-center routine** – Generate: 4-point inside-diameter probe (left, right, front, back). Compute circle center via intersection of chords. | | Math unit tests: known circle → computed center within 0.001 mm. |
| 3.6 | **Boss / pin-center routine** – Same 4-point pattern but from outside. Adjust probe direction and safety offsets. | | Unit tests for outside probe geometry. |
| 3.7 | **Tool-length setter routine** – Generate: rapid over setter plate → `G38.2 Z-<depth>` → retract. Compute tool length = `trigger_z - setter_plate_z`. | | Reference test with known plate height. |
| 3.8 | **Routine parameters** – Each routine accepts configurable parameters: `probe_feed_fast`, `probe_feed_slow`, `probe_backoff`, `safe_height`, `max_probe_depth`, `setter_plate_z`, `setter_plate_xy`. | | All parameters have sensible defaults and bounds validation. |
| 3.9 | **Result calculator** – After receiving trigger positions, compute final WCS offsets or center coordinates. Return a `ProbeReport` with raw triggers, computed values, and suggested `G10` / `G92` commands. | | Report struct serializes to JSON for save/load. |

---

### Milestone 4 – WCS & Offset Integration
**Target version:** `0.55.0-alpha.4`  
**Effort:** Medium

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 4.1 | **G10 command builder** – Create helper to build `G10 L2 P<n> X<x> Y<y> Z<z>` from `ProbeReport`. | | Command string matches GRBL / LinuxCNC spec. |
| 4.2 | **WCS update service** – Subscribe to `ProbeReport` events and optionally auto-update the active WCS (`G54`–`G59`) or `G92` offset. | | UI toggle "Auto-update WCS" controls behaviour. |
| 4.3 | **Offset preview** – Before applying, show user a preview dialog: *"Probe found Z = -12.345. Set G54 Z = 0?"* with Accept / Cancel. | | Dialog displays computed offset and target WCS. |
| 4.4 | **Tool-length offset (TLO)** – Apply computed tool length via `G43.1 Z<length>` or write to controller tool table if supported. | | TLO is applied immediately after probe; DRO reflects new Z. |
| 4.5 | **Persistent probe results** – Save last probe result per WCS to `settings.json` so they survive restart. | | Load app → previous probe results restored. |

---

### Milestone 5 – UI: Probe CAM Tool Dashboard
**Target version:** `0.55.0-alpha.5`  
**Effort:** Large

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 5.1 | **Dashboard page** – Add "Probe Tools" card to the existing `CamToolsView` dashboard with icon and description. | | Card is visible and navigates to probe sub-stack. |
| 5.2 | **Probe routines stack** – Create `probe_routines.rs` UI module under `crates/gcodekit5-ui/src/ui/gtk/cam_tools/`. Use same `Paned` layout as other CAM tools (40 % sidebar / 60 % settings). | | Layout matches existing CAM tool patterns. |
| 5.3 | **Routine selector** – Left sidebar lists available routines: *Z-Touch, Edge Find, Corner Find, Bore Center, Boss Center, Tool Length*. Selecting one loads the right panel. | | Switching routines preserves previous settings per-routine. |
| 5.4 | **Settings panel** – Per-routine parameter widgets (feed rates, backoff, safe height, max depth, setter plate height). Use `create_dimension_row` from `common.rs` for consistency. | | All numeric entries validated (positive, within machine limits). |
| 5.5 | **Action buttons** – "Probe" (run routine), "Generate G-code" (output to editor), "Load" / "Save" preset parameters to JSON. | | Buttons follow existing CAM tool button patterns. |
| 5.6 | **Result display** – After probing, show a results card with: trigger positions, computed offset / center, suggested G-code snippet, and "Apply to WCS" button. | | Card updates dynamically on probe completion. |
| 5.7 | **Live probe status** – Show probe pin state (idle / triggered) with a coloured indicator during the routine. | | Indicator turns green when `Pn:P` received. |
| 5.8 | **Offline mode** – When disconnected, allow editing parameters and generating G-code; disable "Probe" button. | | Offline indicator text matches other tools. |

---

### Milestone 6 – Jog Panel Quick-Probe Buttons
**Target version:** `0.55.0-alpha.6`  
**Effort:** Small

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 6.1 | **Quick Z-touch button** – Add a small "🔽 Probe Z" button to the jog / DRO panel. One-click: run default Z-touch routine on current XY position. | | Button is disabled when disconnected or already probing. |
| 6.2 | **Quick edge-find buttons** – Small directional arrows around DRO display for X-/X+/Y-/Y+ edge probe. | | Each runs a single-axis edge-find routine in the indicated direction. |
| 6.3 | **Configuration** – Allow users to set default probe feed and backoff in Settings → Probing tab. | | Settings persist to `config.json`. |

---

### Milestone 7 – Documentation & Testing
**Target version:** `0.55.0-alpha.7`  
**Effort:** Medium

| # | Task | Owner | Acceptance Criteria |
|---|------|-------|---------------------|
| 7.1 | **Unit tests** – Cover all probe math (bore center, corner intersection, tool-length calc). Target ≥ 90 % line coverage for `probe_routines.rs`. | | `cargo test` passes; no regressions in existing tests. |
| 7.2 | **Simulator integration tests** – Run full probe cycles against GRBL 1.1 simulator in CI. | | CI job added to `.github/workflows/`. |
| 7.3 | **User documentation** – Add `docs/user/PROBING.md` with setup instructions, safety warnings, and step-by-step for each routine. | | Document reviewed for clarity by fresh user. |
| 7.4 | **In-app tooltips** – Add tooltip help to every probe parameter explaining purpose and safe ranges. | | All tooltips translated to supported languages (de, es, fr, it, pt). |
| 7.5 | **Changelog & spec update** – Update `CHANGELOG.md`, `SPEC.md`, and `RELEASE.md` with probe feature summary. | | Docs reflect all new capabilities. |

---

## 5. Task Dependency Graph

```
Milestone 1 (Foundation)
    │
    ▼
Milestone 2 (Communication)
    │
    ▼
Milestone 3 (Routines) ◄────┐
    │                       │
    ▼                       │
Milestone 4 (WCS)           │
    │                       │
    ▼                       │
Milestone 5 (UI Dashboard) ─┘
    │
    ▼
Milestone 6 (Jog Quick-Probe)
    │
    ▼
Milestone 7 (Docs & Tests)
```

> Milestones 3 and 5 can be developed in parallel once Milestone 2 is complete, because the UI can generate G-code offline while the core routines are being wired to the controller.

---

## 6. File Checklist (New & Modified)

### New files
| Path | Purpose |
|------|---------|
| `crates/gcodekit5-camtools/src/probe_routines.rs` | Core probe routine G-code generation & math |
| `crates/gcodekit5-camtools/src/probe_routines_test.rs` | Unit tests for probe math |
| `crates/gcodekit5-ui/src/ui/gtk/cam_tools/probe_routines.rs` | GTK probe dashboard & dialogs |
| `docs/user/PROBING.md` | End-user probing guide |

### Modified files
| Path | Change |
|------|--------|
| `crates/gcodekit5-core/src/event_bus/events.rs` | Add `ProbePinActive`, `ProbePinInactive` events |
| `crates/gcodekit5-core/src/error.rs` | Add probe-specific error variants |
| `crates/gcodekit5-core/src/types.rs` (or probe module) | Add `ProbeType`, `ProbeRoutine`, `ProbeResult`, `ProbeReport` |
| `crates/gcodekit5-core/src/mod.rs` | Implement real `probe_z`, `probe_x`, `probe_y` on controller trait |
| `crates/gcodekit5-communication/src/grbl.rs` | Send `G38.x`, parse `[PRB:…]`, handle `Pn:P` |
| `crates/gcodekit5-communication/src/grblhal.rs` | Verify `G38.x` compatibility |
| `crates/gcodekit5-communication/src/fluidnc.rs` | Verify `G38.x` compatibility |
| `crates/gcodekit5-communication/src/tinyg.rs` | Parse JSON probe results |
| `crates/gcodekit5-camtools/src/lib.rs` | Export probe module |
| `crates/gcodekit5-ui/src/ui/gtk/cam_tools/mod.rs` | Register probe tool in dashboard |
| `crates/gcodekit5-settings/src/lib.rs` (or config) | Add probe defaults to settings schema |

---

## 7. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Firmwares report `[PRB:…]` in subtly different formats | Medium | High | Parser fuzz tests with samples from all 5 firmwares; graceful fallback to raw string display. |
| Probe stuck / crash if retract fails | Low | Critical | Always retract before any error return; use `G0` with feed-rate limit; hardware e-stop remains ultimate safety. |
| User forgets to set safe height | Medium | High | Default safe height = 5 mm; warn if safe height < 2 mm; visual red border on low values. |
| Math precision in bore-center calc | Low | Medium | Use `f64` internally; unit tests with known circles; reject if chord intersection is degenerate. |
| GRBL 0.9 lacks `G38.3` | Medium | Low | Feature-detect firmware version; disable graceful-fail routines on old GRBL. |

---

## 8. UI Mock (Textual)

```
┌────────────────────────────────────────┬──────────────────────────────────────────┐
│  🔙 Probe Tools                        │  Settings                                │
│                                        │  ┌────────────────────────────────────┐  │
│  ▶ Z-Touch                             │  │ Safe Height        [ 5.00 ] mm     │  │
│  ▶ Edge Find                           │  │ Fast Feed Rate     [ 100  ] mm/min │  │
│  ▶ Corner Find                         │  │ Slow Feed Rate     [ 25   ] mm/min │  │
│  ▶ Bore Center          ◄ selected     │  │ Backoff Distance   [ 2.00 ] mm     │  │
│  ▶ Boss Center                         │  │ Max Probe Depth    [ 20.0 ] mm     │  │
│  ▶ Tool Length                         │  │                                    │  │
│                                        │  │  [Generate G-code]  [ Probe ]     │  │
│  ─────────────────────��                │  └────────────────────────────────────┘  │
│  Status: ● Idle                        │                                          │
│  Probe pin: ○ Open                     │  Results (last probe)                   │
│                                        │  ┌────────────────────────────────────┐  │
│                                        │  │ X trigger:  12.345 mm              │  │
│                                        │  │ Y trigger:  -8.210 mm              │  │
│                                        │  │ Computed center: (12.35, -8.21)    │  │
│                                        │  │                                    │  │
│                                        │  │ [Apply to G54]  [Copy G-code]      │  │
│                                        │  └────────────────────────────────────┘  │
└────────────────────────────────────────┴────────────────────────────────���─────────┘
```

---

## 9. Acceptance Criteria (Overall)

1. A user can connect a touch probe, open **CAM Tools → Probe Tools**, select **Z-Touch**, and automatically set the active WCS Z-zero within ±0.01 mm.
2. A user can probe a rectangular bore and have the app compute and optionally set the bore center as WCS origin.
3. A user can generate probe G-code while offline, save it, and run it later on the machine.
4. All probe parameters are persisted per-routine and editable in Settings.
5. No existing CAM tool functionality is broken (regression test suite passes).
6. Documentation exists for at least English, German, and Spanish.

---

*End of plan.*
