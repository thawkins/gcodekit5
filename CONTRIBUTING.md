# Contributing to GCodeKit5

Thank you for your interest in contributing to GCodeKit5! This document provides guidelines and information to help you contribute effectively.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Code Style](#code-style)
- [Branch Naming](#branch-naming)
- [Commit Messages](#commit-messages)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Documentation](#documentation)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

Be respectful, inclusive, and constructive. We welcome contributors of all backgrounds and experience levels.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally
3. **Set up** the development environment (see [DEVELOPMENT.md](DEVELOPMENT.md))
4. **Create a branch** for your changes
5. **Make changes** following the guidelines below
6. **Submit** a pull request

## Code Style

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt
```

Key formatting rules:
- **Indentation**: 4 spaces (no tabs)
- **Line width**: Maximum 100 characters
- **Imports**: Automatically reordered
- **Newlines**: Unix-style (LF)

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Functions | `snake_case` | `parse_gcode()` |
| Variables | `snake_case` | `line_count` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_BUFFER_SIZE` |
| Types/Structs | `PascalCase` | `MachineState` |
| Enums | `PascalCase` | `ConnectionStatus` |
| Enum variants | `PascalCase` | `ConnectionStatus::Connected` |
| Modules | `snake_case` | `gcode_parser` |
| Traits | `PascalCase` | `Serializable` |

### Linting

All code must pass Clippy without warnings:

```bash
cargo clippy
```

Key linting rules:
- No `unwrap()` calls without justification (use `expect()` with explanation or proper error handling)
- No wildcard imports (`use module::*`)
- Cognitive complexity ≤ 30 per function
- No `println!()` or `eprintln!()` in library code (use `tracing` instead)

### Example: Good vs Bad

```rust
// ❌ Bad
fn process(data: Option<String>) {
    let value = data.unwrap();  // Will panic!
    println!("Processing: {}", value);
}

// ✅ Good
fn process(data: Option<String>) -> Result<(), ProcessError> {
    let value = data.ok_or(ProcessError::MissingData)?;
    tracing::debug!("Processing: {}", value);
    Ok(())
}
```

## Branch Naming

Use descriptive branch names with prefixes:

| Prefix | Use Case | Example |
|--------|----------|---------|
| `feature/` | New features | `feature/add-laser-mode` |
| `fix/` | Bug fixes | `fix/serial-timeout` |
| `refactor/` | Code refactoring | `refactor/visualizer-cache` |
| `docs/` | Documentation only | `docs/update-readme` |
| `test/` | Test additions/fixes | `test/grbl-parser-coverage` |
| `chore/` | Build, CI, dependencies | `chore/update-gtk4-version` |

### Examples

```bash
# Good branch names
git checkout -b feature/tool-library-import
git checkout -b fix/dro-display-precision
git checkout -b docs/api-documentation

# Bad branch names (avoid)
git checkout -b my-changes      # Not descriptive
git checkout -b fix             # Too vague
git checkout -b WIP             # Not meaningful
```

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `style` | Formatting, no code change |
| `refactor` | Code change that neither fixes nor adds |
| `perf` | Performance improvement |
| `test` | Adding or fixing tests |
| `chore` | Build, CI, dependencies |

### Scope (optional)

The crate or component affected: `core`, `ui`, `communication`, `visualizer`, `designer`, `camtools`, etc.

### Examples

```bash
# Good commit messages
git commit -m "feat(designer): add shape rotation support"
git commit -m "fix(communication): handle serial timeout correctly"
git commit -m "docs: update installation instructions for macOS"
git commit -m "refactor(visualizer): extract grid rendering to module"
git commit -m "test(core): add unit tests for gcode parser"

# With body for complex changes
git commit -m "fix(ui): prevent crash when closing settings dialog

The dialog was being destroyed before the save operation completed.
Added proper async handling and lifecycle management.

Fixes #123"
```

### Bad Examples (avoid)

```bash
git commit -m "fixed stuff"           # Not descriptive
git commit -m "WIP"                   # Not meaningful
git commit -m "asdfasdf"              # Meaningless
git commit -m "feat: everything"      # Too broad
```

## Pull Request Process

### Dependabot PRs

This repository uses [Dependabot](https://docs.github.com/en/code-security/dependabot) for automated dependency updates. When reviewing Dependabot PRs:

1. **Minor/Patch updates**: Generally safe to merge after CI passes
2. **Major updates**: Review changelog for breaking changes before merging
3. **Security updates**: Prioritize and merge promptly after CI passes
4. Dependabot PRs are labeled with `dependencies` for easy filtering

### Before Submitting

1. **Rebase** on latest `main`:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

2. **Run all checks**:
   ```bash
   cargo fmt --check
   cargo clippy
   cargo test
   ```

3. **Update CHANGELOG.md** with your changes

4. **Self-review** your code for:
   - Unnecessary debug code
   - Commented-out code
   - TODO comments that should be issues
   - Missing error handling

### PR Checklist

Your PR will use the [PR template](.github/pull_request_template.md). Ensure:

- [ ] No new `unwrap()` calls (unless justified with comment)
- [ ] `cargo fmt` passed
- [ ] `cargo clippy` has no new warnings
- [ ] Tests added for new functionality
- [ ] Public APIs documented with `///`
- [ ] Error cases handled (no silent failures)
- [ ] No debug `eprintln!()` or `println!()` statements
- [ ] Changelog entry added

### Review Process

1. **Submit PR** with clear description
2. **Automated checks** run (CI must pass)
3. **Reviewer assigned** (usually within 24 hours)
4. **Address feedback** with additional commits or amendments
5. **Approval** from at least one maintainer
6. **Merge** by maintainer (squash or rebase)

### Tips for Faster Reviews

- Keep PRs focused and small (< 400 lines ideal)
- One logical change per PR
- Include screenshots for UI changes
- Link related issues
- Respond to feedback promptly

## Testing Requirements

### New Features

All new features must include tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // Arrange
        let input = create_test_input();
        
        // Act
        let result = new_feature(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
}
```

### Bug Fixes

Bug fixes should include a regression test:

```rust
#[test]
fn test_issue_123_serial_timeout() {
    // This test verifies the fix for issue #123
    // Previously, the connection would hang indefinitely
    let result = connect_with_timeout(Duration::from_secs(1));
    assert!(result.is_err()); // Should timeout, not hang
}
```

### Test Organization

```
crates/gcodekit5-core/
├── src/
│   └── lib.rs
└── tests/           # Integration tests
    └── gcode_tests.rs

// Unit tests go in the same file as the code
// Integration tests go in tests/ directory
```

### Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p gcodekit5-core

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name
```

## Documentation

### Code Documentation

All public APIs must be documented:

```rust
/// Parses a G-code line into structured commands.
///
/// # Arguments
///
/// * `line` - A string slice containing the G-code line
///
/// # Returns
///
/// Returns `Ok(GCodeLine)` on successful parse, or `Err(ParseError)` if
/// the line contains invalid syntax.
///
/// # Examples
///
/// ```
/// use gcodekit5_core::parse_line;
///
/// let line = parse_line("G1 X10 Y20 F1000")?;
/// assert_eq!(line.command, "G1");
/// ```
pub fn parse_line(line: &str) -> Result<GCodeLine, ParseError> {
    // ...
}
```

### Module Documentation

Each module should have a top-level doc comment:

```rust
//! # G-Code Parser Module
//!
//! This module provides parsing functionality for G-code files.
//!
//! ## Overview
//!
//! The parser handles standard G-code as well as extensions for
//! GRBL, TinyG, and other firmware variants.
//!
//! ## Usage
//!
//! ```
//! use gcodekit5_core::parser;
//! let result = parser::parse_file("program.nc")?;
//! ```
```

### When to Update Documentation

- **README.md**: User-facing feature changes
- **CHANGELOG.md**: All changes (required for every PR)
- **API docs**: Public interface changes
- **DEVELOPMENT.md**: Build/setup changes
- **ADRs**: Significant architectural decisions

## Issue Reporting

### Bug Reports

Include:
1. **GCodeKit5 version** and **OS**
2. **Steps to reproduce**
3. **Expected behavior**
4. **Actual behavior**
5. **Logs** (with `RUST_LOG=debug`)
6. **Screenshots** (for UI issues)

### Feature Requests

Include:
1. **Problem** you're trying to solve
2. **Proposed solution**
3. **Alternatives** considered
4. **Use cases**

### Labels

| Label | Meaning |
|-------|---------|
| `bug` | Something isn't working |
| `enhancement` | New feature request |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention needed |
| `documentation` | Documentation improvements |
| `P0`, `P1`, `P2`, `P3` | Priority levels |

## Questions?

- Open a [Discussion](https://github.com/thawkins/gcodekit5/discussions)
- Check existing [Issues](https://github.com/thawkins/gcodekit5/issues)
- Read the [Documentation](docs/)

Thank you for contributing! 🎉
