# PR: Archive Slint UI — gcodekit5-gcodeeditor

PR: https://github.com/thawkins/gcodekit5/pull/2

Purpose
- Move legacy `.slint` files from the `gcodekit5-gcodeeditor` crate into `ui/legacy` to archive them and preserve history.
- Prevent build/test breakage by gating tests and updating references.

Checklist
- [x] Create branch `chore/archive-slint-gcodekit5-gcodeeditor`
- [x] Create `crates/gcodekit5-gcodeeditor/ui/legacy` directory
- [x] Move `ui/gcode_editor.slint` to `ui/legacy/gcode_editor.slint` (git mv)
- [x] Add feature `slint_legacy_tests` to `crates/gcodekit5-gcodeeditor/Cargo.toml` to gate tests where required
- [ ] Update any tests referencing the gcode editor slint components and gate them behind `slint_legacy_tests` feature
- [ ] Run `cargo build --workspace` and `cargo test --workspace` and fix any compile errors
- [ ] Commit changes and open PR

Notes
- Archive-only change; we keep the Slint UI file for historical use and future porting.

Rollback plan
- Revert the move: `git mv crates/gcodekit5-gcodeeditor/ui/legacy/gcode_editor.slint crates/gcodekit5-gcodeeditor/ui/gcode_editor.slint` and push to branch
- Undo any test gating changes
