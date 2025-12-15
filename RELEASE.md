Version: 0.36.1-alpha.0

## [0.36.1-alpha.0] - 2025-12-14

### Added
- **CAM Tools**: Implemented unit switching support (Metric/Imperial) for all CAM tools.
- **CAM Tools**: Added `create_dimension_row` helper and unit update listeners.

### Fixed
- **Designer**: Fixed issue where resizing shapes using Top-Left or Bottom-Right handles would deselect the shape on release if the handle was off-grid.
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
