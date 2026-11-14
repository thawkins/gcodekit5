# Agent Guidelines for Rust apps
- First when you startup say "Hi im Rust Agent and I have read Agents.md"

## Available Tools

You have access to the following tools. Use ONLY these exact tool names — do not invent or guess tool names. The canonical names below are the only ones you should use:

### Core Tools (always available)
- `bash` — Execute a shell command. Use `command` to provide the command, or `code` for a code snippet.
- `read` — Read file contents with optional `start_line`/`end_line` range.
- `edit` — Replace an exact occurrence of text in a file.
- `multiedit` — Apply multiple edits across one or more files atomically.
- `write` — Create or overwrite a file.
- `create` — Create a new file with content.
- `append_to_file` — Append text to the end of a file.
- `grep` — Search file contents for a regex pattern using ripgrep.
- `glob` — Find files matching a glob pattern.
- `list` — List directory contents.
- `question` — Ask the user a question and wait for their typed response.
- `get_env` — Read environment variables.
- `file_info` — Return metadata for a file or directory.
- `diff_files` — Show a unified diff between two files or inline strings.
- `copy_file` — Copy a file to a new location.
- `move_file` — Move or rename a file or directory.
- `rm` — Delete a single file.
- `patch` — Apply a unified diff patch to one or more files.
- `mkdir` — Create a directory.
- `calculator` — Evaluate a mathematical expression.
- `think` — Record a short reasoning note without changing project state.
- `todo_read` — List TODO items for the current session.
- `todo_write` — Add, update, remove, or clear TODO items.
- `memory_read` — Read the contents of a memory file.
- `memory_write` — Persist notes or learnings to memory files.
- `memory_replace` — Replace a specific string in a named memory block.
- `memory_search` — Search memories using semantic similarity or keyword matching.
- `memory_store` — Store a structured memory with category, tags, and confidence score.
- `memory_forget` — Delete structured memories by ID or filter criteria.
- `memory_migrate` — Analyse a flat MEMORY.md file and propose splitting it into named blocks.
- `plan_enter` — Delegate to the plan agent for read-only codebase analysis.
- `codeindex_search` — Search the codebase index for symbols, functions, types, and documentation.
- `codeindex_symbols` — Query symbols (functions, structs, enums, traits) from the codebase index.
- `codeindex_references` �� Find all references to a symbol by name across the indexed codebase.
- `codeindex_dependencies` — Query file-level dependencies from the code index.
- `codeindex_status` — Show the current status and statistics of the codebase index.
- `codeindex_reindex` — Trigger a full re-index of the codebase.

### Code Intelligence Decision Flow
When the codebase index is active, you MUST use `codeindex` tools instead of `grep` for code symbol queries. The index is faster, returns structured results, and understands symbol kinds.

| Query type | Use |
|---|---|
| "Where is function X defined?" | `codeindex_search` (NOT grep) |
| "Find all structs matching Y" | `codeindex_symbols` |
| "Who calls function Z?" | `codeindex_references` |
| "What does file A import?" | `codeindex_dependencies` |
| "Is the index working?" | `codeindex_status` |
| "Re-index after bulk edits" | `codeindex_reindex` |

When searching for arbitrary text strings, comments, or non-symbol content, use `grep` with the `pattern` parameter. **Do NOT use `search` or `search_in_repo`** — these are not available tools. Always use `grep` for all text and pattern matching across files.

**CRITICAL — grep parameter requirement:**
The `grep` tool requires the `pattern` parameter. This is the ONLY required field. Do NOT omit it. Example:
```
grep(pattern: "fn main", path: "src")
```

**CRITICAL — grep is the ONLY text search tool:**
There is no `search` or `search_in_repo` tool. Use `grep` for every text search need, whether it's a regex pattern or a plain text string. There are no aliases or shortcuts.

### Shell Execution Rules
- For simple commands or code snippets, use `bash` with the `command` or `code` parameter.
- Timeout defaults to 120 seconds.
- The `bash_reset` tool resets the persistent shell state.

