# Rust-Specific Project Guidelines

When loaded, first say: "Rust language instructions loaded."

## MANDATORY: Run `cargo fmt` After EVERY Rust File Edit

**THIS IS THE MOST IMPORTANT RULE IN THIS FILE. READ IT TWICE.**

Rust is NOT Python, NOT JavaScript, NOT Go. Rust files have strict, non-negotiable
formatting rules enforced by `rustfmt`. If you try to hand-format a Rust file the way
you would format Python (indentation, brace placement, spacing), you WILL get it wrong
and break the build with `cargo fmt --check` failures.

**You MUST run `cargo fmt` after writing or editing ANY `.rs` file. No exceptions.**

### The Rule

After you create, edit, or append to ANY Rust source file (`.rs`), you MUST run:

```bash
cargo fmt
```

BEFORE you do anything else. Do this for every single file you touch. Do not batch
it. Do not skip it because "the edit looked small." Do not skip it because "I already
know rustfmt style." YOU DO NOT. RUN IT.

### Why This Is Non-Negotiable

1. **You will get indentation wrong.** Rust uses 4 spaces, no tabs. Python-trained
   models frequently emit 2-space or tab-indented Rust. `cargo fmt` fixes this
   automatically.
2. **You will get brace and parenthesis spacing wrong.** `rustfmt` has specific rules
   for spacing inside `fn(...)`, `match { ... }`, struct literals, etc. that differ
   from Python/JS conventions. You cannot reproduce these by hand reliably.
3. **You will get import ordering wrong.** `rustfmt` reorders `use` statements.
   Hand-ordering them will fail `cargo fmt --check`.
4. **`cargo fmt --check` is enforced.** A file that does not pass `cargo fmt --check`
   is a broken file, period. It does not matter if the code compiles.
5. **Broken formatting wastes review cycles.** Every time you submit a Rust file that
   fails `cargo fmt --check`, a human or CI must fix your mistake. This is
   unacceptable.

### Enforcement Procedure

For every Rust file you edit, the workflow is:

```
edit/create the .rs file  →  run `cargo fmt`  →  verify `cargo fmt --check` passes
```

If you edit three Rust files, you run `cargo fmt` three times (or at minimum once at
the end before you finish). NEVER mark a task complete if any edited `.rs` file has
not been formatted with `cargo fmt`.

**If you are unsure whether a file is formatted correctly, run `cargo fmt`. It is
always safe — `rustfmt` is idempotent and will only fix formatting, never change
semantics.**

### What This Replaces

Do NOT attempt to format Rust code by hand to "match existing style." Do NOT copy
indentation from a Python file. Do NOT guess at `rustfmt` rules. The ONLY reliable
source of truth for Rust formatting is the `cargo fmt` command itself. Use it.

## Technology Stack

- **Language**: Rust edition 2024 or greater. Verified: the root `Cargo.toml` and
  all crates declare `edition = "2024"` (workspace-inherited), which requires
  Rust 1.85+.

## Build Commands

Use the `Bash` tool to run the following cargo commands:

- `cargo build` — Build debug binary; allow up to 1500 seconds.
- `cargo build --release` — Build optimized release binary; allow up to 1500 seconds.
- `cargo check` — Check code without building.
- Build only debug builds unless specifically asked to perform a `release build`.

Builds can take a long time, so allow up to 1500 seconds for a rebuild.

## Test Commands

Use the Bash tool to run the following `cargo` commands.

Test running is very very expensive, it can take 40 minutes to run a full test suite, you have to strive to minimize the number of times a test suite is run, if you need to run a full test suite, make sure you capture the output in a file and use that file to run any subsequent passes that require reinterpretation of the output instead of rerunning the test suite.

**The workspace runs tests with `cargo nextest`, not `cargo test`.** Every test
runs in its own process scheduled across the machine, so a wedged test cannot
block its neighbours and a process-global mutation cannot leak between two tests.
The configuration lives in `.config/nextest.toml`.

- `cargo nextest run` — Run every test in the workspace
- `cargo nextest run -p <crate>` — Run one crate
- `cargo nextest run -p <crate> -E 'test(<name>)'` — Run tests matching a filter;
  `-E 'test(test_foo::)'` isolates one `tests/test_foo.rs` module, and
  `-E 'test(a) + test(b)'` unions filters
