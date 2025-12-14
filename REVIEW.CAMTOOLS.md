# REVIEW.CAMTOOLS.md

Scope: CAM Tools UI dialogs (`crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs`) and the associated G-code generators in `crates/gcodekit5-camtools/src/*`.

Constraints:
- Do **not** introduce any new `SpinButton` controls; prefer the existing unit-aware entry/controls pattern used by the Designer “Position” and “Size” sections.

---

## CAMTOOLS-001 — Extract a shared “progress + cancel + background work” helper
**Why:** `BitmapEngravingTool`, `VectorEngravingTool`, `TabbedBoxMaker`, and `JigsawTool` repeat near-identical progress-window code and cancellation plumbing.

**Prompt (actionable):**
- In `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs`, create a small helper (e.g. `run_with_progress_dialog(...)`) that:
  - Builds the modal progress window (title + status label + progress bar + Cancel).
  - Spawns the worker thread.
  - Provides a cancellation flag (`Arc<AtomicBool>`).
  - Polls both progress and result on the main thread.
- Replace the duplicated progress-dialog blocks (e.g. around ~1133, ~1868, ~2663, and the jigsaw generator progress window) with calls to this helper.

---

## CAMTOOLS-002 — Replace simulated progress with real progress reporting for generators
**Why:** `TabbedBoxMaker` and `JigsawTool` simulate progress with a timer, which can mislead users and doesn’t reflect long operations.

**Prompt (actionable):**
- Add optional progress callbacks to the relevant generator APIs in `crates/gcodekit5-camtools` (or at least at the UI level wrap internal loops) so the UI can show real stage/progress.
- Targets:
  - `crates/gcodekit5-camtools/src/tabbed_box.rs` generation pipeline.
  - `crates/gcodekit5-camtools/src/jigsaw_puzzle.rs` generation pipeline.
- Update `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs` to use these progress callbacks instead of time-based simulation.

---

## CAMTOOLS-003 — Make cancellation actually stop generation
**Why:** Current “Cancel” often just stops updates; worker thread may continue (e.g. progress callback returns early but generation continues), wasting CPU and confusing users.

**Prompt (actionable):**
- In each generator loop in `gcodekit5-camtools` (scanline loops, hatch generation loops, puzzle/box generation loops), periodically check an `AtomicBool`/cancellation closure and early-return with a well-typed cancellation error.
- In the UI (`cam_tools.rs`), set cancellation flag on Cancel and treat cancellation as a non-error user action (close dialog without showing an error popup).

---

## CAMTOOLS-004 — Stop injecting/removing `$H` via string replacement
**Why:** `gcode.replace("$H\n", "").replace("$H", "")` is brittle (can delete legitimate `$H` in comments, macros, etc.) and mixes UI concerns into post-processing.

**Prompt (actionable):**
- Introduce a structured “header options” concept for generators (include_homing, include_wcs_setup, etc.) rather than generating `$H` always.
- Update:
  - `crates/gcodekit5-camtools/src/laser_engraver.rs` (currently emits `$H` in generator).
  - `crates/gcodekit5-camtools/src/vector_engraver.rs`.
  - `crates/gcodekit5-camtools/src/spoilboard_grid.rs`.
  - `crates/gcodekit5-camtools/src/spoilboard_surfacing.rs`.
  - `crates/gcodekit5-camtools/src/jigsaw_puzzle.rs`.
  - `crates/gcodekit5-camtools/src/tabbed_box.rs`.
- Then remove the UI-level string replace blocks in `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs`.

---

## CAMTOOLS-005 — Separate “device-specific init” from “toolpath” in generated G-code
**Why:** Many generators emit GRBL-specific commands (`$H`, `$32=1`, `G10 L20 ...`) that may be inappropriate depending on device/firmware and user workflows.

**Prompt (actionable):**
- In `gcodekit5-camtools`, split output into:
  1) Preamble/init (units, plane, homing, laser mode, WCS setup)
  

---

## CAMTOOLS-006 — Standardize numeric types to internal units conventions (mm as `f32`)
**Why:** Core guidance says dimensions are internally mm as `f32`, but spoilboard generators use `f64`.

**Prompt (actionable):**
- Convert `SpoilboardGridParameters` and `SpoilboardSurfacingParameters` fields from `f64` to `f32` (and propagate to the UI collectors).
- Files:
  - `crates/gcodekit5-camtools/src/spoilboard_grid.rs`
  - `crates/gcodekit5-camtools/src/spoilboard_surfacing.rs`

