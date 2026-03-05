## [0.54.0-alpha.0] - 2026-03-05

### Added
- **Performance benchmarks** across 3 crates: core (12 benchmarks), visualizer (8 benchmarks), designer (existing 10 benchmarks)
- **Pre-commit hooks** enforcing `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --lib`
- Pre-commit hook setup instructions in CONTRIBUTING.md

### Changed
- **Memory optimizations**: `Cow<'static, str>` for G-code command counts, `SmallVec<[Point; 4]>` for rectangle corners, `Vec::with_capacity` pre-allocation, clone reduction in pipeline processing
- **Arc audit**: confirmed all Arc usage is in genuinely multi-threaded contexts