- `cargo nextest run --workspace --no-fail-fast` — The CI shape: every failure in
  one pass
- `cargo nextest list` — List what would run, without running it
- `cargo nextest run <filter>` — A bare trailing argument is a substring filter
  over test names; prefer `-E 'test(...)'`, which is an exact expression
- `cargo nextest run --success-output immediate` — Show output for passing tests
  too (`failure-output` is already `immediate-final`)
- `cargo nextest run -j 1` — Sequential execution, for diagnosing an interleaving
- `cargo test --doc` — The one remaining `cargo test` form: **nextest does not run
  doc tests**
- **Test Timeout**: All test runs should have a 1500 second timeout to prevent hanging
  - Use `timeout 1500 cargo nextest run` on Unix/Linux
  - `.config/nextest.toml` additionally kills any single test that runs longer
    than 150 s, so a hang is reported as a failed test rather than a stalled run


### Test Organization

All tests **MUST** be located in the `tests/` directory inside each crate. If a test is at the workspace root, it should be placed in the root `tests/` folder, not inline in source files.

- Use `#[test]` for sync tests and `#[tokio::test]` for async tests.
- Import from the relevant public crate for the crate under test rather than assuming a single `ragent` crate path.
- Organize related tests together.
- Follow the naming convention: `test_<component>_<scenario>` (e.g. `test_jog_x_positive`).
- For each project crate, migrate related tests into suitable subfolders within that crate's `tests/` directory. Review both inline and external tests for migration candidates, and relocate inline tests from `.rs` files into separate files under the appropriate `tests/` subfolder where practical.
- **Migration strategies** (see `docs/reports/testconsolidation-review.md` for the migration review and plan):
  - **Public-API tests**: use `use ragent_<crate>::module::Item;` — no source changes needed.
  - **Private-item tests**: widen the tested items to `pub(crate)` where necessary and re-import the source module via `#[path = "../src/<module>.rs"] mod <module>;`. Provide shims for `super::` and `crate::` references at the test file root.
  - **Complex cases** (`//!` doc comments + `crate::` cross-module deps): use `#[cfg(test)] #[path = "../../tests/test_<module>.rs"] mod test_<module>;` in the source file to compile the external test within the crate's module tree.
- **Do not add new inline `#[cfg(test)]` modules** to library source files. All new tests go in `tests/`.

#### One file per test, one binary per crate (consolidated targets)

Keep **one file per unit of test** (descriptive `test_*.rs` names, as above), but
**one test binary per crate**. Do not create a new auto-discovered top-level
`[[test]]` target for every file: `cargo`'s default autotest discovery makes each
top-level `tests/*.rs` file its own binary, and at this workspace's scale (900+
files) that is 900+ link steps and 900+ separate test binaries. This is **not**
the layout to grow further.

Instead, each crate exposes a small, fixed number of integration targets, and
every `tests/test_*.rs` file is included as a child module of that target's root:

```toml
# crates/<crate>/Cargo.toml
[package]
autotests = false          # stop auto-discovering one target per file

[[test]]
name = "all"
path = "tests/all.rs"
```

```rust
// crates/<crate>/tests/all.rs
#[path = "test_activity_event.rs"] mod test_activity_event;
#[path = "test_automation_run.rs"] mod test_automation_run;
// ... one `#[path = ...] mod ...;` line per former top-level test file
```

Rules for this model:

- **One target per crate** (two or three grouped by feature gate only when a
  `cfg(feature = ...)` build genuinely requires a different binary). New test
  files are added as one more `#[path = "test_x.rs"] mod test_x;` line in
  `tests/all.rs` — never as a new top-level file that becomes its own binary.
- **Use `#[path]` inclusion, not `mod` relocation.** `#[path]` anchors module
  resolution at the child file, so an existing test file keeps working unchanged:
  plain `mod support;`, `#[path = "../src/<module>.rs"]` source re-imports,
  `include_str!("support/...")`, and inner attributes
  (`#![forbid(...)]`, `#![allow(...)]`, `#![cfg(feature = ...)]`) on a child
  module are all preserved.
- **Test names keep their file prefix.** A test defined in `test_foo.rs` is named
  `test_foo::<fn>` inside the `all` binary.
