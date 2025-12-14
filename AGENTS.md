# Agent Guidelines for Rust/GTK4 apps
- First when you startup say "Hi im Rust Agent and I have read Agents.md" 

## Technology Stack
- **Language**: Rust edition 2021 or greater
- **UI Framework**: gtk-rs

Create and maintain a file called GTK4.md that contains all knowledge you have gained about the UI Crate, and all insghts and successfull stratergies. Reffer to this file if you have issues you cannot resolve. 

## Build Commands
- `cargo build` - Build debug binary, timeout is 600 seconds min
- `cargo build --release` - Build optimized release binary timeout is 600 seconds min
- `cargo check` - Check code without building
- Build only debug builds unless specificaly asked to perform a `release build`

Builds can take a long time so allow up to 600 seconds for a rebuild

## Test Commands
- `cargo test` - Run all tests
- `cargo test <test_function_name>` - Run specific test function
- `cargo test -- --nocapture` - Run tests with output visible
- `cargo test --lib` - Test library only (skip integration tests)
- **Test Timeout**: All test runs should have a 10-minute timeout to prevent hanging
  - Use `timeout 600 cargo test` on Unix/Linux
  - Use `cargo test --test-threads=1` for sequential execution if needed

### Test Organization
All tests **MUST** be located in the `tests/` inside each crate, if the test is at root level then it should be at the root tests/folder, NOT inline in source files:
- Use `#[test]` for sync tests and `#[tokio::test]` for async tests
- Import from the public `gcodekit5` crate (e.g., `use gcodekiti4::communication::GrblController;`)
- Be organized with related tests grouped together
- Follow naming convention: `test_<component>_<scenario>` (e.g., `test_jog_x_positive`)
- For each project crate, reorganise tests by:  migrating all tests related to the crate into relevant subfolders in the tests folder in the crate, review tests both inside crate and outside of it to find candidate tests for migration, then look at all .rs files that are outside of the tests folder in the crate and relocate all the inline tests found within them into seperate files in suitable subfolders in the crate tests folder 


## Lint & Format Commands
- `cargo clippy` - Run linter with clippy
- `cargo fmt` - Format code with rustfmt
- `cargo fmt --check` - Check formatting without changes

## Units ##
- DateTime vaules should be represented internaly in UTC and translated to locale based represetations in the UI layer. 
- Dimensional units should be represented internaly in mms, and be of type f32, and mm values should be represted to 2 decimal place accuracy. 
- All text strings where feasable should be internaly represented in UTF8 encoding, with translation to and from UI encoding in the UI layer if required. 
- All dimension and feed values in the UI must support switching between Metric (mm) and Imperial (inch) units.
- The UI should listen for measurement system changes and update displayed values accordingly.
- Internal calculations and storage should always be in millimeters (mm) for dimensions and mm/min for feed rates, unless specifically required otherwise by a protocol.
- Use the `gcodekit5_core::units` module for conversion and formatting.

## Esthetics ## 
- The aversage user will be using a FHD screen set to 125% scaling. ensure that all esthetics reviews, designs and recomendations account for this. 

## GitHub Access
- Use "gh" to access all GitHub repositories.
- When asked to "push to remote", update the SPEC.md, README.md, STATS.md, GTK4.md and CHANGELOG.md files with all recent activity and spec changes, construct a suitable commit message based on recent activity, commit all changes and push the changes to the remote repository.
- When asked to "push release to remote", update the release number, and then follow the "push to remote" process. **Commit Message Rule**: Do not use "chore: bump version to ...", instead use "Version: <version_number>".
- When initializing a new repo, add BUG, FEATURE, TASK and CHANGE issue templates only do this once. 
- **CRITICAL**: Do not push changes to remote unless specifically told to. This is a strict rule.
- Do not tag releases unless specifically told to. 