---

## CAMTOOLS-007 — Validate parameters and surface validation errors inline (not via silent defaults)
**Why:** Many `Entry` parses use `parse().unwrap_or(default)` which silently changes user inputs.

**Prompt (actionable):**
- In `cam_tools.rs` collectors (e.g. `collect_params(...)` functions), replace silent fallback with:
  - Validation step that returns `Result<Params, ValidationError>`.
  - Inline error presentation (Adw toast/banner or error row) for invalid fields.
- Key hotspots:
  - `TabbedBoxMaker::collect_params` (width/height/thickness/burn/finger-joint multiples)
  - `JigsawTool::collect_params`
  - `BitmapEngravingTool::collect_params`
  - `VectorEngravingTool::collect_params`
  - Spoilboard tools’ params collection.

---

## CAMTOOLS-008 — Guard against invalid values that can cause infinite loops or unsafe moves
**Why:** Some generators assume non-zero step sizes.

**Prompt (actionable):**
- In `crates/gcodekit5-camtools/src/spoilboard_surfacing.rs`, ensure `step_dist > 0` (e.g. stepover_percent > 0 and tool_diameter > 0), otherwise return an error.
- In `crates/gcodekit5-camtools/src/spoilboard_grid.rs`, ensure `grid_spacing > 0` to avoid infinite `while` loops.
- In `crates/gcodekit5-camtools/src/speeds_feeds.rs`, guard `tool.diameter > 0` to prevent divide-by-zero.

---

## CAMTOOLS-009 — Use device profile limits for speeds/feeds (remove hardcoded max RPM)
**Why:** `SpeedsFeedsCalculator` clamps RPM to a hardcoded `24000` which is wrong for 12k spindles and other machines.

**Prompt (actionable):**
- In `crates/gcodekit5-camtools/src/speeds_feeds.rs`, replace `let max_rpm = 24000.0` with device-derived spindle max RPM (new schema field you added under device capabilities).
- Add warning text referencing the device profile field used for clamping.

---

## CAMTOOLS-010 — Standardize “safe Z” behaviour across tools
**Why:** Some generators hardcode safe height and/or don’t ensure Z retract before XY moves.

**Prompt (actionable):**
- Ensure every generator:
  - Retracts to safe Z before any rapid XY move.
  - Uses a consistent safe Z parameter (either per-tool or pulled from device profile).
- Files to normalize:
  - `laser_engraver.rs` (uses `G0 Z5.0` hardcoded)
  - `vector_engraver.rs` (uses `G0 Z5.0` hardcoded)
  - `spoilboard_surfacing.rs` (uses param `safe_z` but plunge feed derived from feed rate)
  - `spoilboard_grid.rs` (no Z axis handling at all)

---

## CAMTOOLS-011 — Improve preamble/postamble safety defaults
**Why:** `spoilboard_grid.rs` always enables laser mode `$32=1` and ends with `G0 X0 Y0` without Z retract (and assumes GRBL).

**Prompt (actionable):**
- Add options for:
  - Laser mode enable (`$32=1`) only when firmware is GRBL+laser.
  - Park position (none / work origin / machine origin / custom).
  - Optional “Z retract before park”.
- Implement in `crates/gcodekit5-camtools/src/spoilboard_grid.rs` and surface these toggles in `cam_tools.rs`.

---

## CAMTOOLS-012 — Parameter persistence: use typed structs and schema versioning
**Why:** Some load paths parse into `serde_json::Value`, losing validation and forward-compat support.

**Prompt (actionable):**
- For each tool’s “Save/Load Parameters”, define a `#[derive(Serialize, Deserialize)]` struct with a `version` field and migrate on load.
- Update save/load implementations in `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs`:
  - Bitmap tool load/save (currently uses `serde_json::Value`)
  - Vector tool load/save (currently uses `serde_json::Value`)
  - Ensure all tools show an error dialog/toast when load fails instead of silently doing nothing.

---

## CAMTOOLS-013 — Consolidate file chooser configuration (filters, default sizes, titles)
**Why:** Repeated `FileChooserDialog` configuration; also `set_default_size(900, 700)` is unusually large.

