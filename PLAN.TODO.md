# PLAN: Code Markers Remediation

This document lists TODO, FIXME, and XXX markers found in the codebase (excluding `legacy/`) and provides a remediation plan for each entry.

Note: This plan is a guide for implementation work — it does not contain changes or code. Use the prompts to begin implementation tasks.

---

## Step 1 — Implement eStop behavior
- Location: `crates/gcodekit5-ui/src/gtk_app.rs` (status_bar.estop_btn.connect_clicked)
- Description: Emergency Stop button logs only; needs a safe stop logic in the machine controller and UI state updates.
- Proposed remediation:
  - Implement an emergency stop flow in `MachineControlView`/communicator that sends appropriate stop commands (firmware-specific e.g., "!" or other emergency commands).
  - On eStop: stop all motion, halt queued G-code, set UI state to disabled/fault, and show a modal warning informing the user.
  - Add a clear/recover action (or a separate confirm step to recover) to reset the UI once safe.
  - Add tracing logs and telemetry for state updates.
- Prompt:
  - Add `MachineControlView::emergency_stop(&self)` and call it from `estop_btn.connect_clicked`. Ensure `MachineControlView` uses the communicator to send emergency commands and publishes a `is_estopped` state for other UI components.

---

## Step 2 — Visualizer GPU update optimization
- Location: `crates/gcodekit5-ui/src/ui/gtk/visualizer.rs` (rasterization & buffer updates)
- Description: The visualizer updates GPU buffers on every draw; optimize by updating only when geometry or toolpaths change.
- Proposed remediation:
  - Add `dirty` flags to the visualizer state for geometry and toolpath buffer sets.
  - Set `dirty` on events that change geometry (shape updates, toolpaths, device profile) and clear after the CPU->GPU update.
  - Replace unconditional buffer updates with a conditional update when `dirty` is true.
  - Add simple benchmarking for draw performance and tests for frame redraw correctness.
- Prompt:
  - Add buffer dirty flags; set them where shapes/toolpaths are modified; update draw logic to check flags before updating GPU buffers.

---

## Step 3 — Group/Ungroup shape support
- Location: `crates/gcodekit5-ui/src/ui/gtk/designer_layers.rs` (group_selected_shapes, ungroup_selected_shapes)
- Description: Grouping and ungrouping UI placeholders exist; need to implement grouping semantics and UI behavior.
- Proposed remediation:
  - Implement group creation and group ID assignment in `DesignerState`/Canvas shape storage.
  - Add UI flows: group selected shapes, ungroup shapes, toggle group visibility/lock, and resurrect group semantics through selection/toolbar.
  - Persist group info in file formats; add undo/redo support for grouping operations.
  - Add tests for grouping/ungrouping and group selection behavior.
- Prompt:
  - Implement `state.canvas.group_selected()` and `state.canvas.ungroup_selected()` to change `group_id` for shapes and wire UI to call them.

---

## Step 4 — Tools Manager: parse coating combobox
- Location: `crates/gcodekit5-ui/src/ui/gtk/tools_manager.rs` (`let coating = None; // TODO: Parse from combobox`)
- Description: The `coating` property is not read from the combobox control.
- Proposed remediation:
  - Introduce an `Option<ToolCoating>` enum or string for coating values in the tool model.
  - Read combobox selected text or active index and convert to `ToolCoating` or `String`.
  - Update `save_tool` logic to set `coating` and add unit tests.
- Prompt:
  - Replace `let coating = None;` with code that maps `self.edit_coating.active_text()` to a `coating` value and update tests for tool saves.

---

## Step 5 — Materials Manager: implement Save and Delete
- Location: `crates/gcodekit5-ui/src/ui/gtk/materials_manager.rs` (save_material, delete_material)
- Description: Placeholders for saving/deleting materials exist but no persistence flows are implemented.
- Proposed remediation:
  - Implement `save_material` to validate the form, construct a `Material` data structure, persist using the materials manager, and update the UI list.
  - Implement `delete_material` to remove the selected material with confirmation and update persistence.
  - Add undo or archive (soft delete) as an option for user safety.
  - Add tests to ensure round-trip persistence and UI updates.
