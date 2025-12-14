# Device Manager – Cleanup Recommendations

Scope:
- UI: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Backend: `crates/gcodekit5-devicedb/src/{manager.rs,ui_integration.rs}`

Each item is discrete and can be done independently.

---

## DM-001 — Replace hidden ID label with row data attachment
**Problem:** Device ID is stored as a hidden `Label` child and retrieved by iterating children (`connect_row_activated`, lines ~627–647). This is fragile and easy to break with future UI tweaks.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- In `load_devices()` change each row to be a `ListBoxRow` and store the `device_id` on the row via `glib::ObjectExt::set_data("device_id", ...)` (or a small custom row struct).
- In `connect_row_activated`, read `row.data::<String>("device_id")` (or equivalent) and call `load_device_for_edit()`.

---

## DM-002 — Ensure combo boxes reflect the loaded model
**Problem:** `load_device_for_edit()` sets `edit_port`, `edit_timeout`, etc., but doesn’t set the active item for `edit_device_type`, `edit_controller_type`, `edit_connection_type`, `edit_baud_rate` (comment says “simplified”). This makes edits non-WYSIWYG and can cause accidental changes on save.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- In `load_device_for_edit()`, set each `ComboBoxText` active item by matching `profile.*` to `combo.active_text()` / iterating items.
- Ensure `baud_rate` and `connection_type` are also updated.

---

## DM-003 — Disable/enable fields based on connection type
**Problem:** Connection UI always shows Serial + TCP fields; this creates confusion.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- In `create_connection_tab()` and/or event handlers, when `connection_type` is Serial: enable Port + Baud, disable TCP Port.
- When TCP/IP: enable TCP Port, disable Serial Port + Baud.
- When WebSocket: show/enable relevant fields (likely hostname/url and port) or at least disable irrelevant fields.

---

## DM-004 — Use `SpinButton` for numeric fields instead of freeform `Entry`
**Problem:** Many numeric fields are plain `Entry` (`timeout_ms`, max s-value, wattage). This increases invalid data and parsing fallbacks. axis min/max, max feed are dimensional values and should use the support for dynamic units already implemented and driven from the prefferences switchs. 

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Replace numeric `Entry` widgets with `SpinButton` (with sensible ranges and increments) for:
  - timeout (ms), tcp_port, axis min/max, max_feed_rate, max_s_value, spindle/laser watts.
- Update `save_device()` to read from `SpinButton::value()` instead of `Entry::text()`.

---

## DM-005 — Add inline validation + user feedback (instead of silent defaults)
**Problem:** Backend parsing in `DeviceUiController::update_profile_from_ui()` uses many `parse().unwrap_or(default)` fallbacks; bad input is silently “corrected” without telling the user.

**Prompt:**
- File: `crates/gcodekit5-devicedb/src/ui_integration.rs`
- Change `update_profile_from_ui()` to validate fields and return structured errors (e.g., `anyhow::bail!("X Min must be a number")`).
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- On save error, show a GTK `MessageDialog` describing what to fix.

---

## DM-006 — Stop auto-moving the `Paned` divider every frame
**Problem:** `add_tick_callback` continuously forces the divider to 20% width. This fights user resizing and wastes work. the initial value should be 20%. 

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Replace `add_tick_callback` with a one-time initialization:
  - set initial position on first map/realize or first tick then remove callback.
  - optionally persist divider position in settings.

---

## DM-007 — Improve list item hierarchy + spacing consistency
**Problem:** Rows are `Box` widgets appended directly to `ListBox`, which loses some row-specific styling/behavior.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Wrap each `row_box` in a `ListBoxRow` and set margins on the row rather than on children.
- Ensure row content uses consistent spacing and alignment (badge alignment, wrap rules).

---

## DM-008 — Add search/filter for devices list
**Problem:** No search/filter; will degrade with more profiles.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Add a `SearchEntry` above the `ListBox`.
- Filter `profiles` by name/description/controller type before populating list.

---

## DM-009 — Confirm destructive delete
**Problem:** Delete action immediately deletes without confirmation.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- In `delete_device()`, show a confirmation dialog with device name; only proceed if confirmed.

---

## DM-010 — Fix duplicate assignment in update_profile_from_ui
**Problem:** `profile.has_coolant` is assigned twice (lines ~143–148).

**Prompt:**
- File: `crates/gcodekit5-devicedb/src/ui_integration.rs`
- Remove the duplicate assignment and keep a single, clear assignment.

---

## DM-011 — Normalize units and formatting in UI
**Problem:** Axis values and limits are freeform strings; formatting is inconsistent and can display excessive precision.

**Prompt:**
- File: `crates/gcodekit5-devicedb/src/ui_integration.rs`
- When converting `DeviceProfile -> DeviceProfileUiModel`, format floats to a fixed precision (e.g., 3 decimals for mm).
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Add unit labels (mm, mm/min, W, ms) next to relevant fields, based on the units control in prefferences.

---

## DM-012 — Improve “Connected” badge matching
**Problem:** “Connected” badge matches by `profile.port == connected_port` (string compare). This can be wrong if ports differ by symlink/format, or for TCP connections.

**Prompt:**
- File: `crates/gcodekit5-ui/src/ui/gtk/device_manager.rs`
- Update connected matching to consider `connection_type` and relevant identifier:
  - Serial: compare canonical/normalized port strings.
  - TCP/WebSocket: compare host+port.
- Consider storing the active connection target in `device_status` as a structured type.
