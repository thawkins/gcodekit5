# PR: Archive Slint UI — gcodekit5-gcodeeditor

Purpose
- Move legacy `.slint` files from the `gcodekit5-gcodeeditor` crate into `ui/legacy` to archive them and preserve history.
- Prevent build/test breakage by gating tests and updating references.

Checklist
- [ ] Create branch `chore/archive-slint-gcodekit5-gcodeeditor` (done)
- [ ] Create `crates/gcodekit5-gcodeeditor/ui/legacy` directory
- [ ] Move `ui/gcode_editor.slint` to `ui/legacy/gcode_editor.slint` (git mv)
- [ ] Update tests in `crates/gcodekit5-gcodeeditor/tests` to be gated behind `slint_legacy_tests` feature or mark with `#[ignore]` where needed
- [ ] Update README and docs references if any
- [ ] Run `cargo build --workspace` and `cargo test --workspace` and fix immediate compile errors
- [ ] Commit changes to the branch and open a PR
- [ ] Add reviewers and include the rationale: "Archive Slint files to prepare migration to GTK4 — not deleting and history preserved."

Notes
- If any consumers import `ui/gcode_editor.slint` directly, update imports and tests to use the new `ui/legacy` path.

Rollback plan
- Revert the move: `git mv crates/gcodekit5-gcodeeditor/ui/legacy/gcode_editor.slint crates/gcodekit5-gcodeeditor/ui/gcode_editor.slint` and push to branch
- Reverse any test gating changes