- **Run a single file by filter, not by `--test`.** `cargo test -p <crate> --test test_foo` no longer exists; use the filter form
  `cargo nextest run -p <crate> -E 'test(test_foo::)'` (the `test_foo::` module
  prefix isolates that file). The same filter works under `cargo test` as
  `cargo test -p <crate> --test all test_foo::`, which is the form `cargo test`
  still needs when a doc test or an in-process run is what you want.
- **Merging binaries merges process state.** Under `cargo test` a child module
  runs in the `all` binary's single process alongside every other child, so
  process-global resources (env vars, cwd, `static` registries) that were
  previously isolated between test files now race. `cargo nextest` runs every
  test in its own process and so removes most of that, but it is not a licence to
  share a mutable global: a `static` registry or a shared scratch directory is
  still one object per binary, and tests within one crate can still interleave
  over a filesystem path. When two files touch the same global, serialise the
  mutating tests with `#[serial]` (`serial_test`) or the crate's
  `global_env_lock()` — never by weakening an assertion or by splitting into more
  targets just to regain isolation. Prefer giving a test its own directory (a
  per-test temp dir or a `target/temp/<unique-name>` path) over serialising.
- **`#[serial]` orders one process; nextest runs one process per test.** Where a
  test's exclusion must hold across *processes* — a fixture written into the
  repository, a shared `target/temp/` path, a scan of the checked-in tree — a
  `Mutex` or `#[serial]` in the test file does not provide it: serial_test
  publishes its lock key as `SERIAL_TEST_RETRY_KEY` in the parent environment
  and each nextest test process starts from its own saved environment, so two
  siblings take different keys and run together. Define the exclusion in
  `.config/nextest.toml` instead: a `[test-groups]` table with
  `max-threads = 1` plus a `[[profile.default.overrides]]` (and a `profile.ci`
  twin) whose `filter` selects the tests. `test_tool_rename_guard` is the worked
  example (`tool-rename-guard` group). A file-local `Mutex` is still correct for
  an in-process run (`cargo test`), which is why it stays alongside the group.
- **Do not weaken an assertion to make a consolidated build pass.** A cross-file
  failure is a real test-isolation bug; mark `#[serial]` or fix the shared state.
- **`crate::` in a `#[path]`-included source module resolves at the target root.**
  A test file that does `#[path = "../src/<module>.rs"] mod <module>;` and whose
  module body refers to `crate::<x>` worked when the file was a binary root, but
  as a child module of `all` those paths resolve against `all`'s root, not the
  crate under test. Provide the shim (`pub use ragent_<crate>::<x>;`) at the top
  of `all.rs` — a `tests/all_prelude.rs` inlined there is the convention — or
  rewrite the reference to the crate-absolute path (`ragent_<crate>::<x>`).
- **Expect a narrow `#[allow(clippy::duplicate_mod)]` on `all.rs`.** When two
  child files each declare the same `#[path = "support/mod.rs"] mod support;`,
  the shared helper is loaded as a module of several children; clippy's
  `duplicate_mod` flags it. Hoisting would mean editing every child, so `all.rs`
  carries the allow (clippy's `duplicate_mod` is the only such case; all other
  `#![...]` inner attributes stay on the child files).
- **Prefer a `check-*` guard update, not a new target, for CI.** Convert
  `cargo test -p <crate> --test <file>` to
  `cargo nextest run -p <crate> -E 'test(<file>::)'` — the `::` suffix keeps the
  filter anchored to that module.

## Lint & Format Commands

Use the `Bash` tool to run the following cargo commands:

- `cargo clippy` — Run linter with clippy
- `cargo fmt` — Format code with rustfmt. **MANDATORY** to run after editing any `.rs` file. See the top of this file for the full procedure.
- `cargo fmt --check` — Check formatting without changes. Fails if any file is unformatted. This is the CI gate.

## Workspace Layout

- Keep all crates under the `crates/` directory.
- Place each crate in its own subfolder under `crates/`, named after the crate (e.g. `crates/ragent-core`).
- Keep the root `Cargo.toml` configured as a workspace manifest. It may also
  host the primary `ragent` binary package (`[package] name = "ragent"`,
  `[dependencies]`, `[[bin]]`, `[features]`, `[profile.release]`) alongside the
  `[workspace]` block — a standard single-binary monorepo pattern. Do not
  declare package metadata or dependencies in the root `Cargo.toml` *for
  library crates*; list workspace members instead:

  ```toml
  [workspace]
  members = ["crates/*"]
  resolver = "2"
  ```