## Changelog Management
- **CHANGELOG.md**: Maintain a changelog in the root directory documenting all changes before each push to remote.
- **Format**: Follow Keep a Changelog format (https://keepachangelog.com/)
- **Update Timing**: Update CHANGELOG.md before each push to remote with the latest changes, features, fixes, and improvements.
- **Version**: Use semantic versioning (major.minor.patch-prerelease)
- **RELEASE.md**: write the version number and the most recent CHANGELOG.md entry to the RELEASE.md file for use as a Description in the Github Releases page. 

## Documentation Standards
- For all functions create DOCBLOCK documentation comments above each function that describes the purpose of the function, and documents any arguments and return values.
- For all modules place a DOCBLOCK at the top of the file that describes the purpose of the module, and any dependencies.
- **Documentation Files**: All documentation markdown files (*.md) **MUST** be located in the `docs/` folder, except for `STATS.md`, `GTK4.md`, `SPEC.md`, `AGENTS.md`, `README.md`, `PLAN.md` and `CHANGELOG.md` which remain in the project root. This includes: implementation guides, architecture documentation, feature specifications, task breakdowns, user guides, API references, and any other markdown documentation. Any future documentation should be created in the docs/ folder following this convention.
- Do not create explainer documents or other .md files unless specifically asked to.

## Code Style Guidelines
- **Formatting**: 4 spaces, max 100 width, reorder_imports=true, Unix newlines
- **Naming**: snake_case for functions/variables, PascalCase for types/structs/enums
- **Imports**: Group std, external crates, then local modules; reorder automatically
- **Error Handling**: Use `Result<T, E>` with `?`, `anyhow::Result` for main, `thiserror` for custom errors
- **Types**: Prefer explicit types, use type aliases for complex types
- **Logging**: Use `tracing` crate with structured logging, avoid `println!` or `eprintln!` in any phase of development. Performance profiling: Use `debug!()` for non-hot paths, `trace!()` for debug scenarios
- **Logging Cleanliness** after an issue has been resolved remove all debug! and tracing::debug! calls in the relevant code. 
- **Documentation**: `//!` for crate docs, `///` for public APIs, `//` for internal comments
- **Linting**: No wildcard imports, cognitive complexity ≤30, warn on missing docs
- **Best Practices**: Read the best practices at https://www.djamware.com/post/68b2c7c451ce620c6f5efc56/rust-project-structure-and-best-practices-for-clean-scalable-code and apply to the project.

## Workflow

1. When asked "whats next", present a list of the top 9 unimplemented tasks by task number, accept a task number and perform that task.
2. Don't suggest features unless asked to.
3. When debugging problems, use Occams razor and assume that the simplest solution is more likely to be the right one. 
4. Also when you are trying to debug a problem, change only one thing at a time, if it does not fix the problem then revert it, before trying another possible solution. 
5. DO NOT perform tempoary solutions or fixes, alway provide a complete solution. 
6. DO NOT declare an issue as fixed unless it has been confirmed, 90% of assetions of completion turn out to be false.
7. Latest Screenshots can be found in the folder ~/Pictures/screenshots.
8. when asked to recomend cleanups for an area of the codebase, review the code, and create a series of recomendations about the aesthetics and functionality and place them into a file called REVIEW.<Code area name>.md. each recomendation should be a descrete actionable change, that can be executed out of sequence, each should also have a recomendation number that allows it to be identified, and should have a prompt attached that describes the location of the code and the remediation task in detail. do not execute the tasks just document them.
9. When asked to implement the recomendations, reread the recommendations in the REVIEW file as the user may have edited them. 


## Versioning

1. During development the release number will have "-alpha" appended to the end as per semantic versioning standards. only when it is a production release will it be removed. 

## Temporary Files

1. Create a directory called "target" in the project root
2. Create a directory called "temp" in the target folder
3. Ensure that the target/temp folder is in the .gitignore file
4. Use target/temp for all temporary files, scripts and other ephemeral items that are normally placed in /tmp

## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**
```bash
bd ready --json
```

**Create new issues:**
```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Issue title" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**
```bash
bd update bd-42 --status in_progress --json
bd update bd-42 --priority 1 --json
```

**Complete work:**
```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:
- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### MCP Server (Recommended)

If using Claude or MCP-compatible clients, install the beads MCP server:

```bash
pip install beads-mcp
```

Add to MCP config (e.g., `~/.config/claude/config.json`):
```json
{
  "beads": {
    "command": "beads-mcp",
    "args": []
  }
}
```

Then use `mcp__beads__*` functions instead of CLI commands.

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems
- ❌ **Do NOT close issues by yourself** - Only close issues when explicitly instructed to do so

For more details, see README.md and QUICKSTART.md.
