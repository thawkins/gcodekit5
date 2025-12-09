# PR: Archive Slint UI — gcodekit5-gcodeeditor

PR: https://github.com/thawkins/gcodekit5/pull/2

Purpose
- Remove legacy `.slint` files from the `gcodekit5-gcodeeditor` crate as part of Slint UI deletion.
- [x] Removed `ui/gcode_editor.slint` and legacy copies.
- [ ] Update any tests referencing the gcode editor slint components and gate them behind `slint_legacy_tests` feature
- [ ] Run `cargo build --workspace` and `cargo test --workspace` and fix any compile errors
- [ ] Commit changes and open PR

Notes
- Archive-only change; we keep the Slint UI file for historical use and future porting.

Rollback plan
- Revert the move: `git mv crates/gcodekit5-gcodeeditor/ui/legacy/gcode_editor.slint crates/gcodekit5-gcodeeditor/ui/gcode_editor.slint` and push to branch
- Undo any test gating changes