- Define crate-specific metadata, dependencies, and targets in each crate's own `Cargo.toml`.
- Prefer small, focused crates with a single responsibility.

## Code Style Guidelines

- **Formatting**: 4 spaces, max width 100, reorder imports automatically, Unix newlines.
- **Naming**: snake_case for functions/variables, PascalCase for types/structs/enums.
- **Imports**: Group std, external crates, then local modules.
- **Error Handling**: Use `Result<T, E>` with `?`, `anyhow::Result` for main, and `thiserror` for custom errors.
- **Types**: Prefer explicit types, and use type aliases for complex types.
- **Logging**: Use the `tracing` crate with structured logging; avoid `println!` or `eprintln!` in application code. For performance profiling, use `debug!()` for non-hot paths and `trace!()` for debug scenarios. **Documented exception:** the root `ragent` binary's *CLI presentation surface* is allowed to use `println!`/`eprintln!` for user-facing command output (help text, tables, JSON reports, and the interactive TUI's stdout) and for panic/abort diagnostics that must be emitted before the tracing subscriber is installed (`src/panic_hook.rs`, `src/stderr_spool.rs`, the crash-marker warnings, and the `--log-level` parse fallback in `src/main.rs`). The CLI presentation surface comprises `src/main.rs`, `src/cli.rs`, `src/cli_surface.rs`, and the slash-command parity surfaces `src/connectors.rs`, `src/plugins.rs` and `src/acp_server.rs` (their `[err]`/usage/table output is the CLI contract, not a diagnostic). Any diagnostic emitted *after* tracing is initialised must use `tracing` instead (ANTIPAT M2.16 R-05).
- **Logging Cleanliness**: After an issue has been resolved, remove temporary `debug!()` and `tracing::debug!()` calls in the relevant code.
- **Documentation**: Use `//!` for crate/module docs, `///` for public APIs, and `//` for internal comments.
- **Linting**: No wildcard imports in production code. This rule does not apply
  to `use super::*;` inside `#[cfg(test)]` modules or `pub use foo::*` re-export
  globs (both are idiomatic and allowed). Treat cognitive complexity ≤30 and
  missing docs warnings as review targets rather than guaranteed
  compiler-enforced limits.
- **Best Practices**: Read the best practices at https://www.djamware.com/post/68b2c7c451ce620c6f5efc56/rust-project-structure-and-best-practices-for-clean-scalable-code and apply them to the project where relevant.

### Core Code Rules

* **No `unsafe` Blocks:** Never use `unsafe` unless explicitly approved or required by FFI bounds; document all safety invariants clearly when used.
  Every crate root (`crates/*/src/lib.rs`) and the `ragent` binary root (`src/main.rs`)
  carries a crate-level `#![deny(unsafe_code)]`, so a new `unsafe` block fails to
  compile until it is explicitly re-enabled. The approved production sites are
  gated with a scoped `#[allow(unsafe_code)]` plus a `// SAFETY:` comment stating
  the invariant, and each exists only because libc has no safe std equivalent:

  | Site | Why |
  | --- | --- |
  | `crates/ragent-tools-core/src/bash.rs` (`kill_process_group`) | `libc::killpg` - bash-timeout process-group kill |
  | `crates/ragent-agent/src/team/manager.rs` (`kill_hook_process_group`) | `libc::killpg`/`kill` - hook timeout teardown |
  | `crates/ragent-agent/src/mcp/mod.rs` (`kill_stdio_child`) | `libc::kill`/`killpg` - MCP stdio child teardown (3 blocks) |
  | `src/stderr_spool.rs` (`install`) | `libc::pipe`/`dup`/`dup2`/`close` and `File::from_raw_fd` - stderr spool capture (7 blocks) |
  | `src/crash_dump.rs` (`process_alive`) | `libc::kill` signal 0 liveness probe, `OpenProcess`/`GetExitCodeProcess`/`CloseHandle` on Windows (2 blocks) |

  Tests may opt back in with a file-level `#![allow(unsafe_code)]` for the Rust 2024
  `std::env::set_var`/`remove_var` migration; that is the only sanctioned test use.