### Important
Do NOT use `execute_bash`, `execute_code`, `execute_python`, `run_shell_command`, or `run_terminal_cmd` — these are deprecated aliases. Always use the canonical `bash` tool.

## Technology Stack
- **Language**: Rust edition 2024 or greater

## Build Commands
- `cargo build` - Build debug binary; allow up to 600 seconds.
- `cargo build --release` - Build optimized release binary; allow up to 600 seconds.
- `cargo check` - Check code without building.
- Build only debug builds unless specifically asked to perform a `release build`.

Builds can take a long time, so allow up to 600 seconds for a rebuild.

## Test Commands
- `cargo test` - Run all tests
- `cargo test <test_function_name>` - Run specific test function
- `cargo test -- --nocapture` - Run tests with output visible
- `cargo test --lib` - Test library only (skip integration tests)
- **Test Timeout**: All test runs should have a 10-minute timeout to prevent hanging
  - Use `timeout 600 cargo test` on Unix/Linux
  - Use `cargo test --test-threads=1` for sequential execution if needed

### Test Organization

All tests **MUST** be located in the `tests/` directory inside each crate. If a test is at the workspace root, it should be placed in the root `tests/` folder, not inline in source files.
- Use `#[test]` for sync tests and `#[tokio::test]` for async tests.
- Import from the relevant public crate for the crate under test rather than assuming a single `ragent` crate path.
- Organize related tests together.
- Follow the naming convention: `test_<component>_<scenario>` (e.g. `test_jog_x_positive`).
- For each project crate, migrate related tests into suitable subfolders within that crate's `tests/` directory. Review both inline and external tests for migration candidates, and relocate inline tests from `.rs` files into separate files under the appropriate `tests/` subfolder where practical.


## Lint & Format Commands
- `cargo clippy` - Run linter with clippy
- `cargo fmt` - Format code with rustfmt
- `cargo fmt --check` - Check formatting without changes

## Units
- DateTime values should be represented internally in UTC and translated to locale-based representations in the UI layer.
- Dimensional units should be represented internally in millimetres (`mm`) as `f32`, and presented to 2 decimal places where relevant.
- Text should be represented internally as UTF-8 where feasible, with translation to and from UI-specific encodings only when required.

## GitHub Access
- Use "gh" to access all GitHub repositories.
- When asked to "push to remote", update the SPEC.md, README.md, STATS.md, RELEASE.md, QUICKSTART.md and CHANGELOG.md files with all recent activity and spec changes, construct a suitable commit message based on recent activity, commit all changes and push the changes to the remote repository.
- When asked to "push release to remote", update the release number, and then follow the "push to remote" process. **Commit Message Rule**: Do not use "chore: bump version to ...", instead use "Version: <version_number>".
- When initializing a new repo, add BUG, FEATURE, TASK and CHANGE issue templates only do this once. 
- **CRITICAL**: Do not push changes to remote unless specifically told to. This is a strict rule.
- Do not tag releases unless specifically told to. 
- DO NOT use "git checkout" to rewind files, this ALWAYS results in lost work. 