- Prompt:
  - Implement form field extraction and persist to the `MaterialsManager` backend; ensure UI updates after save/delete.

---

## Step 6 — Designer Rotation
- Location: `crates/gcodekit5-ui/src/ui/gtk/designer.rs` (arc drawing/rotation TODO)
- Description: There's a TODO for rotation handling in shape rendering and in other parts of the designer.
- Proposed remediation:
  - Ensure shapes with rotation correctly apply affine transforms in rendering and bounding boxes.
  - Update rotation editing UI and handlers so rotations can be set on shape properties and the drawing reflects rotation.
  - Add tests and verify bounding box math, resize handles, and rotation-dependent geometries respond correctly.
- Prompt:
  - Implement `rotation` lifecycle for shapes: set/get, render by rotating the shape's transform, update handles accordingly.

---

## Step 7 — Designer File operations once shape structures aligned
- Location: `crates/gcodekit5-ui/src/ui/gtk/designer.rs` (File operations TODO near file ops)
- Description: File operations are planned but conditional on shape structure updates.
- Proposed remediation:
  - Align shape file serialization formats and integrate save/load/export logic consistently.
  - Implement export to common formats (SVG/GCode) with correct transformations/units conversions.
  - Add tests and integration tests for serialization/deserialization of shapes, and handle versioning of file formats for compatibility.
- Prompt:
  - Implement file export/import once shapes have a stable schema; add CLI/GUI flows to build shapes from persisted formats.

---

## Step 8 — Editor: Show error dialog on file I/O failures
- Location: `crates/gcodekit5-ui/src/ui/gtk/editor.rs` (multiple error handling spots)
- Description: File read/write error logging exists without user-facing dialogs.
- Proposed remediation:
  - Implement a generic `show_error_dialog(window, title, message)` helper or call `MainWindow::show_error_dialog` if available.
  - Replace `TODO: Show error dialog` with a call to present a helpful error message in a modal dialog.
  - Add tests using an integration harness or manual QA.
- Prompt:
  - Implement `show_error_dialog` function and call it in file read/write error cases.

---

## Step 9 — Firmware Settings: file load/save
- Location: `crates/gcodekit5-communication/src/firmware/settings.rs` (load_from_file and save_to_file TODOs)
- Description: Implement firmware settings persistence to a file format (JSON recommended).
- Proposed remediation:
  - Define `serde`-compatible `FirmwareSetting` and implement `load_from_file`/`save_to_file` with serde_json, handling validation for setting types.
  - Add unit tests for load/save and error handling.
- Prompt:
  - Add `serde` serialization for `FirmwareSetting` and implement `load_from_file`/`save_to_file` using `std::fs::read_to_string` and `write`.

---

## Step 10 — GRBL Controller listener registration/unregistration
- Location: `crates/gcodekit5-communication/src/firmware/grbl/controller.rs` (register_listener, unregister_listener)
- Description: Implement handling for listeners that want to receive status updates from GRBL controller.
- Proposed remediation:
  - Add a thread-safe storage (HashMap) of listener handles and support registration/unregistration.
  - Use a unique handle key and ensure clean removal.
  - Implement proper reference counting and cleanup.
  - Add tests for listener lifecycle.
- Prompt:
  - Implement `register_listener`, `unregister_listener`, and `listener_count` using a `RwLock` or similar to provide thread-safe registration.

---

## Notes & Next Steps
- This plan is intentionally high-level. Each Step above can be split further into more specific subtasks (UI changes, backend changes, tests, etc.).
- If you want, I can convert the above Steps into `bd` issues, or create GitHub issues/PRs with code scaffolding for the most critical steps.
- To proceed, choose one Step for implementation or let me know if you'd like the items prioritized by risk/impact.

Generated: 2025-12-11

