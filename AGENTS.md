# Agent Guidelines for Rust/GTK4 apps
- First when you startup say "Hi im Rust Agent and I have read Agents.md" 

## Technology Stack
- **Language**: Rust edition 2021 or greater
- **UI Framework**: gtk-rs

Create and maintain a file called GTK4.md that contains all knowledge you have gained about the UI Crate, and all insghts and successfull stratergies. Reffer to this file if you have issues you cannot resolve. 

## Cargo ##
- Use cargo for all rust operations, preceed the commend with a timeout 600 command to ensure that suffient time is provided for the command to complete. 

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

## Lint & Format Commands
- `cargo clippy` - Run linter with clippy
- `cargo fmt` - Format code with rustfmt
- `cargo fmt --check` - Check formatting without changes

## Units ##
- DateTime vaules should be represented internaly in UTC and translated to locale based represetations in the UI layer. 
- Dimensional units should be represented internaly in mms, and be of type f32, and mm values should be represted to 2 decimal place accuracy. 
- All text strings where feasable should be internaly represented in UTF8 encoding, with translation to and from UI encoding in the UI layer if required. 
- All dimension and feed values in the UI must support switching between Metric (mm) and Imperial (inch) units.
- All rate units (feed rate etc) must support switching between mm/sec, mm,min, in/sec or in/min. 
- The UI should listen for measurement system changes and update displayed values accordingly.
- Internal calculations and storage should always be in millimeters (mm) for dimensions and mm/min for feed rates, unless specifically required otherwise by a protocol.
- Use the `gcodekit5_core::units` module for conversion and formatting.

## Esthetics ## 
- The aversage user will be using a FHD screen set to 125% scaling. ensure that all esthetics reviews, designs and recomendations account for this. 

- **CRITICAL**: Do not push changes to remote unless specifically told to. This is a strict rule.
- Do not tag releases unless specifically told to. 

## Changelog Management
- **CHANGELOG.md**: Maintain a changelog in the root directory documenting all changes before each push to remote.
- **Format**: Follow Keep a Changelog format (https://keepachangelog.com/)
- **Update Timing**: Update CHANGELOG.md before each push to remote with the latest changes, features, fixes, and improvements.
- **Version**: Use semantic versioning (major.minor.patch-prerelease)
- **RELEASE.md**: write the version number and the most recent CHANGELOG.md entry to the RELEASE.md file for use as a Description in the Github Releases page. 


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

## Versioning

1. During development the release number will have "-alpha" appended to the end as per semantic versioning standards. only when it is a production release will it be removed. 

## Temporary Files

1. Create a directory called "target" in the project root
2. Create a directory called "temp" in the target folder
3. Ensure that the target/temp folder is in the .gitignore file
4. Use target/temp for all temporary files, scripts and other ephemeral items that are normally placed in /tmp

Use the LSP MCP Bridge tools to find code instead of performing 
