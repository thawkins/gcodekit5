# REVIEW.MATERIALS.md

Scope: Materials tab UI and supporting backend.

Code locations reviewed:
- UI: `crates/gcodekit5-ui/src/ui/gtk/materials_manager.rs`
- Backend: `crates/gcodekit5-ui/src/ui/materials_manager_backend.rs`
- Help: `crates/gcodekit5-ui/resources/markdown/materials_manager.md`
- App wiring: `crates/gcodekit5-ui/src/gtk_app.rs` (Materials tab registration)

Each recommendation below is discrete and can be executed independently.

---

## R.MAT.001 — Remove per-frame Paned resizing (tick callback)
**Type:** Performance / UX

**Prompt:** In `crates/gcodekit5-ui/src/ui/gtk/materials_manager.rs`, the `Paned` is continuously repositioned via `widget.add_tick_callback(...)` near the end of `MaterialsManagerView::new()`.
- Replace this with a one-time initial sizing approach (e.g., connect to `realize` or `size-allocate`, or set initial position after first layout) and then stop adjusting.
- Ensure the user can drag the divider and it stays where they set it.

---

## R.MAT.002 — Make category filter actually filter the list
**Type:** Functional correctness

**Prompt:** In `MaterialsManagerView::load_materials()`, the code reads `search_entry.text()` but ignores `category_filter`.
- Use `self.category_filter.active_id()` to determine category selection.
- Apply category filtering (and then apply search) so the UI matches the visible filter control.
- Keep “All Categories” as a no-filter path.

---

## R.MAT.003 — Replace hidden-label MaterialId storage with row/user-data
**Type:** Maintainability / correctness

**Prompt:** In `materials_manager.rs`, list rows are built as a `gtk4::Box` appended into a `ListBox`, and the material ID is stored in a hidden `Label` (see creation in `load_materials()` and lookup in `connect_row_activated`).
- Replace the hidden widget approach with row-attached data (`glib::ObjectExt::set_data` / `data`) or migrate the list to `ListView` with `gio::ListStore` so selection yields the model item directly.
- This avoids brittle “walk children until you find an invisible label” logic.

---

## R.MAT.004 — Fix category selection logic in the edit form
**Type:** Functional correctness

**Prompt:** In `load_material_for_edit()`, category is set by looping `for i in 0..7` but checking `self.edit_category.active_text()` (which does not change until `set_active` is called).
- Replace the loop with a mapping from `MaterialCategory` to combo index (or use IDs via `append(Some(id), label)` + `set_active_id`).
- Ensure it selects the correct category reliably.

---

## R.MAT.005 — Use IDs consistently for all ComboBoxText widgets
**Type:** Maintainability

**Prompt:** The sidebar filter uses `append(Some("wood"), "Wood")` style IDs, but the edit form uses `append_text()`.
- Standardize on ID-based combo population for edit widgets too (`edit_category`, `edit_chip_type`, `edit_heat_sensitivity`, etc.).
- Then set values using `set_active_id(Some("...") )` instead of repeated string/loop comparisons.

---

## R.MAT.006 — Hide the ID field unless creating new material
**Type:** UX

**Prompt:** `create_general_tab()` includes an “ID” row with a comment “shown only when creating” but it is always appended and visible.
- Store the container for the ID row (e.g., `id_grid`) in `MaterialsManagerView`.
- Toggle visibility: visible during `start_create_new()`, hidden during `load_material_for_edit()` and `cancel_edit()`.
- Keep `edit_id` insensitive when editing existing materials.

---

## R.MAT.007 — Replace emoji button labels with GTK icon buttons
**Type:** Aesthetics / Accessibility

**Prompt:** Buttons currently include emoji in labels (e.g., “💾 Save”, “❌ Cancel”, “🗑️ Delete”, “➕ New Material”).
- Replace with icon-based buttons (`icon_name` + text, or `ButtonContent`) using symbolic icons for theme consistency.
- Ensure accessible labels/tooltips exist (screen reader friendly) and the layout remains clean at FHD @ 125% scaling.

---

## R.MAT.008 — Add an “empty state” for no selection and no search results
**Type:** UX

**Prompt:** When there is no selected material (initial state / after cancel) the right pane shows an editable form but is effectively blank.
- Add a placeholder view (e.g., a `Stack` page) that says “Select a material” and/or “No results” when the list is empty.
- Only show the form stack when a material is selected or a new material is being created.

---

## R.MAT.009 — Separate view state from widget references
**Type:** Maintainability

**Prompt:** `MaterialsManagerView` stores many widget fields directly (entries, combos, textviews) and also tracks `selected_material` + `is_creating`.
- Introduce a small internal “form model” struct or helper methods that:
  - `read_form() -> MaterialDraft`
  - `write_form(&Material)`
  - `set_form_sensitive(bool)`
- This reduces duplicated “set text / set active / set value” code scattered across methods.

---

## R.MAT.010 — Implement Save/Delete wiring with backend (remove TODO stubs)
**Type:** Functional completeness

**Prompt:** `save_material()` and `delete_material()` are TODO stubs that only refresh the list and cancel.
- Implement them to:
  - Validate inputs
  - Add/update/remove materials via `MaterialsManagerBackend`
  - Respect `material.custom` constraints for delete
- Ensure errors are surfaced to the user (dialog/toast), not silently ignored.

---

## R.MAT.011 — Add basic input validation + user feedback
**Type:** UX / correctness

**Prompt:** Currently the UI allows empty/invalid values (e.g., blank name, non-numeric tensile strength/melting point entries).
- Add validations before save:
  - required: name, category, subcategory
  - parse optional numeric fields; show inline error or dialog if invalid
  - ensure ID uniqueness when creating
- Disable “Save” until the form is valid (or show validation message on click).

---

## R.MAT.012 — Ensure list refresh preserves selection where possible
**Type:** UX

**Prompt:** After save/delete, the list is rebuilt from scratch and selection is lost.
- Track selected `MaterialId` and re-select it after refresh (if it still exists), otherwise select a sensible neighbor.
- When creating, after save, select the newly created material.

---

## R.MAT.013 — Add sort order and grouping for the materials list
**Type:** UX

**Prompt:** `load_materials()` iterates `search_materials()` results in whatever order the backend returns.
- Add deterministic sorting (e.g., by category then name, or name only).
- Optionally group by category with separators/headers (especially useful once the library grows).

---

## R.MAT.014 — Centralize default form values
**Type:** Maintainability

**Prompt:** Defaults like density=750 and machinability=7 appear in `create_properties_tab()` and again in `clear_form()`.
- Centralize into one place (constants or `Default` for a draft struct) so changing defaults doesn’t require editing multiple functions.

---

## R.MAT.015 — Move repeated margins/spacing into CSS classes for consistent layout
**Type:** Aesthetics / maintainability

**Prompt:** Tabs repeatedly set margins/spacing manually (`set_margin_top/bottom/start/end` across pages).
- Create/standardize CSS classes (e.g., `.page-padding`, `.form-grid`) and apply them.
- Verify the layout remains balanced and readable at FHD @ 125% scaling.

---

## R.MAT.016 — Improve help content to match current UI capabilities
**Type:** Documentation / UX

**Prompt:** `crates/gcodekit5-ui/resources/markdown/materials_manager.md` claims “Store recommended feeds/speeds per tool/material”, but the current UI does not expose feeds/speeds.
- Update help text to reflect current functionality, or add a short “Planned” section.
- Add links to related tabs/features that actually exist.