* **Exhaustive Pattern Matching:** Always match enums and results exhaustively; avoid wildcard `_` fallbacks unless handling non-exhaustive external types.
* **Explicit Cloning:** Call `.clone()` explicitly on non-`Copy` types; forbid hidden or cascading clones inside intensive iterators or closures.
* **No Silent Error Swallowing:** Never use `.unwrap()` or `.expect()` in production code paths on user-facing paths; always propagate or handle `Result` and `Option`. Test and bench code may use them freely. The workspace clippy config sets `unwrap_used = "allow"` so the compiler does not enforce this; treat it as a review-enforced rule and keep the T-006 sites fixed.

### Code Style & Formatting

* **Naming Conventions:** Use `snake_case` for variables, functions, and modules; `PascalCase` for types, traits, and enums; `SCREAMING_SNAKE_CASE` for global constants.
* **Idiomatic Constructs:** Prefer expressive iterator adapters (`map`, `filter`, `fold`) over explicit manual index-based loops.
* **Formatting standard:** Adhere strictly to default `rustfmt` layouts and keep maximum line lengths under 100 characters.
* **No Unicode/Emojis:** Exclude emojis and non-ASCII characters from comments and identifiers. ASCII tree-drawing glyphs in user-facing terminal output are allowed where they improve readability (e.g. TUI, agent-tree rendering, config dump borders); non-ASCII box-drawing characters should be avoided in favour of plain ASCII (`-`, `|`, `+`) where practical.

### Workflow & Tool Enforcement

* **Zero Warnings Policy:** Generated code must compile cleanly without warnings under standard compiler checks.
* **Clippy Compliance:** Run and pass `cargo clippy` with standard lints on all proposed changes.
* **Automated Formatting Check (MANDATORY):** You MUST run `cargo fmt` on every Rust file you edit before finishing. See the "MANDATORY: Run `cargo fmt`" section at the top of this file for the full procedure. `cargo fmt` is idempotent and safe — run it freely. A task is NOT complete if any edited `.rs` file has not been formatted with `cargo fmt`. This is enforced by `cargo fmt --check` in CI; unformatted files are build failures.

## Idiomatic Rust Practices

