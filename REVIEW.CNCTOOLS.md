# REVIEW.CNCTOOLS.md

Date: 2025-12-14
Area: **CNC Tools Tab**

Scope reviewed:
- UI: `crates/gcodekit5-ui/src/ui/gtk/tools_manager.rs`
- Backend: `crates/gcodekit5-ui/src/ui/tools_manager_backend.rs`
- Data model: `crates/gcodekit5-core/src/data/tools.rs`
- Help: `crates/gcodekit5-ui/resources/markdown/tools_manager.md`

Assumptions:
- Typical user display: **FHD @ 125% scaling** (per `AGENTS.md`).

---

## Recommendations

### CNCTOOLS-001 — Stop forcing the Paned divider every frame
**Prompt:** In `crates/gcodekit5-ui/src/ui/gtk/tools_manager.rs` the `Paned` uses `add_tick_callback` to continuously set the position (~20% width). This prevents the user from resizing the sidebar and causes “UI fighting the user”. Replace with a one-shot initialization (e.g., set position on first map/realize/size-allocate only once) and/or persist user preference.

### CNCTOOLS-002 — Replace the hidden Label “tool id” hack with widget data
**Prompt:** In `tools_manager.rs` list rows store tool id by appending a hidden `Label` and then iterating children to find it on activation. Replace with `glib::ObjectExt::set_data()`/`set_property()`/`set_widget_name()` or attach the id to the `ListBoxRow` via `set_data` so selection is robust and does not depend on widget tree structure.

### CNCTOOLS-003 — Use `ListBoxRow` + selection-changed (not activate) for editing
**Prompt:** The list uses `connect_row_activated`, which requires click/Enter activation and is coupled to the hidden label scan. Prefer `ListBox::connect_selected_rows_changed` (or single selection) and load the selected tool immediately. This improves keyboard navigation and expected GTK behavior.


### CNCTOOLS-005 — Enforce units + formatting consistently (mm, 3dp)
**Prompt:** Standardize all displayed/editable dimensional values to **mm, f32, 3dp** (per `AGENTS.md`). Ensure that when a tool is loaded, values are formatted the same way they’re shown in the list (e.g., diameter `Ø{:.2}mm` matches edit fields).

### CNCTOOLS-006 — Implement actual Tool Material mapping on save/load
**Prompt:** In `load_tool_for_edit()`, `edit_material` is forced to Carbide and not mapped from `tool.material`. In `save_tool()`, the tool is always saved as `ToolMaterial::Carbide` regardless of UI selection. Implement bidirectional mapping using helper functions (`string_to_tool_material` exists in backend) so the UI reflects and persists the real value.

### CNCTOOLS-007 — Implement coating mapping on save/load (and support None)
**Prompt:** `edit_coating` includes “None/TiN/TiAlN/DLC/AlOx” but `save_tool()` always sets `coating = None` and `load_tool_for_edit()` sets coating combobox to 0 regardless. Implement bidirectional mapping to `Option<ToolCoating>` and preserve existing values.

### CNCTOOLS-008 — Add/Expose missing geometry fields based on ToolType
**Prompt:** The `Tool` model supports `corner_radius` and `tip_angle`, but UI does not allow editing them. Add conditional UI in Geometry tab that shows the relevant fields:
- Corner radius for `EndMillCornerRadius`
- Tip angle for `VBit`, `DrillBit`, `SpotDrill`, etc.

### CNCTOOLS-009 — Add “Shank” editing or remove misleading auto-assignment
**Prompt:** `Tool` has `ShankType`, but UI doesn’t offer shank selection; `save_tool()` always sets `ShankType::Straight((shaft_dia*10) as u32)`. Either (a) add a Shank section (Straight/Collet/Tapered) or (b) default to `Collet` consistently, or (c) clearly document that shank is derived from shaft diameter.

### CNCTOOLS-010 — Validate uniqueness of Tool ID
 Add validation:
- `ToolId` must be non-empty and unique
- remove Tool Number as it is not a tool on the device. remove from UI and database schema

### CNCTOOLS-011 — Add “dirty state” and enable Save only when changed
**Prompt:** Save/Cancel are enabled any time the edit form is shown. Track whether any field has changed compared to the loaded tool, and only enable Save when dirty. Provide a “discard changes?” confirmation when switching selection or leaving edit mode with unsaved changes.

### CNCTOOLS-012 — Add explicit sorting and show sort controls
**Prompt:** `load_tools()` lists tools in whatever order the backend returns. Sort deterministically (e.g., tool id ascending, then name) and optionally add a sort dropdown (Number/Name/Diameter/Type). This reduces “jumping” and helps find tools.

### CNCTOOLS-013 — Improve list row info density (material, flutes, length)
**Prompt:** The list rows show “type + diameter” only. Add a second line that includes key specs like `Ø`, flutes, flute length, material/coating to reduce clicks (especially on FHD @125%). Keep wrapping but consider max width and alignment.

### CNCTOOLS-014 — Expand filters beyond tool type (diameter range, material)
**Prompt:** There’s a type filter and search box. Add optional filters:
- Diameter range (min/max)
- Material/coating
- Flute count
Or at minimum allow search to match tool number and diameter strings.

### CNCTOOLS-015 — Wire up existing GTC import functionality into the UI
**Prompt:** Backend already provides `import_gtc_package()` and `import_gtc_json()`, but the UI offers no Import action. Add Import buttons (e.g., “Import .zip”, “Import .json”) with file chooser and success/failure summary.

### CNCTOOLS-016 — Add Export for custom tools (JSON) + “reset to defaults”
**Prompt:** Add “Export Tools…” to write custom tools (or full library) to JSON for backup/sharing; add “Reset Custom Tools” to clear the persisted file (with confirmation). Backend already has persistence; extend with explicit user control.

### CNCTOOLS-017 — Use consistent iconography and avoid emoji-only semantics
**Prompt:** Buttons use emoji (➕ 💾 ❌ 🗑️). Consider replacing with GTK icons (or keep emoji but also use proper `accessible_name`/tooltip). Ensure the “destructive-action” delete button is visually distinct and has a tooltip.

### CNCTOOLS-018 — Improve help content for the tab
**Prompt:** `resources/markdown/tools_manager.md` is minimal. Expand help to include:
- Definitions of tool fields (diameter vs shaft diameter vs flute length)
- Recommended conventions for tool ids/numbers
- Notes about units (mm) and typical defaults
- Workflow examples (create tool → use in CAM)

### CNCTOOLS-019 — Prevent silent fallback defaults on parse failure
**Prompt:** `save_tool()` uses `unwrap_or(6.35)` etc, which silently changes values. Replace with validation errors surfaced in the UI (e.g., mark invalid entries in red, show a message, block Save).

### CNCTOOLS-020 — Persist UI state (selected tool, filter, search)
**Prompt:** On opening the Tools tab, the search/filter resets and no tool is selected. Persist last used filter/search and last selected tool id (best-effort) in settings so the tab is “stateful” and faster to use.
