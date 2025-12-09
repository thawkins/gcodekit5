# PR: Archive Slint UI — gcodekit5-visualizer

PR: https://github.com/thawkins/gcodekit5/pull/1

Purpose
- Archive the Slint UI file `gcode_visualizer.slint` by moving it to `ui/legacy`.
- Update imports in `crates/gcodekit5-ui/ui.slint` to import from the new `legacy` path.

Checklist
- [ ] Create branch `chore/archive-slint-gcodekit5-visualizer`
- [ ] Create `crates/gcodekit5-visualizer/ui/legacy` directory
- [ ] Move `ui/gcode_visualizer.slint` to `ui/legacy/gcode_visualizer.slint` (git mv)
- [ ] Update `crates/gcodekit5-ui/ui.slint` import: change path to `../gcodekit5-visualizer/ui/legacy/gcode_visualizer.slint`
- [ ] Update any tests referencing the visualizer slint components and gate them behind `slint_legacy_tests` feature
- [ ] Run `cargo build --workspace` and `cargo test --workspace` and fix any compile errors
- [ ] Commit changes and open PR

Notes
- This is an archive only: we keep the Slint UI file for historical reference and future porting.

Rollback plan
- Revert the moves if any regressions found or if further porting is required.