**Prompt (actionable):**
- Create a helper to build file dialogs (Open/Save) with:
  - Reasonable default size (or none, letting GTK choose)
  - Consistent filters and titles
  - Remember last directory per tool
- Update all occurrences in `crates/gcodekit5-ui/src/ui/gtk/cam_tools.rs` (search `FileChooserDialog::new`).

---

## CAMTOOLS-014 — Use unit-aware controls for dimension/feed inputs (no SpinButtons)
**Why:** CAM tools currently use plain text `Entry` fields with fixed “(mm)” labels and manual parsing.

**Prompt (actionable):**
- Replace CAM Tools dimension/feed entries with the same unit-aware entry/control pattern used by Designer “Position” and “Size” sections.
- Apply to (examples):
  - Jigsaw width/height/kerf/offsets/feed
  - Tabbed box dimensions/thickness/burn/offsets/feed
  - Vector offsets/feed/travel/hatch spacing
  - Bitmap offsets/feed/travel/pixels-per-mm/line spacing
  - Spoilboard width/height/tool diameter/stepover/feed/safe Z
- Keep the UI as text-entry based (unit-aware parsing/formatting), do not introduce SpinButtons.

---

## CAMTOOLS-015 — Reduce “magic defaults” and present presets instead
**Why:** Many defaults are arbitrary and not tied to device/tool/material libraries.

**Prompt (actionable):**
- For each CAM tool, add a “Preset” dropdown that seeds parameters from:
  - Selected Device
  - Selected Tool (from CNC Tools library)
  - Selected Material
- Start with Speeds & Feeds calculator output feeding into other tools.
- Implementation sites:
  - `cam_tools.rs` tool panels
  - `gcodekit5-camtools/src/speeds_feeds.rs` as the calculation source.

---

## CAMTOOLS-016 — Improve dashboard UX (discoverability and navigation)
**Why:** Dashboard is a grid of cards but there’s no search, “recent tools”, or “recent presets”.

**Prompt (actionable):**
- In `CamToolsView::create_dashboard` (`cam_tools.rs`):
  - Add a search entry to filter tool cards.
  - Add a “Recently used” section.
  - Replace placeholder icons with consistent symbolic icons per tool.
  - Ensure layout looks good on FHD @ 125% (avoid cramped 2-column grid if window is narrow).

---

## CAMTOOLS-017 — Add consistent inline “estimated time” + “bounding box” summaries
**Why:** Generators compute useful metrics (e.g. laser engraver time estimates), but UI doesn’t consistently show it before generating.

**Prompt (actionable):**
- Expose and display:
  - Estimated time (minutes)
  - Output extents (W×H)
  - Line count / path count
- Use the existing `estimate_time()` patterns (e.g. in `laser_engraver.rs`, `vector_engraver.rs`) and show a summary label near Generate.

---

## CAMTOOLS-018 — Run generated G-code through Validator/Optimizer before emitting
**Why:** The camtools crate has `validator` and `optimizer`, but CAM Tools UI appears to emit raw strings.

**Prompt (actionable):**
- In the UI generation path (`cam_tools.rs`), after generation:
  1) Validate G-code (`GCodeValidator`)
  2) Optionally optimize (`GCodeOptimizer`)
  3) Show warnings inline before sending output to the editor/viewer.
- Provide toggles in “Advanced” sections.

---

## CAMTOOLS-019 — Improve help content and link it from the UI
**Why:** `resources/markdown/cam_tools.md` is minimal and doesn’t explain safety assumptions (homing/WCS/laser mode) or how to use each tool.

**Prompt (actionable):**
- Expand `crates/gcodekit5-ui/resources/markdown/cam_tools.md` to include per-tool sections:
  - What it generates
  - Required machine setup
  - Parameter explanations (especially offsets/home-before)
  - Safety warnings (e.g. homing behaviour)
- Add a “Help” button on each tool panel linking to the relevant help anchor.

---

## CAMTOOLS-020 — Add/extend tests for generator invariants and cancellation
**Why:** There are tests in `crates/gcodekit5-camtools/tests`, but key safety invariants are not covered.

**Prompt (actionable):**
- Add tests in `crates/gcodekit5-camtools/tests/` to verify:
  - No infinite loops when spacing/stepover invalid (expects error).
  - Output includes/omits init blocks based on options.
  - Cancellation returns a deterministic cancellation error.
  - Generated toolpaths stay within declared bounds.