Follow idiomatic Rust practices and community standards. This section
supplements Code Style Guidelines above; on any conflict, the earlier, more
specific section (Code Style Guidelines / Core Code Rules) wins. These
instructions are based on
[The Rust Book](https://doc.rust-lang.org/book/), the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/),
[RFC 430 naming conventions](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md),
and the broader Rust community at [users.rust-lang.org](https://users.rust-lang.org).

### General Instructions

- Always prioritize readability, safety, and maintainability.
- Use strong typing and leverage Rust's ownership system for memory safety.
- Break down complex functions into smaller, more manageable functions.
- For algorithm-related code, include explanations of the approach used.
- Write code with good maintainability practices, including comments on why certain design decisions were made.
- Handle errors gracefully using `Result<T, E>` and provide meaningful error messages.
- For external dependencies, mention their usage and purpose in documentation.
- Use consistent naming conventions following [RFC 430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md).
- Write idiomatic, safe, and efficient Rust code that follows the borrow checker's rules.
- Ensure code compiles without warnings.

### Patterns to Follow

- Use modules (`mod`) and public interfaces (`pub`) to encapsulate logic.
- Handle errors properly using `?`, `match`, or `if let`.
- Use `serde` for serialization and `thiserror` or `anyhow` for custom errors.
- Implement traits to abstract services or external dependencies.
- Structure async code using `async/await` on `tokio` (the async runtime used across this workspace).
- Prefer enums over flags and states for type safety.
- Use builders for complex object creation.
- Split binary and library code (`main.rs` vs `lib.rs`) for testability and reuse.
- Use `rayon` for data parallelism and CPU-bound tasks.
- Use iterators instead of index-based loops as they're often faster and safer.
- Use `&str` instead of `String` for function parameters when you don't need ownership.
- Prefer borrowing and zero-copy operations to avoid unnecessary allocations.

#### Ownership, Borrowing, and Lifetimes

- Prefer borrowing (`&T`) over cloning unless ownership transfer is necessary.
- Use `&mut T` when you need to modify borrowed data.
- Explicitly annotate lifetimes when the compiler cannot infer them.
- Use `Rc<T>` for single-threaded reference counting and `Arc<T>` for thread-safe reference counting.
- Use `RefCell<T>` for interior mutability in single-threaded contexts and `Mutex<T>` or `RwLock<T>` for multi-threaded contexts.

### Patterns to Avoid

- Don't use `unwrap()` or `expect()` unless absolutely necessary—prefer proper error handling.
- Avoid panics in library code—return `Result` instead.
- Don't rely on global mutable state—use dependency injection or thread-safe containers.
- Avoid deeply nested logic—refactor with functions or combinators.
- Don't ignore warnings—treat them as errors during CI.
- Avoid `unsafe` unless required and fully documented.
- Don't overuse `clone()`, use borrowing instead of cloning unless ownership transfer is needed.
- Avoid premature `collect()`, keep iterators lazy until you actually need the collection.
- Avoid unnecessary allocations—prefer borrowing and zero-copy operations.

### Error Handling

- Use `Result<T, E>` for recoverable errors and `panic!` only for unrecoverable errors.
- Prefer `?` operator over `unwrap()` or `expect()` for error propagation.
- Create custom error types using `thiserror` or implement `std::error::Error`.
- Use `Option<T>` for values that may or may not exist.
- Provide meaningful error messages and context.
- Error types should be meaningful and well-behaved (implement standard traits).
- Validate function arguments and return appropriate errors for invalid input.

### API Design Guidelines

Eagerly implement common traits where appropriate:

- `Copy`, `Clone`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Debug`, `Display`, `Default`
- Use standard conversion traits: `From`, `AsRef`, `AsMut`
- Collections should implement `FromIterator` and `Extend`
- Note: `Send` and `Sync` are auto-implemented by the compiler when safe; avoid manual implementation unless using `unsafe` code

Type safety and predictability:

- Use newtypes to provide static distinctions
- Arguments should convey meaning through types; prefer specific types over generic `bool` parameters
- Use `Option<T>` appropriately for truly optional values
- Functions with a clear receiver should be methods
- Only smart pointers should implement `Deref` and `DerefMut`

Future proofing:

- Use sealed traits to protect against downstream implementations
- Structs should have private fields
- Functions should validate their arguments
- All public types must implement `Debug`

### Testing and Documentation

- Write comprehensive unit tests. In this workspace, prefer keeping tests in each crate's `tests/` directory; see the "Test Organization" section above for the migration rules.
- Write integration tests in `tests/` directory with descriptive filenames.
- Write clear and concise comments for each function, struct, enum, and complex logic.
- Ensure functions have descriptive names and include comprehensive documentation.
- Document all public APIs with rustdoc (`///` comments) following the [API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `#[doc(hidden)]` to hide implementation details from public documentation.
- Document error conditions, panic scenarios, and safety considerations.
- Examples should use `?` operator, not `unwrap()` or deprecated `try!` macro.

### Project Organization

- Use semantic versioning in `Cargo.toml`.
- Include comprehensive metadata: `description`, `license`, `repository`, `keywords`, `categories`.
- Use feature flags for optional functionality.
- Organize code into modules using named files with inline `mod` declarations
  (Rust 2018+ layout, e.g. `src/agent/custom.rs` declared via `mod custom;`
  in `src/agent/mod.rs`); reserve `mod.rs` for legacy or deeply nested modules.
- Keep `main.rs` or `lib.rs` minimal - move logic to modules.

### Quality Checklist

Before publishing or reviewing Rust code, ensure:

#### Core Requirements

- [ ] **Naming**: Follows RFC 430 naming conventions
- [ ] **Traits**: Implements `Debug`, `Clone`, `PartialEq` where appropriate
- [ ] **Error Handling**: Uses `Result<T, E>` and provides meaningful error types
- [ ] **Documentation**: All public items have rustdoc comments with examples
- [ ] **Testing**: Comprehensive test coverage including edge cases

#### Safety and Quality

- [ ] **Safety**: No unnecessary `unsafe` code, proper error handling
- [ ] **Performance**: Efficient use of iterators, minimal allocations
- [ ] **API Design**: Functions are predictable, flexible, and type-safe
- [ ] **Future Proofing**: Private fields in structs, sealed traits where appropriate
- [ ] **Tooling**: Code passes `cargo fmt`, `cargo clippy`, and `cargo nextest run`
