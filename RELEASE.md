# 0.2.5-alpha.4

### Fixed
- **Visualizer**: Fixed "Fit to Device" functionality to correctly center the device working area in the view.
- **Designer**: Aligned "Fit to Device" logic with Visualizer to use consistent padding (5%) and default bounds.
- **Designer**: Fixed Pan tool jumping on drag start.
- **Designer**: Fixed padding discrepancy between Designer and Visualizer by syncing widget size to backend viewport.
- **Designer**: Fixed Origin markers to extend to world extent (Red/Green).

### Added
- **Designer**: Added Device Bounds rendering (Blue, 2px wide) to visualize the working area.
- **Designer/Visualizer**: Ensured both views initialize to "Fit to Device" on startup.

### Changed
- **Refactor**: Major refactor of Designer and Visualizer integration to improve coordinate handling and rendering performance.
- **Designer**: Updated `DesignerCanvas` and `Viewport` to use improved coordinate transformation logic.
- **UI**: Updated GTK UI components (`designer.rs`, `visualizer.rs`, `main_window.rs`) to align with the new backend architecture.
- **Visualizer**: Enhanced 2D visualizer logic and coordinate transforms in `crates/gcodekit5-visualizer`.
- **Cleanup**: Removed unused helper functions in `src/app/helpers.rs` and refactored `src/app/mod.rs`.