## Changelog Management
- **CHANGELOG.md**: Maintain a changelog in the root directory documenting all changes before each push to remote.
- **Format**: Follow Keep a Changelog format (https://keepachangelog.com/)
- **Update Timing**: Update CHANGELOG.md before each push to remote with the latest changes, features, fixes, and improvements.
- **Version**: Use semantic versioning (major.minor.patch-prerelease)
- **RELEASE.md**: write the version number and the most recent CHANGELOG.md entry to the RELEASE.md file for use as a Description in the Github Releases page. 
- Whenever a new feature or function is added ensure that SPEC.md and QUICKSTART.md is updated if relevant. 

## Documentation Standards
- For all functions create DOCBLOCK documentation comments above each public function that describes the purpose of the function, and documents any arguments and return values.
- For all modules place a DOCBLOCK at the top of the file that describes the purpose of the module, and any dependencies.
- **Documentation Files**: All documentation markdown files (`*.md`) **SHOULD** be located in the `docs/` folder, except for `QUICKSTART.md`, `RELEASE.md`, `STATS.md`, `SPEC.md`, `AGENTS.md`, `README.md`, `PLAN.md`, and `CHANGELOG.md`, which remain in the project root. Existing root-level project documents that predate this convention may remain until they are explicitly reorganized. When updating legacy root-level documents, prefer moving or consolidating them into `docs/` unless they are one of the approved root exceptions. Any future documentation should be created in the `docs/` folder following this convention.
- Do not create explainer documents or other `.md` files unless specifically asked to.

## Code Style Guidelines
- **Formatting**: 4 spaces, max width 100, reorder imports automatically, Unix newlines.
- **Naming**: snake_case for functions/variables, PascalCase for types/structs/enums.
- **Imports**: Group std, external crates, then local modules.
- **Error Handling**: Use `Result<T, E>` with `?`, `anyhow::Result` for main, and `thiserror` for custom errors.
- **Types**: Prefer explicit types, and use type aliases for complex types.
- **Logging**: Use the `tracing` crate with structured logging; avoid `println!` or `eprintln!` in application code. For performance profiling, use `debug!()` for non-hot paths and `trace!()` for debug scenarios.
- **Logging Cleanliness**: After an issue has been resolved, remove temporary `debug!()` and `tracing::debug!()` calls in the relevant code.
- **Documentation**: Use `//!` for crate/module docs, `///` for public APIs, and `//` for internal comments.
- **Linting**: No wildcard imports. Treat cognitive complexity ≤30 and missing docs warnings as review targets rather than guaranteed compiler-enforced limits.
- **Best Practices**: Read the best practices at https://www.djamware.com/post/68b2c7c451ce620c6f5efc56/rust-project-structure-and-best-practices-for-clean-scalable-code and apply them to the project where relevant.

## Thinking / Reasoning Configuration

- New code should use the typed `ChatRequest.thinking: Option<ThinkingConfig>` path for provider reasoning controls.
- Do not introduce new writes to legacy `options["thinking"]` or related ad-hoc fields except when maintaining backward-compatibility shims.
- When documenting or configuring defaults, use provider/model `thinking` blocks in `ragent.json` and the `/thinking` command terminology (`auto`, `off`, `low`, `medium`, `high`).

## Team Workflow

When asked to use a team or when a task benefits from parallel reviewers / workers:


1. **Create the team**: Use `team_create` with an appropriate `blueprint` (e.g. `code-review`).
   **Always pass `context`** — the user's specific request details: which directories/files to
   target, what task to perform, and where to write output. This context is prepended to every
   teammate's spawn prompt so they know exactly what to work on.
2. **Wait for results**: Call `team_wait` after creation. This blocks until every teammate becomes idle. **Do NOT use `wait_tasks` for teammates — `wait_tasks` only tracks `new_task` sub-agents.**
3. **Read results**: Use `team_status` or read the team's output files to collect teammate findings.
4. **Do not duplicate work**: Do not independently read files or do analysis that a teammate is already doing. Wait for them first.

```
team_create blueprint="code-review" context="Review the crates/ragent-server directory for security, test coverage, and performance issues. Write findings to docs/COMPLIANCE.md"
team_wait                          ← REQUIRED: blocks until all idle
team_status                        ← read what they found
```


1. Don't suggest features unless asked to.
2. When debugging problems, use Occam's razor and assume that the simplest solution is more likely to be the right one.
3. When debugging a problem, change only one thing at a time. If it does not fix the problem, revert it before trying another possible solution.
4. DO NOT perform temporary solutions or fixes; always provide a complete solution.
5. DO NOT declare an issue as fixed unless it has been confirmed; 90% of assertions of completion turn out to be false.

## Versioning

1. During development the release number will have `-alpha` appended to the end in line with semantic versioning conventions. Only remove it for a production release.

## Temporary Files

1. Use the existing `target/` directory in the project root for build artifacts.
2. Create and use a `target/temp` directory for temporary files, scripts, and other ephemeral items that would normally be placed in `/tmp`.
3. Ensure that the `target/temp/` path is present in `.gitignore`.


## Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

For more details, see README.md and QUICKSTART.md.
