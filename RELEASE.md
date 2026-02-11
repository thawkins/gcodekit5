## [0.50.2-alpha.0] - 2026-01-30

### Added
- **Dependabot Configuration**: Automated dependency update management
  - Created `.github/dependabot.yml` for Cargo, GitHub Actions, and npm
  - Weekly schedule (Monday 09:00 UTC) for dependency checks
  - Groups minor/patch updates to reduce PR noise
  - Proper labeling for easy PR filtering
  - REMEDIATION_PLAN.md Task 7.2.1 completed

- **Monthly Dependency Review Process**: Documented dependency maintenance workflow
  - Created `scripts/monthly-dependency-review.sh` for automated review
  - Created `docs/dependency_management.md` with full process documentation
  - Includes security vulnerability response guidelines
  - REMEDIATION_PLAN.md Task 7.2.2 completed

- **MSRV (Minimum Supported Rust Version)**: Set and enforced Rust 1.88
  - Added `rust-version = "1.88"` to workspace Cargo.toml
  - All 9 crates inherit MSRV from workspace
  - Created `.github/workflows/msrv.yml` CI workflow
  - REMEDIATION_PLAN.md Task 7.3.1 completed

- **Event Bus System Design**: Architecture design for unified event handling
  - Created `docs/adr/ADR-006-event-bus-system.md` with comprehensive design
  - 7 event categories with typed events and compile-time safety
  - EventBus API with publish/subscribe, filters, and optional history
  - REMEDIATION_PLAN.md Task 6.1.1 completed

- **Core Event Bus Implementation**: Unified event system in gcodekit5-core
  - EventBus with publish/subscribe, filters, optional history
  - Global singleton, convenience macros, async support
  - 12 unit tests covering all functionality
  - REMEDIATION_PLAN.md Task 6.1.2 completed

- **File Format Documentation**: Documented .gck4 design file format
  - Created `docs/file_format.md` with complete format specification
  - Added 5 additional round-trip tests
  - REMEDIATION_PLAN.md Task 2.4.4 completed

- **Type Aliases for Complex Types**: Improved code readability
  - Created `types/aliases.rs` with 22 type aliases + 11 helper functions
  - Covers Rc<RefCell>, Arc<Mutex>, Arc<RwLock>, callbacks
  - REMEDIATION_PLAN.md Task 3.1.1 completed

- **Box<dyn T> Audit and Type Aliases**: Comprehensive trait object analysis
  - Audited 47 `Box<dyn>` usages across 4 crates
  - Added `BoxedIterator<T>`, `BoxedError`, `BoxedResult<T>` type aliases
  - All patterns documented with justifications for dynamic dispatch
  - REMEDIATION_PLAN.md Task 3.1.2 completed

- **Core Crate API Documentation**: 100% documentation coverage
  - Documented 264 public items across 18 source files
  - `cargo doc` builds with 0 warnings
  - REMEDIATION_PLAN.md Task 3.2.1 completed

- **Unwrap Audit Documentation**: Comprehensive audit of all unwrap() calls
  - Created `docs/audits/unwrap_audit.csv` with 585 categorized unwraps
  - Created `docs/audits/UNWRAP_AUDIT_REPORT.md` with executive summary
  - 144 high-risk, 158 medium-risk, 283 low-risk unwraps identified
  - Priority remediation targets: Mutex locks, RefCell borrows, File I/O
  - REMEDIATION_PLAN.md Task 1.1.1 completed

- **CI Code Quality Checks**: Prevent unwrap() regression
  - Created `.github/workflows/code-quality.yml` with clippy unwrap detection
  - Created `.github/PULL_REQUEST_TEMPLATE.md` with error handling checklist
  - REMEDIATION_PLAN.md Task 1.1.5 completed

- **Structured Error Types**: Added thiserror-based error types to 3 crates
  - `gcodekit5-designer/src/error.rs`: DesignError, GeometryError, ToolpathError
  - `gcodekit5-communication/src/error.rs`: CommunicationError, ProtocolError, FirmwareError
  - `gcodekit5-visualizer/src/error.rs`: VisualizationError, ParsingError, FileError
  - REMEDIATION_PLAN.md Task 1.2.1 completed

- **GitHub Issues for TODOs**: Converted all 20 TODOs to tracked issues (#12-#19)
  - REMEDIATION_PLAN.md Task 2.4.1 completed

- **Pre-commit Hook**: Added `.githooks/pre-commit` for code quality checks
  - REMEDIATION_PLAN.md Task 9.1.1 completed

### Changed
- **Error Handling**: Removed ALL 585 unsafe unwrap() calls from production code
- **Test Quality**: Replaced all 235 test unwrap() calls with expect()
- **Code Structure**: Extracted DesignerCanvas to separate module
- **Code Cleanup**: Replaced debug eprintln/println with structured tracing
