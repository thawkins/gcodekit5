Version: 0.40.0-alpha.1

## [0.40.0-alpha.1] - 2025-12-16

### Changed
- **Version**: Bumped version to 0.40.0-alpha.1.

### Fixed
- **Build**: Fixed remote build failures by adding missing `model.rs` and `ops.rs` files to the repository.
- **Build**: Fixed unused import and variable warnings in `gcodekit5-designer` and `gcodekit5-ui` crates.

### Added
- **CI/CD**: Added GitHub Actions stages to build `.deb` and `.rpm` packages for Linux releases.
- **CAM Tools**: Implemented unit switching support (Metric/Imperial) for all CAM tools.
- **CAM Tools**: Added `create_dimension_row` helper and unit update listeners.

### Fixed
- **Designer**: Fixed issue where resizing a rotated shape would cause it to jump and increase in size.
- **Designer**: Fixed issue where resizing shapes using Top-Left or Bottom-Right handles would deselect the shape on release if the handle was off-grid.
- **Designer**: Removed redundant context menu items to declutter the interface.
- **CAM Tools**: Fixed type mismatch errors in `VectorEngravingTool` and `TabbedBoxMaker`.
- **CAM Tools**: Fixed duplicate method definitions in `VectorEngravingTool`.

## [0.36.0-alpha.0] - 2025-12-14

### Fixed
- **Help Browser**: Fixed issue where help content was selected by default.
- **Help Browser**: Changed help button icon to `info-outline-symbolic`.
- **Device Console**: Added help button linking to Machine Control help topic.

## [0.35.0-alpha.0] - 2025-12-12

### Changed
- **Version**: Bumped version to 0.35.0-alpha.0.

## [0.33.0-alpha.0] - 2025-12-12

### Added
- **Designer**: Text tool with font selection (family/bold/italic) and point-size UI.
- **Preferences/About**: “Show About on startup” option and startup auto-dismiss.

### Changed
- **Designer/Visualizer**: Standardized Fit actions and long-running progress + cancel UX.
- **Designer**: Improved Layers/Inspector UX (selection behavior, separator resizing, group/ungroup wiring).
- **Version**: Bumped version to 0.33.0-alpha.0.

## [0.30.0-alpha.0] - 2025-12-12

### Added
- **GRBL**: Decode `WPos` (working coordinates) and update the working DRO.
- **Visualizer**: Sidebar hide/show UX, grouped sections, legend, empty states.
- **Designer**: Grid spacing + snap controls, toolbox active-tool chip, empty states, async preview generation w/ cancel.

### Changed
- **Visualizer/Designer**: Standardized OSD formatting and aligned draw colors with theme palette.
- **Version**: Bumped version to 0.30.0-alpha.0.

## [0.29.0-alpha.0] - 2025-12-11

### Changed
- **Visualizer**: Removed debug println! statements to clean up console output.
- **Version**: Bumped version to 0.29.0-alpha.0.
