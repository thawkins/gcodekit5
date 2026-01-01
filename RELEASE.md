Version: 0.45.0-alpha.8

## [0.45.0-alpha.8] - 2026-01-01

### Changed
- **CI/CD**: Updated macOS x86_64 runner from `macos-13` to `macos-15-large` for newer Intel runner

## [0.42.0-alpha.1] - TBD

### Changed  
- **Designer**: Right-click context menu now appears whenever there are selected shapes, regardless of click location.
- **Designer**: Simplified right-click logic to avoid complex coordinate and geometry calculations.

### Fixed
- **Designer**: Fixed right-click context menu reliability issues with multi-selections and groups.
- **Designer**: Fixed height/width property editing incorrectly changing x/y position during typing.
- **Designer**: Entry widgets now use `connect_activate` and focus-out handlers to prevent intermediate keystroke updates.

### Added
- **Designer**: Implemented non-destructive geometry operations (Offset, Fillet, Chamfer).
- **Designer**: Added `offset`, `fillet`, and `chamfer` properties to `DrawingObject`.
- **Designer**: Added `get_effective_shape()` to compute modified geometry on-the-fly for rendering, selection, and G-code.

### Changed
- **UI**: Removed "Apply" buttons from Geometry Operations in the Properties Panel.
- **UI**: Geometry operations now apply immediately on Enter or Focus Out.
- **UI**: Geometry Operations frame is now context-sensitive (hidden for Text shapes).
- **Designer**: Updated spatial indexing and hit-testing to use the effective shape bounds.

## [Unreleased]
### Fixed
- **Designer**: Fixed issue where changing height or width of shapes in the inspector would incorrectly alter the position.
  - For multi-selection: The bounding box center now remains fixed when resizing multiple shapes.
  - For single selection: Width and height changes now preserve the current position directly from the shape data instead of reading from UI text fields.
  - Width and height changes now only apply when the user finishes editing (Enter key or Tab to next field) instead of on every keystroke, preventing intermediate values from causing position shifts.
- **Designer**: Fixed UI formatting issue where position values would appear to change after editing size properties due to text field reformatting from user input to formatted output.

## [0.40.0-alpha.5] - 2025-12-19

### Changed
- **Version**: Bumped version to 0.40.0-alpha.5.

### Added
- **Device Manager**: Added device type selection (CNC, Laser, Other) to Device Config tab, automatically configuring `$32` (Laser Mode) and device capabilities.
- **Machine Control**: Added feedrate and spindle speed indicators to the central panel and status bar.
- **Machine Control**: Added feedrate and spindle speed override controls with console logging.
- **Machine Control**: Added command history navigation (Up/Down arrows) to the device console input.
- **Machine Control**: Added logging of manual commands, jogs, and overrides to the device console.

### Fixed
- **Device Manager**: Fixed `$32` (Laser Mode) display not updating when device type changes.
- **Machine Control**: Fixed feedrate and spindle speed override button logic and values.
- **DRO**: Fixed slow WPos updates and spindle position indicator lag in Visualizer.
- **UI**: Fixed GTK dialog warnings regarding transient parents and sizing.
- **UI**: Cleaned up Device Console UI, removing redundant buttons and improving layout.
- **Build**: Fixed various compilation warnings in `gcodekit5-visualizer`, `gcodekit5-designer`, and `gcodekit5-ui`.


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
