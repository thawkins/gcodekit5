# Codebase Reorganization Proposal

## Executive Summary
The current `gcodekit5` codebase structure suffers from role ambiguity in the root directory, creating confusion between the workspace configuration and the application logic. This proposal outlines a plan to refactor the project into a strict **Virtual Workspace**, moving all application code into specific crates and eliminating the root `src/` directory. This will improve maintainability, simplify dependency management, and clarify the architectural boundaries.

## Current State Analysis
*   **Root Ambiguity:** The root `Cargo.toml` acts as both a workspace definition and a package manifest. The root `src/` directory contains a mix of:
    *   A thin binary entry point (`main.rs`) that delegates immediately to `gcodekit5-ui`.
    *   A massive library facade (`lib.rs`) that re-exports symbols from sub-crates but is unused by the workspace itself.
    *   Misplaced application state (`types.rs`).
    *   Legacy compatibility shims (`app/`).
*   **Hidden Core:** The actual application logic resides in `crates/gcodekit5-ui`, while the root crate pretends to be the application.
*   **Legacy Debt:** The `src/app` directory appears to be left over from a previous refactoring.

## Goals
1.  **Clear Separation of Concerns:** Decouple the workspace management from the application code.
2.  **Simplified Navigation:** Developers should know exactly where the entry point and core logic reside without jumping between root `src/` and `crates/`.
3.  **Eliminate Dead Code:** Remove unused facades and legacy shims.

## Proposed Changes

### Phase 1: Cleanup & Relocation
Before structural changes, we should clean up the existing artifacts.

1.  **Remove Legacy Shims:** Delete `src/app` and `src/app/types.rs`. Update any lingering references (though none are expected in active code).
2.  **Relocate Orphaned Logic:** Move `GcodeSendState` from `src/types.rs` to `crates/gcodekit5-core` (or `gcodekit5-communication` if strictly related to sending).
3.  **Evaluate Facade:** Determine if the re-exports in `src/lib.rs` serve any external consumers (e.g., is this published as a library?). If not, plan for its removal.

### Phase 2: Establish Virtual Workspace
Convert the root into a pure workspace.

1.  **Create Application Crate:**
    *   Rename `crates/gcodekit5-ui` to `crates/gcodekit5-app` (or keep as `ui` if it strictly handles UI, but it seems to be the main integrator).
    *   Move the logic from root `src/main.rs` into `crates/gcodekit5-app/src/main.rs`.
2.  **Update Root Manifest:**
    *   Modify root `Cargo.toml` to remove `[package]` and `[dependencies]`. It should contain only `[workspace]`.
3.  **Delete Root Source:** Remove the root `src/` directory entirely.

### Phase 3: Standardization
Ensure consistency across the `crates/` directory.

1.  **Consistent Naming:** Verify all crates follow the `gcodekit5-*` naming convention (already mostly done).
2.  **Shared Dependencies:** Consider using `[workspace.dependencies]` in the root `Cargo.toml` to manage versions of common libraries (like `serde`, `gtk4`, `tokio`) centrally.

## Expected Directory Structure
After these changes, the project will look like this:

```text
/home/thawkins/Projects/gcodekit5/
├── Cargo.toml          (Virtual Manifest: [workspace] only)
├── crates/
│   ├── gcodekit5-app/  (Formerly ui + root main.rs. The binary entry point.)
│   ├── gcodekit5-core/
│   ├── gcodekit5-communication/
│   ├── ... (other crates)
├── assets/
├── ...
```

## Benefits
*   **Zero Ambiguity:** "Where is main?" -> `crates/gcodekit5-app/src/main.rs`.
*   **Clean Build Graph:** The root is just a container; it doesn't compile code itself.
*   **Better Tooling Support:** Rust tools (RA, Cargo) often handle virtual workspaces more predictably than mixed package/workspaces.
