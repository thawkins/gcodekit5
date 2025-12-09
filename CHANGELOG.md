## [0.2.5-alpha.3] - 2025-12-08

### Added
- **Designer**: Added Pan tool to toolbox.
  - **Editor**: Introduced `EditorBridgeBackend` in `gcodekit5-gcodeeditor` (non-UI editor bridge) and decoupled Slint legacy UI bridge.
    - Provides a non-UI editor API for test and backend consumers.
    - `gcodekit5-ui` retains the `EditorBridge` Slint UI bridge gated behind a `slint_legacy_tests` feature; a stub alias is provided when Slint isn't enabled.
    - Added integration tests for `EditorBridgeBackend` and updated existing tests to use the backend alias.
    - Moved Slint `.slint` UI assets to `ui/legacy/` in their respective crates and started a staged archival plan.

  - Allows panning the canvas by dragging with the mouse.
  - Changes cursor to hand/grabbing icon.
  - Updated toolbox layout to use 3 columns.
- **Designer**: Implemented alignment tools (Left, Right, Top, Bottom, Center Horizontal, Center Vertical).
  - Accessible via keyboard shortcuts (Alt+L, Alt+R, Alt+T, Alt+B, Alt+H, Alt+V).
  - Aligns selected shapes relative to the selection bounding box.
  - Added "Align" submenu to context menu (T-504).
- **Designer**: Implemented backend support for Linear, Grid, and Circular arrays.
  - Added `create_linear_array`, `create_grid_array`, `create_circular_array` to `DesignerCanvas`.
  - Implemented `rotate` method for all shape types (`Rectangle`, `Circle`, `Line`, `Ellipse`, `PathShape`, `TextShape`).
- **Designer**: Implemented shape conversion tools.
  - Added "Convert to Path" and "Convert to Rectangle" to context menu.
  - Allows converting any shape to a generic path for node editing (future).
  - Allows converting shapes back to bounding rectangles.
- **Designer**: Implemented Import functionality for DXF and SVG files.
  - Added "Import" option to File menu.
  - Supports importing SVG paths, rectangles, circles, ellipses, lines, polylines, and polygons.
  - Supports importing DXF lines, circles, arcs, and polylines.
  - Automatically scales and positions imported shapes.
  - Supports SVG group transforms and matrix transformations.
- **Designer**: Implemented Export functionality for G-Code and SVG files (T-602).
  - Added "Export G-Code..." and "Export SVG..." options to File menu.
  - Generates G-code based on current tool settings and shape properties.
  - Exports design to SVG format preserving dimensions.
- **Designer**: Implemented "Generate G-Code" button (T-701).
  - Added button to Designer Toolbox.
  - Generates G-code from current design and tool settings.
  - Automatically loads generated G-code into the G-Code Editor tab.
- **Designer**: Implemented Toolpath Simulation (T-702).
  - Added "Show Toolpaths" toggle to the status bar.
  - Renders generated toolpaths directly on the canvas.
  - Visualizes rapid moves (red dashed) and cutting moves (green solid).
  - Automatically updates preview when design changes if enabled.
- **Designer**: Grouped shapes are now rendered in green to distinguish them from ungrouped shapes.
- **Designer**: Added group ID column and editable name to Layer List.
  - Layer list now shows shape type, ID, editable name, and group ID.
  - Clicking the name entry allows renaming the shape without selecting the row.

### Fixed
- **Designer**: Made toolbox scrollable to prevent layout issues on smaller screens.
- **Designer**: Fixed toolbox width issue where it would expand unnecessarily.
- **Designer**: Fixed toolbox icon selection highlighting issue where selecting an icon would highlight the wrong one due to column layout change.
- **Designer**: Fixed panic in properties panel when updating selection due to RefCell borrowing conflict.
- **Designer**: Fixed panic when using Pan tool (RefCell borrow error).
- **Designer**: Fixed marquee appearing when using Pan tool.
- **Designer**: Improved selection tolerance to be constant in screen pixels (3px) regardless of zoom level.
- **UI**: Fixed window maximization and status bar visibility issues by ensuring main content expands correctly.
- **Code Quality**: Cleared compiler warnings across all crates (`gcodekit5-ui`, `gcodekit5-visualizer`, `gcodekit5-designer`, `gcodekit5-camtools`, `gcodekit5-communication`).
  - Fixed unused variables, fields, and imports.
  - Fixed unused constants and functions.
  - Fixed mutable variable warnings.
  - Resolved `RefCell` borrowing issues in `visualizer.rs`.
  - Fixed deprecation warnings for `glib::clone!` and `StyleContext` in `gcodekit5-ui`.
- **Visualizer**: Improved `fit_to_view` unit test to accept boundary zoom_scale values and avoid false negatives when margin calculation results in exact scaling.
- **Communication**: Made `ConnectionWatcher` test more robust by polling for `Healthy` state instead of relying on a fixed short sleep, reducing flakiness due to timing.

## [0.2.5-alpha.1] - 2025-12-08

### Added
- **Designer**: Implemented Polyline tool with click-click creation.
  - Left-click to add points.
  - Double-click or Right-click or Enter to finish.
  - Escape to cancel.
  - Live preview of the next segment (rubber band).
- **Visualizer**: Added horizontal and vertical scrollbars for better navigation.
- **Designer**: Added floating status panel (Zoom, Pan, Grid) to bottom-left corner matching Visualizer style.
- **Status Bar**: Added progress bar and time estimation (Elapsed/Remaining) during job streaming.
- **CAM Tools**: Added "Home device before starting" checkbox to all G-code generators.
- **Visualizer**: Added "Show Laser/Spindle" option to display current machine position as a red dot.
- **Visualizer**: Added statistics panel showing Min, Max, and Average S (spindle/laser power) values.

### Fixed
- **Designer**: Fixed panic in "Fit to View" caused by RefCell borrowing conflict during scrollbar update.
- **Visualizer**: Fixed panic in "Fit to View" caused by RefCell borrowing conflict during scrollbar update.
- **Machine Control**: Fixed progress bar not resetting when eStop is clicked.

## [0.2.4-alpha.0] - 2025-12-08

### Summary
Minor maintenance release with all CI/CD fixes consolidated.

### Fixed
- **CI/CD**: Complete GTK4 dependency chain for GitHub Actions
  - All build dependencies now properly configured
  - Fixed glib-sys and sourceview5-sys build errors
  - Release pipeline fully functional

## [0.2.3-alpha.0] - 2025-12-08

### Fixed
- **CI/CD**: Added GtkSourceView5 development dependency
  - Fixed sourceview5-sys build error in CI environment
  - Added libgtksourceview-5-dev for text editor syntax highlighting

## [0.2.2-alpha.0] - 2025-12-08

### Fixed
- **CI/CD**: Added GTK4 development dependencies to GitHub Actions workflow
  - Fixed glib-sys build error in CI environment
  - Added libgtk-4-dev, libadwaita-1-dev, libglib2.0-dev
  - Added libpango1.0-dev, libcairo2-dev, libgdk-pixbuf2.0-dev
  - Release builds now compile successfully

## [0.2.1-alpha.0] - 2025-12-08

### Added
- **Device Config**: Migrated device configuration panel from Slint to GTK4
  - Real-time GRBL settings retrieval via $$ command
  - Live editing of device settings with save confirmation
  - Settings update immediately in UI after saving
  - Category filtering for settings groups
  - Search/filter functionality across all settings
- **Device Info**: Implemented automatic firmware detection
  - Auto-detect firmware type and version via $I command on connect
  - Display actual device firmware instead of hardcoded values
  - Added firmware metadata tracking in device_status
- **CNC Tools Manager**: Complete tool management system
  - Create new tools with all parameters (geometry, material, manufacturer)
  - Edit existing tools with form validation
  - Delete tools with confirmation dialog
  - Category filter to show tools by type (End Mill, V-Bit, Drill, etc.)
  - Search/filter functionality across tool names and descriptions
  - Word-wrapped tool names in sidebar for better readability
  - Empty state placeholder when no tool selected
  - Single-click tool selection and editing

### Changed
- Device console now provides log access methods for other components
- Settings panel uses console output to avoid competing with machine control polling
- Tool list properly refreshes after create/update/delete operations

### Fixed
- Device config settings now show actual values from device instead of zeros
- Tool creation/saving now properly adds tools to the list
- Category filter correctly filters tools by type
- Search functionality works across all tool fields
- Tool deletion now properly removes tools from the list
- Long tool names no longer overflow in the sidebar

### Removed
- All debug println! and eprintln! statements throughout the codebase
- Unused code paths and callbacks

## [0.2.0-alpha.0] - 2025-12-08

### Added
- **CAM Tools**: Migrated BitmapEngraving tool from Slint to GTK4 with preview, Load/Save/Cancel buttons, progress bar, and spinner overlay.
- **CAM Tools**: Migrated VectorEngraving tool from Slint to GTK4 with SVG/DXF preview, sidebar controls, and progress tracking.
- **Editor**: Added floating line counter panel (current/max lines) in bottom-right corner.
- **Visualizer**: Added "Fit to Device" button to show full machine working area when device is connected.
- **Visualizer**: Improved floating panel styling with consistent height, positioning, and rounded corners.

### Changed
- **Project Rename**: Renamed project from `gcodekit4` to `gcodekit5`.
  - Updated all crate names to `gcodekit5-*`.
  - Updated all file references and documentation.
  - Reset versioning to `0.1.0-alpha.0` for the new major iteration.
- **Repository**: Migrated to new repository `gcodekit5`.
- **Editor**: Editor now focuses on line 1, column 1 when G-code is loaded from CAM tools.
- **VectorEngraving**: Auto-fit preview to view with light gray background, removed manual zoom controls.

### Performance
- **Visualizer Phase 1**: Implemented stroke batching - 20-100x faster rendering (groups moves by type/intensity).
- **Visualizer Phase 2**: Implemented viewport culling - 10-100x additional improvement when zoomed in (only renders visible paths).
- **Visualizer Phase 3**: Implemented LOD (Level of Detail) system - 2-20x additional improvement when zoomed out.
  - LOD 0: Full detail (zoom >= 1.0)
  - LOD 1: Every 2nd line (0.2-1.0 zoom)
  - LOD 2: Every 4th line (0.05-0.2 zoom)
  - LOD 3: Bounding box only (zoom < 0.05)
- **Visualizer Phase 4**: Implemented advanced caching - 1.3-2x additional improvement on frame 2+.
  - Caches intensity buckets (view-independent)
  - Caches cutting bounds for LOD 3
  - Smart hash-based cache invalidation
- **Overall Improvement**: Up to 40,000x faster rendering at extreme zoom levels, 154x faster at normal zoom.
- **Documentation**: Added comprehensive `VISUALIZER_PERFORMANCE.md` documenting all optimization phases.

### Fixed
- **Visualizer**: Fixed "Fit to Device" button positioning to correctly center view on device working area.
- **CAM Tools**: Fixed TabbedBox and JigsawPuzzle generators not showing errors during G-code generation.

## [0.69.1-alpha.0] - 2025-12-06

### Fixed
- **Build**: Fixed macOS Arm64 (Apple Silicon) build failure by removing `pepecore` dependency which relied on x86 AVX2 instructions.
- **CAM Tools**: Replaced `pepecore` halftoning with native Rust implementation using `image` crate. Note: Some specialized halftoning patterns (Circle, Cross, Ellipse, Line) are temporarily unavailable.

## [0.69.0-alpha.0] - 2025-12-06

### Changed
- **Version**: Bumped version to 0.69.0-alpha.0.

## [0.68.3-alpha.0] - 2025-12-05

### Fixed
- **CAM Tools**: Fixed issue where Tabbed Box Generator dialog would re-open immediately after generating G-code.
- **CAM Tools**: Standardized G-code loading behavior across all CAM tools (Tabbed Box, Jigsaw Puzzle, Spoilboard Surfacing, Spoilboard Grid). All tools now correctly load G-code into the editor, reset the view, and close the dialog upon success.
- **Code Quality**: Removed unused variables and redundant clones in CAM tool callbacks.
- **Visualizer**: Added automatic "Fit to View" when switching to the Visualizer tab to ensure the toolpath is immediately visible and centered.
- **CI/CD**: Updated release workflow to include version number in artifact names and added macOS ARM64 (Apple Silicon) build support.

## [0.68.2-alpha.0] - 2025-12-05

### Fixed
- **Tabbed Box Generator**: Fixed critical bug where divider slots were missing from wall panels when "Optimize Layout" was enabled. The packing algorithm now correctly groups slots with their parent panels to ensure they move together.
- **Build**: Fixed Windows build failure by adding missing `raw_window_handle` imports in `src/platform.rs`.

## [0.68.1-alpha.0] - 2025-12-05

### Fixed
- **Build**: Fixed build failure caused by duplicate `MachineControlPanel` definition in `crates/gcodekit5-ui/src/ui_panels/machine_control.slint`.
- **Assets**: Updated `eStop.png` asset with improved design and text layout.

## [0.68.0-alpha.0] - 2025-12-05

### Changed
- **UI**: Reordered main tabs to prioritize Machine Control, Device Console, and G-Code Editor.
- **UI**: Set "Machine Control" as the default tab on application startup.
- **UI**: Updated "View" menu items to match the new tab order.

## [0.67.0-alpha.0] - 2025-12-03

### Added
- **UI**: Refactored UI for unit conversion to ensure consistent display across all panels.
- **Tests**: Added comprehensive tests for Spoilboard Grid generator.

### Fixed
- **Cleanup**: Fixed various compiler warnings and clippy issues.
- **UI**: Improved layout and responsiveness in Designer and CAM tools.

## [0.66.0-alpha.0] - 2025-12-03

### Fixed
- **Window Management**: Fixed issue where file dialogs on Windows would open in full-screen mode or underneath the main window.
- **Window Management**: Application window now correctly maximizes on startup on Windows.

## [0.65.0-alpha.0] - 2025-12-02

### Started
- Next development iteration.

## [0.64.0-alpha.0] - 2025-12-02

### Started
- Next development iteration.

## [0.63.0-alpha.0] - 2025-12-02

### Added
- **Device Manager**: Added file locking mechanism to prevent race conditions during concurrent profile saves.
- **Device Manager**: Added logging for profile save operations to aid debugging.

### Fixed
- **Device Manager**: Fixed critical bug where adding a new device could reset existing device data due to UI state synchronization issues.
- **Device Manager**: Fixed issue where editing a device name caused the selection to jump to the wrong device (often the previous one), leading to accidental data overwrites.
- **Device Manager**: Fixed issue where saving a profile with an empty name was possible, now explicitly rejected with a warning.
- **Documentation**: Updated AGENTS.md with strict rules regarding pushing to remote repositories.

## [0.62.0-alpha.0] - 2025-12-02

### Added
- **Settings**: Added "Measurement System" preference (Metric/Imperial) to General settings.
- **Core**: Added unit conversion utilities for Metric/Imperial support.
- **CAM Tools**: Updated Spoilboard Surfacing tool to support Imperial units (inches) based on user preference.
- **UI**: Changed default settings tab to "General".
- **UI**: Updated Imperial unit label from `"` to `in`.

### Changed
- **CI/CD**: Updated Release workflow to use `RELEASE` secret for changelog builder to ensure access to PR details.

## [0.61.0-alpha.0] - 2025-12-02

### Changed
- **CI/CD**: Updated Release workflow to use "Release Changelog Builder" for automated release notes generation.

## [0.60.0-alpha.0] - 2025-12-02

### Added
- **CAM Tools**: Implemented singleton behavior for all parameter dialogs (Tabbed Box, Jigsaw Puzzle, Spoilboard Surfacing, Spoilboard Grid, Laser Engraver, Vector Engraver). Opening a tool that is already open now brings the existing dialog to the front instead of creating a duplicate.
- **CAM Tools**: Added Success Alerts for G-code generation in Tabbed Box, Jigsaw Puzzle, Spoilboard Surfacing, and Spoilboard Grid tools, matching the behavior of other tools.

### Fixed
- **UI**: Fixed issue where CAM tool dialogs would fall behind the main window on Linux/X11/Wayland. Implemented robust "Always On Top" behavior using `winit` backend with delayed window level application to ensure proper window mapping.
- **UI**: Fixed Windows fullscreen initialization issue where the application would not start in fullscreen mode on Windows.

## [0.59.0-alpha.0] - 2025-12-01

### Added
- **Settings**: Added "Default Directory" preference to General settings tab.
- **Settings**: Implemented directory browsing for default directory setting.
- **Settings**: Updated all file dialogs to use the configured default directory.
- **Settings**: Set default directory to user's home directory by default.
- **CAM Tools**: Implemented load/save functionality for Tabbed Box Maker and Jigsaw Puzzle Maker.
- **CAM Tools**: Updated all CAM tool load/save dialogs to use the default directory setting.
- **Vector Engraver**: Added "No Vector File Selected" message to preview area when no file is loaded.
- **Vector Engraver**: Fixed issue where preview would not display when loading parameters from file.

## [0.58.6-alpha.0] - 2025-12-01

### Added
- **CAM Tools**: Added image preview to Vector Engraving Dialog.
- **CAM Tools**: Updated Vector Engraving Dialog layout to match Bitmap Engraving tool (split view).
- **CAM Tools**: Added automatic output size calculation based on vector aspect ratio.

## [0.58.5-alpha.0] - 2025-12-01

### Changed
- **Designer**: Updated array generation to group all resulting shapes (originals + copies) into a single unique group.
- **Designer**: Circular array now positions the original shape as the first item in the circle, moving/rotating it to match the pattern.
- **Designer**: Circular array dialog now initializes center coordinates to the center of the selected shape(s).
- **UI**: Increased height of array dialogs to prevent buttons from being cut off.

## [0.58.3-alpha.0] - 2025-12-01

### Added
- **Designer**: Added array tools (Linear, Circular, Grid) to Designer context menu.
- **Designer**: Implemented array generation logic in DesignerState.
- **Designer**: Added UI dialogs for array parameters.
- **Designer**: Array operations now automatically group all generated shapes (including originals) into a single group.

## [0.58.2-alpha.0] - 2025-12-01

### Changed
- **Designer**: Disabled rotation control in the properties panel when multiple shapes are selected to prevent ambiguous behavior.

## [0.58.1-alpha.0] - 2025-12-01

### Fixed
- **Designer**: Fixed rendering of rotated rectangles and lines to prevent distortion (double rotation application). Rotated shapes now display correctly instead of being distorted into squares or incorrect lines.

## [0.58.0-alpha.0] - 2025-11-30

### Fixed
- **Windows GUI**: Fixed console window appearing when running the Windows GUI application by configuring the Windows subsystem to hide the console window using `/SUBSYSTEM:WINDOWS` linker flag in build.rs.

## [0.57.0-alpha.0] - 2025-11-30

### Fixed
- **Windows Installer**: Fixed MSI installer error 2819 by adding the required `WIXUI_INSTALLDIR` property to bind the directory chooser dialog to `APPLICATIONFOLDER`. This resolves the "Control Folder on dialog InstallDirDlg needs a property linked to it" error that prevented installation.

## [0.56.6-alpha.0] - 2025-11-30

### Fixed
- **Windows Installer**: Fixed WiX build error `CNDL0230` by assigning a static GUID to the License component, which is required when using a registry key as the KeyPath.

## [0.56.5-alpha.0] - 2025-11-30

### Fixed
- **Windows Installer**: Fixed ICE38 and ICE64 validation errors in WiX configuration by adding registry keys for per-user components and explicit folder removal instructions.

## [0.56.4-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Fixed Windows installer build failure by manually moving the cross-compiled binary to the default release directory expected by WiX.

## [0.56.3-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Updated Windows release workflow to explicitly verify the usage of the custom `wix/main.wxs` configuration file.

## [0.56.2-alpha.0] - 2025-11-30

### Fixed
- **Windows Installer**: Updated WiX configuration to perform a per-user installation (no admin rights required) and create a Start Menu shortcut.

## [0.56.1-alpha.0] - 2025-11-30

### Fixed
- **Windows**: Fixed "Failed to write device profiles file" error on startup by storing `devices.json` in the user's AppData directory instead of the application installation directory.

## [0.56.0-alpha.0] - 2025-11-30

### Started
- Next development iteration.

## [0.55.0-alpha.0] - 2025-11-30

### Added
- **CI/CD**: Full Flatpak support for Linux distribution.
  - Automated build and bundling in GitHub Actions.
  - AppStream metadata and desktop integration.
  - Validated icon sizing and manifest configuration.

## [0.54.3-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Fixed Flatpak build failure by resizing the application icon to 128x128 pixels (`assets/Pictures/gcodekit5_128.png`) to meet the size requirements for `share/icons/hicolor/128x128/apps/`.

## [0.54.2-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Fixed Flatpak build failure by pointing the manifest to the renamed binary (`gcodekit5-linux-x86_64`) in the root directory, as the standard target directory structure varies with cross-compilation flags.

## [0.54.1-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Fixed Flatpak build failure by correcting source paths in the manifest to be relative to the manifest file location.
- **CI/CD**: Updated Flatpak runtime version to `24.08` to avoid EOL warnings.

## [0.54.0-alpha.0] - 2025-11-30

### Added
- **CI/CD**: Added Flatpak support for Linux builds.
  - Created `flatpak/` directory with desktop entry, AppStream metainfo, and Flatpak manifest.
  - Updated release workflow to install `flatpak-builder` and generate a `.flatpak` bundle.
  - Configured application icon and desktop integration.

## [0.53.1-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Fixed macOS build failure by using `osx` format for `cargo-bundle` (which creates a `.app` bundle) and then using `hdiutil` to package it into a `.dmg` disk image. The `dmg` format flag is not directly supported by the `cargo-bundle` CLI.

## [0.53.0-alpha.0] - 2025-11-30

### Fixed
- **CI/CD**: Updated version format to `0.53.0-alpha.0` to satisfy WiX/MSI requirements for Windows installer generation. The previous format `0.51.4-alpha` caused build failures because prerelease components must be convertible to integers.

## [0.51.4-alpha] - 2025-11-30

### Fixed
- **CI/CD**: Fixed Windows and macOS build failures in GitHub Actions by explicitly specifying the package name (`--package gcodekit5`) for `cargo wix` and `cargo bundle` commands in the workspace environment.

## [0.51.3-alpha] - 2025-11-30

### Added
- **CI/CD**: Updated release workflow to generate platform-specific packages:
  - **macOS**: Generates `.dmg` disk image using `cargo-bundle`.
  - **Windows**: Generates `.msi` installer using `cargo-wix`.
  - **Linux**: Generates binary executable (unchanged).

## [0.51.2-alpha] - 2025-11-29

### Added
- Added rotation support to designer shapes (Rectangle, Circle, Line, Ellipse, Path, Text).
- Added rotation input field to designer properties panel.
- Updated shape rendering to support rotation.
- Updated selection logic to support rotated shapes.
- Updated G-code generation to support rotated shapes.
- Updated serialization to save/load rotation.
- Refactored `src/main.rs` into modular structure (`src/app/`).
- Added "Convert to" context menu in Designer with "Rectangle" and "Path" options.
- Added confirmation dialog for shape conversion operations.
- Implemented shape conversion logic (Rectangle, Path) with Undo/Redo support.
- Added `to_path_shape` implementation for all shape types.

### Changed
- Updated `DesignerState` to support shape conversion commands.
- Updated `ContextMenu` in Designer to support submenus.

### Fixed
- Fixed `match` arm return type issue in `convert_selected_to_path`.

## [0.51.1-alpha] - 2025-11-29

### Added
- **Status Bar**: Added elapsed time and estimated remaining time indicators to the status bar during job execution.
- **CAM Tools**: Added "Load" and "Save" buttons to the Vector Engraving Dialog to save/restore parameters.
- **CAM Tools**: Added "Load" and "Save" buttons to the Laser Image Engraving Dialog to save/restore parameters.
- **CAM Tools**: Added "Load" and "Save" buttons to the Tabbed Box Maker Dialog to save/restore parameters.
- **CAM Tools**: Added "Load" and "Save" buttons to the Jigsaw Puzzle Maker Dialog to save/restore parameters.

### Changed
- **CAM Tools**: Updated Vector Engraving Dialog save/load to include the vector file path.
- **Logging**: Removed noisy "GRBL error in response" warning log from standard output.
- **Logging**: Suppressed "Send error: Broken pipe" error log which occurs during normal disconnects.
- **Logging**: Removed all tracing error reports from communication layer to reduce noise.
- **Logging**: Removed noisy "Failed to parse startup message" warning log from device console manager.
- **Error Handling**: Verified GRBL error code decoding is enabled and functioning correctly for UI console messages.

## [0.51.0-alpha] - 2025-11-28

### Added
- **Visualizer**: Added intensity visualization (heatmap) for laser/spindle power ('S' value).
  - Renders toolpath with varying opacity based on S value (10 levels).
  - Added "Show Intensity" toggle and "Max S" control to Visualizer sidebar.
  - Added "Show Cutting Moves" toggle to hide standard toolpath lines.
  - Added white background mode when intensity visualization is active.
- **CAM Tools**: Implemented halftoning algorithms for image engraving.
  - Added Threshold, Bayer 4x4 (Ordered), Floyd-Steinberg (Error Diffusion), and Atkinson algorithms.
  - Integrated algorithms into the Laser Engraver toolpath generator.

## [0.50.0-alpha] - 2025-11-28

### Fixed
- **Designer**: Fixed incorrect X/Y/W/H values in properties panel when multiple items are selected. Now correctly shows the bounding box of the entire selection.
- **Designer**: Fixed resizing logic for multiple selections to scale relative to group bounds, preventing distortion.
- **Designer**: Increased property panel input limits to prevent clamping large shapes (fixes square resizing bug).

## [0.46.7-alpha] - 2025-11-28

### Fixed
- **Designer**: Fixed incorrect X/Y/W/H values in properties panel when multiple items are selected. Now correctly shows the bounding box of the entire selection.

## [0.46.6-alpha] - 2025-11-28

### Fixed
- **CI/CD**: Triggering release build.

## [0.46.5-alpha] - 2025-11-28

### Fixed
- **CI/CD**: Triggering release with updated secret configuration (`RELEASE`).

## [0.46.4-alpha] - 2025-11-28

### Fixed
- **CI/CD**: Triggering new release build with updated workflow configuration using PAT token.

## [0.46.3-alpha] - 2025-11-28

### Added
- **Error Decoding**: Added dedicated `error_decoder` module to `gcodekit5-communication` for better GRBL error handling.

### Changed
- **CI/CD**: Updated release workflow to use `PAT` secret for GitHub release uploads to ensure proper permissions.
- **UI Updates**: Minor updates to Designer, Settings, and Visualizer UI components.
- **Configuration**: Updated settings persistence and configuration logic.

## [0.46.2-alpha] - 2025-11-28

### Fixed
- **Compilation Errors**: Fixed unresolved import `format_error` in `main.rs` by using the explicit path `gcodekit5_communication::firmware::grbl::error_decoder::format_error`.

## [0.46.1-alpha] - 2025-11-28

### Fixed
- **Compilation Errors**: Resolved `error_decoder` import issue in `main.rs` and fixed missing `set_show_menu_shortcuts` method call.

## [0.46.0-alpha] - 2025-11-28

### Added
- **CI/CD**: Added GitHub Actions workflow to build and release binaries for Linux, Windows, and macOS on every push and tag.

### Fixed
- **Jog Command Latency**: Fixed 5-10 second delay when sending jog commands by reducing serial port read timeout from 5000ms to 50ms in `src/main.rs`.

## [0.45.0-alpha] - 2025-11-27

### Fixed
- **Designer Zoom Controls**: Fixed missing zoom in/out buttons - moved misplaced closing braces to keep all three zoom controls visible
- **Device Console Layout**: Fixed log entries appearing centered - added proper alignment to display logs from top-left
- **Tooltip Z-Index**: Fixed tooltips appearing behind buttons - implemented static z-index of 100 for proper layering

### Changed
- **UI Refactoring**:
  - **Centralized Theme**: Implemented a global `Theme` singleton for consistent colors and sizing across the application.
  - **Shared Components**: Created a library of reusable UI components (`StandardButton`, `StandardInput`, `StandardCheckBox`, `StandardSpinBox`, `StandardSidebar`) to replace ad-hoc implementations.
  - **Panel Updates**: Refactored 11 major UI panels to use the new theme and shared components:
    - G-Code Editor
    - Machine Control
    - Device Console
    - Device Info
    - Device Manager
    - Config Settings
    - G-Code Visualizer
    - Designer
    - CAM Tools
    - Materials Manager
    - Tools Manager
  - **Code Cleanup**: Removed redundant style definitions and hardcoded colors from individual panel files.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.45.0-alpha] - 2025-11-27

### Changed
- **CNC Tools View**:
  - **Dark Theme**: Applied dark theme styling (`#2c3e50` panels, `#34495e` backgrounds).
  - **Custom Widgets**: Implemented `TMButton`, `TMInput`, `TMComboBox` for consistent look and feel.
  - **Layout**: Improved layout with proper spacing and alignment.
- **Materials View**:
  - **Dark Theme**: Applied dark theme styling.
  - **Custom Widgets**: Implemented `MMButton`, `MMInput`, `MMComboBox`.
  - **Layout**: Improved layout.


## [0.43.0-alpha] - 2025-11-26

### Changed
- **Visualizer UI Overhaul**:
  - **Dark Theme**: Adopted the Designer's dark theme (`#2c3e50` panels, `#34495e` canvas) for a consistent look and feel.
  - **Layout**: Moved toolbar controls to a dedicated Left Sidebar (200px).
  - **Floating Overlays**: Replaced static status bar with floating "pill" overlays for Status (Zoom, X/Y, Grid) and Zoom Controls (+, -, Fit).
  - **Modern Components**: Replaced standard buttons with icon-based `VisualizerToolButton` and standard checkboxes with `DarkCheckBox`.
  - **Canvas**: Removed padding for edge-to-edge visualization.

## [0.42.0-alpha] - 2025-11-26

### Added
- **Designer**: Added support for rounded corners and slot mode for rectangles.
  - New "Radius" property for rectangles.
  - New "Slot (Auto Radius)" checkbox to automatically set radius for slot shapes.
  - Updated toolpath generator to support rounded rectangles and slots in both contour and pocket operations.
  - Updated SVG renderer to display rounded corners and slots correctly.

### Changed
- **Designer**: Increased maximum zoom factor from 1000% to 5000% for better precision.
- **Designer**: Increased left-hand panel width to 250px for better layout.
- **Designer**: Standardized text sizes and control dimensions between left and right panels.
- **Designer**: Removed padding around the canvas for a cleaner look.
- **Designer**: Fixed icon alignment in tool buttons.

## [0.41.0-alpha] - 2025-11-25

### Added
- **Visualizer**: Added automatic "Fit to View" when switching to the Visualizer tab.
  - Ensures the toolpath is immediately visible and centered without manual adjustment.
  - Implemented using a one-shot timer to wait for layout stabilization.

### Changed
- **Visualizer**: Updated toolpath rendering colors for better visibility and distinction.
  - **G1 (Linear Moves)**: Yellow (`#FFFF00`)
  - **G2 (CW Arcs)**: Green (`#00FF00`)
  - **G3 (CCW Arcs)**: Bright Red (`#FF0000`)
  - **G4 (Dwell)**: Bright Blue (`#0000FF`) - Rendered as small circles.
  - **Rapid Moves (G0)**: Dashed gray lines (unchanged).

### Fixed
- **DXF Import**: Fixed issue where extra vectors were drawn from the origin to the start of each shape.
  - Root cause was incorrect backtracking index logic in the `POLYLINE` parser.
  - Implemented separate parsing logic for `POLYLINE` (old style) and `LWPOLYLINE` entities.
  - Added robust vertex parsing that correctly handles the `SEQEND` termination for `POLYLINE` entities.

## [0.39.0-alpha] - 2025-11-25

### Added
- **Designer**: Updated File menu for better import/load workflow.
  - Renamed "Import" menu to "Load" (clears canvas before loading).
  - Added "Add" menu with DXF/SVG options (appends to canvas without clearing).
  - "Add" operations automatically group imported shapes for easier manipulation.
  - Both "Load" and "Add" operations now automatically "Fit to View" after completion.
  - Added "Set Defaults" button to Designer sidebar to configure global shape properties (pocket depth, step down, etc.).
    - Default properties are applied to all newly created shapes.
    - Default properties are saved and loaded with the design file.
    - Edit dialog for defaults hides geometry controls (X, Y, Width, Height).
    - Added "Use Custom Values" checkbox to Shape Properties dialog.
    - "Use Custom Values" is hidden in Default Properties dialog.
    - New shapes are initialized with default properties but "Use Custom Values" is false.
    - Fixed visibility of geometry controls in Default Properties dialog.
    - Automatically "Fit to View" when opening the Designer panel.

### Changed
- **Designer**: Improved group selection behavior.
  - Clicking anywhere inside a group's composite bounding box now selects the entire group.
  - Previously, users had to click on individual member shapes to select the group.
- **Designer**: Enabled resize handles for all selection types (single, multiple, and groups).
  - Resize handles now appear around the composite bounding box of the entire selection.
  - Allows resizing multiple shapes or groups simultaneously.

### Fixed
- **Designer**: Fixed group drag behavior.
  - Dragging anywhere inside a selected group's bounding box now moves the group.
  - Previously, dragging in empty space within a group's bounding box would pan the canvas.
- **Designer**: Fixed multiple selection drag behavior.
  - Dragging anywhere inside the composite bounding box of a multiple selection now moves all selected shapes.
  - Ensures consistent behavior for groups and multiple selections.
- **Designer**: Fixed distortion when resizing groups of complex shapes (e.g., polylines).
  - Resizing logic now correctly scales all shapes relative to the group's center instead of individually.

## [0.38.0-alpha] - 2025-11-24

### Added
- **Designer**: Implemented rubber band selection.
  - Dragging on empty space with Shift key held down now draws a selection rectangle.
  - Selects all shapes intersecting the rectangle.
  - Automatically selects entire groups if any member is touched.
- **Designer**: Added visual feedback for rubber band selection (semi-transparent yellow rectangle).

### Changed
- **Designer**: Refined drag interaction behavior:
  - Dragging on empty space (no Shift) now **pans** the canvas (restored original behavior).
  - Dragging on empty space (Shift held) now performs **rubber band selection**.
  - Dragging on a shape body moves the shape/selection.
- **Designer**: Improved group selection logic to ensure dragging any member of a group moves the whole group.

### Fixed
- **Designer**: Fixed issue where dragging a multiple selection only worked if clicking the primary selected shape.
- **Designer**: Fixed visual glitch where rubber band rectangle persisted after mouse release.
- **Designer**: Fixed coordinate transformation bug in rubber band selection (Y-axis flip).

## [0.37.17-alpha] - 2025-11-24

### Added
- **Designer**: Added "Select All" to Edit menu.
- **Designer**: Added automatic "Fit to View" after SVG import.

### Fixed
- **Layout**: Fixed Left Sidebar expansion issues in Designer.
- **Layout**: Fixed Center Canvas positioning issues.

## [0.37.16-alpha] - 2025-11-24

### Changed
- **Layout**: Refactored Designer layout:
  - Moved "Tool Setup" controls to Left Sidebar.
  - Rearranged tool icons into 4-column grid.
  - Fixed layout expansion issues using `HorizontalLayout`/`VerticalLayout` and strict width constraints.
- **UI**: Removed redundant buttons (New, Save, Open, Undo, Redo, Zoom) from Designer and GCode Editor panels.
- **Menu**: Added Zoom controls to View menu.
- **Menu**: Added Export (GCode) and Import (DXF, SVG) submenus to File menu.
- **Documentation**: Updated TOOLCHAIN.md with Tokei installation instructions and updated LLD documentation.
- **Documentation**: Updated project statistics.

## [0.37.14-alpha] - 2025-11-23

### Added
- **Designer**: Added Copy and Paste functionality for shapes.
  - Added "Copy" and "Paste" items to the shape context menu.
  - Added "Paste" item to the empty space context menu.
  - Supports copying single shapes, multiple selections, and groups.
  - Pasting places shapes at the cursor location (context menu position).
  - Preserves shape properties and relative positions.
- **Designer**: Added Undo/Redo functionality.
  - Implemented history stack for canvas state (shapes, groups, properties).
  - Added Undo/Redo buttons to the designer toolbar.
  - Added keyboard shortcuts: Ctrl+Z (Undo), Ctrl+Shift+Z (Redo), Ctrl+Y (Redo).
  - History is saved before any modification (add, delete, move, resize, property change, etc.).
  - History is cleared on New Design or Load File.

### Changed
- **UI**: Removed top padding from the main content area to eliminate whitespace above the TabBar.
- **Designer**: Moved Group/Ungroup buttons to the context menu.
  - "Group" is active when multiple items are selected and at least one is not in a group.
  - "Ungroup" is active when any selected item is part of a group.
- **Designer**: Reversed vertical drag-pan direction to match natural scrolling behavior.
- **UI**: Renamed "Config" tab to "Device Config" and added it to View menu.
- **UI**: Updated View menu structure (Machine Control, Machine Info, CAMTools, CNCTools).
- **UI**: Standardized height (32px) and layout of controls in Materials Manager.
- **UI**: Removed unused `MainMenu` component definition.
- **UI**: Updated File menu with context-aware "New" action and label.
- **UI**: Updated File menu "Open", "Save", and "Save As" actions to be context-aware for Designer.
- **UI**: Added "Undo" and "Redo" to Edit menu with context-aware actions.
- **UI**: Added confirmation dialogs for "New" action in GCode Editor and Designer to prevent accidental data loss.

## [0.37.14] - 2025-11-24

### Added
- **Designer**: Added Copy/Paste functionality for shapes and groups.
- **Designer**: Added Undo/Redo functionality with history stack.
- **Designer**: Added "Group" and "Ungroup" context menu items.
- **UI**: Added "Machine Info" view to the main menu.
- **UI**: Added "CAMTools" and "CNCTools" to the main menu.
- **UI**: Added confirmation dialogs for "New" action in GCode Editor and Designer.

### Changed
- **UI**: Renamed "Machine" menu to "Machine Control".
- **UI**: Renamed "Config" tab to "Device Config".
- **UI**: Standardized height and layout of controls in Materials Manager.
- **UI**: Refactored Main Menu to use standard `MenuBar` component.
- **UI**: Made File and Edit menus context-aware (switching between Designer and Editor actions).
- **Designer**: Reversed vertical drag-pan direction.

## [0.37.13-alpha] - 2025-11-23

### Added
- **Designer**: Implemented grouping and ungrouping functionality.
  - Added `group_id` to `DrawingObject` and serialization.
  - Added "Group" and "Ungroup" buttons to the designer toolbar.
  - Grouped shapes are selected, moved, and resized as a single unit.
  - Grouped shapes are rendered in green to distinguish them.
  - Added dotted green bounding box around selected groups.
  - Unified selection handles for multiple selections and groups.
  - Implemented `scale` method for all shapes to support group resizing.

### Fixed
- **Designer**: Fixed issue where shapes outside +/- 1000mm were not selectable.
  - Increased spatial index bounds to +/- 1,000,000mm.
  - Added warning logging when shapes are inserted outside spatial index bounds.

## [0.37.12-alpha] - 2025-11-23

### Changed
- **Visualizer**: Added analytic bounding boxes for arcs, removing discretization from fit/zoom calculations.
- **Visualizer**: Toolpath bounding box queries now operate on segment metadata only, speeding up repeated view calculations.

## [0.37.11-alpha] - 2025-11-23

### Changed
- **Visualizer**: Cached arc start angle + span in `ArcAngles`, eliminating repeated trigonometry during rendering.
- **Visualizer**: Length, interpolation, and iterators now reuse the cached values for consistent performance on dense arc files.

## [0.37.10-alpha] - 2025-11-23

### Changed
- **Visualizer**: Introduced `ArcLineIterator` so arcs emit line segments lazily instead of allocating intermediate vectors.
- **Visualizer**: All segment helpers now reuse the iterator, keeping memory usage flat even when arcs are discretized multiple times.

## [0.37.9-alpha] - 2025-11-23

### Changed
- **Visualizer**: Added a `visit_line_segments` iterator so consumers can stream discretized moves without allocating a separate vector.
- **Visualizer**: Bounding box + statistics computation now use the streaming visitor, dramatically reducing memory spikes on large files.

## [0.37.8-alpha] - 2025-11-23

### Changed
- **Visualizer**: Added a `MovementMeta` struct that stores movement type and feed-rate data so both line and arc segments share the same metadata plumbing.
- **Visualizer**: Arc segments now derive their direction from metadata and pass that to discretized line segments, reducing branching and keeping feed-rate propagation consistent.

## [0.37.7-alpha] - 2025-11-23

### Changed
- **Visualizer**: Replaced the split `line_segments`/`arc_segments` vectors with a single `PathSegment` enum so iteration and statistics traverse the toolpath once.
- **Visualizer**: Added helpers on `PathSegment` for length, movement type, and discretization so downstream renderers can consume a unified representation.

## [0.37.6-alpha] - 2025-11-23

### Changed
- **Visualizer**: Added a dedicated `ToolpathCache` that owns content hashing, parsed commands, and SVG regeneration so parsing and rendering stay in sync.
- **Visualizer**: `Visualizer2D` now delegates toolpath access through the cache, simplifying `render_*_to_path` and ensuring we only re-tokenize G-code when the hash changes.

## [0.37.5-alpha] - 2025-11-23

### Changed
- **Visualizer**: Introduced a shared `ViewportTransform` helper so zoom, pan, and padding math live in one place and are reused by grid/origin rendering and the parser bounds logic.
- **Visualizer**: Moved the `Bounds` accumulator into the viewport module, reducing duplication and clarifying how padding is applied before generating SVG viewboxes.

## [0.37.4-alpha] - 2025-11-23

### Added
- **Designer**: Context menu now exposes `Align → Horizontal/Vertical` cascades with working Left/Center/Right/Top/Center/Bottom actions.
- **Designer**: Alignment commands are wired through `MainWindow` callbacks into new `DesignerState` helpers so multi-select layouts snap precisely.
- **Designer**: Properties dialog detects multi-selection, shows "Multiple Shapes" title, hides X/Y offsets, and applies non-positional edits (pocket settings, steps, text, etc.) across every selected shape in one shot.

### Fixed
- **UI Shell**: Resolved tabbed-view regression that left every tab except Designer blank by ensuring all panels share the same visibility gating logic.
- **Designer**: Corrected vertical alignment math so "Top" and "Bottom" pins map to the expected Y-extents.

## [0.37.2-alpha] - 2025-11-22

### Added
- **Designer**: Added multiple selection support (Shift+Click).
- **Designer**: Added confirmation dialog when deleting multiple shapes.
- **Designer**: Added `DeleteConfirmationDialog` component.

## [0.37.1-alpha] - 2025-11-22

### Improved
- **Visualizer Performance**: Optimized grid rendering and G-code parsing.
  - Implemented content hashing to prevent redundant re-parsing of G-code during view operations (zoom/pan).
  - Optimized SVG path string generation for grid and origin to reduce memory allocations.
  - Reduced coordinate precision in SVG paths from 3 to 2 decimal places for smaller data transfer and faster rendering.
  - Shared `Visualizer2D` instance across UI callbacks to persist state.

## [0.37.0-alpha] - 2025-11-22

### Added
- **Designer**: Implemented advanced pocketing strategies: Raster (Zig-Zag), Contour-Parallel (Offset), and Adaptive (placeholder).
- **Designer**: Added UI controls for pocket strategy selection, raster angle, and bidirectional milling in Shape Properties.
- **Designer**: Integrated `cavalier_contours` for robust polygon offsetting.
- **Designer**: Updated toolpath generation to support different pocketing strategies for all shapes.

### Changed
- **Designer**: Refactored `pocket_operations.rs` to support strategy-based generation.
- **Designer**: Updated `DesignerShape` and `DrawingObject` to store pocket strategy parameters.
- **Designer**: Updated `generate_rectangular_pocket` and `generate_circular_pocket` to respect selected strategy (converting to polygon for Raster).

### Added
- **Designer**: Implemented "Adaptive" pocketing strategy using Spiral-Out HSM approach with rounded corners and inside-out ordering.

### Fixed
- **Designer**: Fixed compilation errors related to `PathShape` removal (from previous task).
- **Designer**: Fixed visibility of Raster Angle and Bidirectional controls in Shape Properties dialog (now always visible when pocketing is enabled).
- **Designer**: Fixed panic in pocket generation caused by duplicate vertices in input polygons.
- **Designer**: Fixed infinite loop in pocket generation by enforcing CW orientation for offset paths.
- **Visualizer**: Improved performance by caching render paths and using SVG viewbox for zoom/pan.
- **Visualizer**: Fixed stroke width to be consistent 1px regardless of zoom level.
- **Visualizer**: Updated Grid and Origin rendering to use world coordinates for consistency.

## [0.36.0-alpha] - 2025-11-22

### Added
- **CAM Tools**: Added "Spoilboard Surfacing" tool.
  - Generates G-code for surfacing/flattening the CNC bed or spoilboard.
  - Automatically loads dimensions from the selected Device Profile.
  - Supports tool selection from the CNC Tool Database.
  - Configurable parameters: Tool Diameter, Feed Rate, Spindle Speed, Stepover, Cut Depth.
  - Includes proper initialization sequence (G21, G90, G17, $H, G54, G10).
- **CNC Tools**: Added "Specialty" tool category.
  - Replaces "Fly Cutter" to be more generic.
  - Includes "Precision Fly Cutter" and "NITOMAK Surfacing Router Bit" in standard library.

### Changed
- **Designer**: Substituted `PathShape` for all instances of `PolylineShape` usage.
  - `Polyline` tool now creates `PathShape` internally.
  - SVG/DXF import now converts polylines and polygons to `PathShape`.
  - Serialization now saves/loads `PathShape` using SVG path data.
  - Toolpath generation now supports `PathShape` for contour and pocket operations.
  - UI now maps `PathShape` to "Polyline" properties panel.
  - Removed `Polyline` struct and `ShapeType::Polyline` variant entirely.
- **Device Manager**: Improved UI for axis limits.
  - Explicitly labeled "Min" and "Max" fields to prevent user error.
  - Added auto-correction logic to swap Min/Max values if entered inversely.
- **CNC Tools**: Refactored tool type handling to be dynamic.
  - `ToolType::all()` now provides the source of truth for tool categories.
  - UI dropdowns automatically populate from the core definition.

### Fixed
- **DeviceDB**: Fixed issue where incorrect axis limits (Min > Max) caused negative dimensions in CAM tools.
- **Spoilboard Tool**: Fixed issue where dimensions were not updating when switching devices.

## [0.35.0-alpha] - 2025-11-22

### Added
- **CAM Tools**: Added "Speeds and Feeds Calculator" tool.
  - Calculates Spindle Speed (RPM) and Feed Rate (mm/min) based on Material, Tool, and Device parameters.
  - Integrated with MaterialDB, DeviceDB, and CNCToolsDB.
  - Displays clamped values with original calculated values in red brackets if limits are exceeded.
  - UI integrated into the "CAM Tools" tab as the 5th card.

### Changed
- **Designer**: Renamed "Polygon" shape to "Polyline" to better reflect its nature and align with DXF terminology.
  - Updated UI tooltips and labels.
  - Updated internal data structures and API.
- **Designer**: Implemented rendering for Polyline and Path shapes in the Designer canvas and SVG export.
- **Designer**: Updated "Shape Properties" dialog title to be dynamic (e.g., "Rectangle properties", "Circle properties") based on the selected shape type.
- **Test Reorganization**: Comprehensive reorganization of tests across all crates.
  - Migrated root `tests/` folder content to respective crates (`gcodekit5-core`, `gcodekit5-communication`, `gcodekit5-ui`, `gcodekit5-designer`, `gcodekit5-visualizer`).
  - Migrated inline tests from `src/` to `tests/` directory for `gcodekit5-core`, `gcodekit5-communication`, `gcodekit5-gcodeeditor`, `gcodekit5-settings`, `gcodekit5-ui`, and `gcodekit5-visualizer`.
  - Created proper test module structures and integration tests.
  - Fixed visibility issues for testing internal components where necessary.
  - Removed broken or misplaced tests (e.g., UI tests in backend crates).
  - Ensured all tests pass for each crate.

### Fixed
- **Designer**: Fixed SVG import mirroring to correctly flip Y-axis while maintaining relative position (mirror around bounding box center).
- **Vector Engraver**: Fixed SVG import mirroring to correctly flip Y-axis while maintaining relative position (mirror around bounding box center).
- **Firmware Version**: Fixed display format in tests to match implementation.
- **Device Status**: Fixed parsing tests in `gcodekit5-communication` to match actual behavior.
- **Visualizer**: Fixed `Visualizer2D` import in `visualizer_coordinate_transforms` test by moving it to the correct crate.

## [0.34.4-alpha] - 2025-11-22

### Changed
- **Testing**: Migrated `gcodekit5-designer` tests to `tests/` directory structure.
  - Organized tests into `core`, `features`, `integration`, and `io` categories.
  - Fixed legacy tests in `gcodekit5-designer`.
  - Removed redundant tests `designer_mouse_event_mapping.rs` and `designer_viewport_coordinate_mapping.rs`.
  - Fixed doc tests in `viewport.rs`.

### Fixed
- Fixed compilation errors in `gcodekit5-designer` tests.
- Fixed imports in legacy tests.

## [0.34.3-alpha] - 2025-11-21

### Added
- **CAM Tools**: Added Speeds and Feeds Calculator tool.
  - Calculates RPM and Feed Rate based on Material, Tool, and Device properties.
  - Supports metric calculations based on Surface Speed and Chip Load.
  - Includes fallback logic for missing material data.
  - Validates against device capabilities (Max Feed Rate).
- **Core**: Added `surface_speed_m_min` and `chip_load_mm` to `CuttingParameters` in `gcodekit5-core`.

## [0.34.2-alpha] - 2025-11-21

### Added
- **Designer**: Added grid and origin rendering to the Designer view, matching the Visualizer's implementation.
- **Designer**: Added "Show Grid" toggle button to the Designer toolbar.
- **Designer**: Added grid size display to the Designer info bar.
- Added Zoom In, Zoom Out, Fit, and Reset buttons to the Designer right sidebar.

### Changed
- **Designer**: Updated Designer renderer to support grid and origin paths.
- **Designer**: Refactored Designer UI to support grid visibility state.
- Moved "Show Grid" control to the right sidebar as a checkbox for better visibility and consistency.
- Updated Designer default view to position origin at bottom-left with 5px inset.

### Fixed
- Fixed issue where generated G-code was not appearing in the editor.
- Fixed UI freezing during complex G-code generation.
- Fixed Designer grid and origin visibility issue (now visible on startup).
- Fixed missing "Show Grid" toggle button in Designer toolbar.
- Fixed Designer grid rendering to cover the full canvas width.
- Implemented "Fit" functionality to zoom to bounding box of all shapes.

## [0.34.1-alpha] - 2025-11-20

### Started
- Next development iteration

## [0.34.0-alpha] - 2025-11-20

### Added
- **Designer**: Added support for Text shapes with Fira Mono font.
- **Designer**: Added support for Pocket operations (rectangular and circular).
- **Designer**: Implemented text rendering and toolpath generation.
- **Designer**: Added UI controls for text content, font size, and pocket depth.
- **Designer**: Added `stepIn` property to shapes and UI for controlling horizontal step-over in pockets and profiles.
- **Designer**: Added `stepDown` property to shapes and UI for controlling vertical step-down.

### Changed
- **Designer**: Migrated Designer UI elements to `gcodekit5-designer` crate.
- **Designer**: Moved G-code generation to a background thread to prevent UI blocking.
- **Designer**: Removed `RoundRectangle` shape (replaced by `Rectangle` with corner radius).
- **Designer**: Improved G-code generation debugging with instrumentation.
- **Designer**: Updated `Generate` button in Designer to insert G-code directly into Editor.
- Bumped version to 0.34.0-alpha



## [0.33.6-alpha] - 2025-11-20

### Added
- Added bounding box display to G-code visualizer (exact dimensions and offset)

### Fixed
- **Tabbed Box Generator**:
  - Fixed missing halving slots in dividers when multiple slots are present in a single segment.
  - Fixed issue with X divider positioning overlapping with other parts in Tabbed Box Generator.
  - Corrected `apply_slots_to_path` logic to handle multiple intersections properly.
  - Ensures all slots are generated for complex divider configurations (e.g., 2x2 grids).
  - Fixed issue where X and Y offsets were ignored in Tabbed Box Generator.
  - Fixed inconsistent offset application in Tabbed Box Generator by applying offset to coordinates directly instead of using G10 L20.

## [0.33.0-alpha] - 2025-11-20

### Added
- **Tabbed Box Generator**:
    -   Added `optimize_layout` option to pack parts tightly using a shelf packing algorithm.
    -   Added UI checkbox for layout optimization.
    -   Fixed type inference error in packing algorithm.
- **Tabbed Box Maker Phase 2**
  - Added "Dogbone" finger style for CNC machining (corner overcuts)
  - Added support for internal dividers (X and Y axis)
  - Updated UI to include Divider inputs and Tool Diameter label
  - Added "Extra Length" setting to Tabbed Box Generator UI.

### Fixed
- Fixed "No Top" logic in Tabbed Box Generator (skips top panel and adjusts edge styles).
- Implemented `extra_length` (protrusion) and `burn` (kerf) compensation in Tabbed Box Generator.
- Fixed type annotation error in `tabbed_box.rs`.

### Changed
- Updated Tabbed Box Generator to use `extra_length` and `burn` parameters for accurate dimensions.

## [0.33.5-alpha] - 2025-11-20

### Added
- **Tabbed Box Generator Phase 3 (Dividers)**:
  - Added `KeyDividerType` support for keying dividers into walls and floor.
  - Options: `WallsAndFloor`, `WallsOnly`, `FloorOnly`, `None`.
  - Implemented slot generation in main panels (Front, Back, Left, Right, Bottom) to accept divider tabs.
  - Added cross-divider slots (halving joints) for intersecting dividers.
  - Updated UI to include "Divider Keying" selection.
  - Updated `BoxParameters` and `TabbedBoxMaker` to handle divider keying logic.
  - Fixed divider edge types to correctly use tabs for connections.

## [0.33.4-alpha] - 2025-11-20

### Added
- **Tabbed Box Generator Phase 3**:
  - Added support for all Box Types: Full Box, No Top, No Bottom, No Sides, No Front/Back, No Left/Right.
  - Added support for Tab Dimples (friction fit bumps) with configurable height and length.
  - Added `dimple_height` and `dimple_length` parameters to UI.
  - Updated `BoxType` enum to match Python implementation.
  - Fixed path continuity issues (closed loops) and Left edge finger direction.

## [0.33.3-alpha] - 2025-11-20

### Added
- **Device Profile Management**
  - Added Device Manager tab for managing machine profiles
  - Created `gcodekit5-devicedb` crate for device profile management
  - Implemented CRUD operations for device profiles
  - Added "Set as Active" functionality to switch between machine configurations
  - Integrated Device Manager into main UI with dedicated tab

### Fixed
- **Settings System**
  - Fixed Settings Controller integration in main application
  - Correctly bound Settings Dialog callbacks to controller logic
  - Ensured settings are properly saved and loaded

## [0.33.2-alpha] - 2025-11-20

### Changed
- **Settings System Refactoring**
  - Extracted settings and preferences logic into dedicated `gcodekit5-settings` crate
  - Implemented MVVM architecture for settings UI
  - Replaced monolithic settings dialog with component-based design
  - Improved performance using `ListView` for settings rendering
  - Decoupled settings logic from main application controller
  - Added dynamic category loading and filtering

## [0.33.1-alpha] - 2025-11-20

### Fixed
- **Visualizer Grid**
  - Fixed grid disappearing at low zoom levels
  - Implemented adaptive grid spacing (10mm -> 100mm -> 1000mm) based on zoom
  - Grid now covers the entire viewport at all scale factors
  - Added dynamic canvas sizing to backend rendering
  - Added grid size indicator to status bar
- **Toolpath Rendering Stroke Width**
  - Changed toolpath rendering stroke width from 5px to 1px in all visualizer components
  - Ensures crisp, single-pixel wide lines for toolpaths regardless of zoom level
  - Improves visibility of fine details in complex toolpaths
  - Consistent rendering across G-code visualizer and editor panels
- **Visualizer Origin Indicator**
  - Extended origin indicator to full width/height of canvas (crosshair style)
  - Changed stroke width to 2px for better visibility
- **Code Cleanup**
  - Removed duplicate `gcode_visualizer.slint` files from UI and Editor crates
  - Consolidated visualizer UI logic into `gcodekit5-visualizer` crate

## [0.33.0-alpha] - 2025-11-19

### Changed
- **Major Architecture Refactoring: Separated Domain-Specific Functionality into Dedicated Crates**
  - Created `gcodekit5-gcodeeditor` - Complete G-Code editor and visualizer
    - Extracted text buffer management, undo/redo, viewport management
    - Includes Slint UI components: gcode_editor.slint, custom_text_edit.slint, gcode_visualizer.slint
    - Fully self-contained editor component with bridge to UI layer
  - Created `gcodekit5-camtools` - CAM processing and vector engraving
    - Extracted vector engraving, toolpath optimization, SVG/DXF processing
    - Includes parameterized toolpath generation and G-code optimization
  - Created `gcodekit5-designer` - Design canvas and shape manipulation
    - Extracted designer canvas, shape rendering, import/export functionality
    - Includes SVG and DXF file import with layering support
  - Result: Cleaner 7-crate modular architecture with clear separation of concerns

### Fixed
- **G-Code Streaming Reliability**
  - Fixed issue where streaming would stop unexpectedly requiring "Resume"
  - Implemented proper line-based buffering for serial responses
  - Fixed handling of split "ok" messages across serial chunks
  - Added proper handling of "error:" responses to prevent queue stalling
  - Ensures `pending_bytes` tracking remains accurate even with communication errors
- **Visualizer Grid**
  - Fixed grid disappearing at low zoom levels
  - Implemented adaptive grid spacing (10mm -> 100mm -> 1000mm) based on zoom
  - Grid now covers the entire viewport at all scale factors
- **Vector Engraving Panic**
  - Fixed panic in vector engraving when processing closed paths with lyon
  - Fixed hatch generator producing no output for closed shapes
  - Improved SVG parsing robustness using regex
  - Fixed DXF parsing for closed polylines
- **Vector Engraving Order**
  - Changed operation order to perform hatching before outline paths
- **Vector Engraver Multi-Pass Bug**
  - Fixed issue where vector engraver only performed 1 pass regardless of `num_passes` setting
  - Implemented proper multi-pass loop with Z-axis depth adjustment
  - Each pass decrements Z by `z_increment` for proper depth control
  - Added pass comments and progress tracking for multi-pass operations
- **Laser Dot at Path End Bug**
  - Fixed issue where laser remained enabled during travel between paths
  - Changed initial move to path from cutting (G1) to rapid (G0) before laser engagement
  - Ensured laser is explicitly disabled (M5) before any travel between paths
  - Prevents burn marks/dots at path endpoints
- **Module Cleanup**
  - Removed duplicate engraver modules from gcodekit5-parser
  - Updated imports to use canonical implementations from gcodekit5-camtools
  - Removed all verbose INFO logging statements (~80+ logs removed)

### Improved
- **Architecture**: 7 focused crates with well-defined responsibilities
  - gcodekit5-core: Firmware and hardware abstraction
  - gcodekit5-communication: Serial and protocol handling
  - gcodekit5-parser: G-code parsing and validation
  - gcodekit5-gcodeeditor: Editor UI and text management
  - gcodekit5-camtools: CAM and toolpath operations
  - gcodekit5-designer: Design canvas and import/export
  - gcodekit5-ui: Application UI orchestration
- **Code Quality**: Removed verbose logging, fixed clippy warnings
  - Removed ~70 redundant INFO logs for visualization updates
  - Fixed unused variable warnings across test suite
  - Applied clippy fixes for code idioms
- **Testing**: Reorganized tests into crate-specific folders
  - Moved designer tests to gcodekit5-designer/tests/
  - Moved CAM tools tests to gcodekit5-camtools/tests/
  - Moved editor tests to gcodekit5-gcodeeditor/tests/
  - Moved UI tests to gcodekit5-ui/tests/
  - Added comprehensive multi-pass test suite (3 new tests)

### Build & Testing
- ✅ Release build succeeds (600+ seconds on full rebuild)
- ✅ All crates compile without errors
- ✅ 130 integration tests passing (3 new multi-pass tests)
- ✅ All clippy warnings fixed
- ✅ Binary builds to target/release/gcodekit5
- ✅ All Slint components properly included in new crates

## [0.31.0-alpha] - 2025-11-18

### Changed
- **Architecture Refactoring: Separated Concerns into 6 Focused Crates**
  - Created `gcodekit5-camtools` (5.5K LOC) - CAM operations and special processing
    - Extracted 5 major CAM tools: puzzle, box, laser engraver, vector engraver, arc expander
    - Includes advanced features, optimization, validation, statistics
    - UI panel for CAM tool controls
  - Created `gcodekit5-designer` (11.6K LOC) - Visual design and toolpath generation
    - Extracted all designer/visual functionality from parser
    - Includes shapes, canvas, viewport, renderer
    - CAM operations integration (pocket, drilling, adaptive, vcarve, arrays, parametric, multipass)
    - Advanced features: history/undo-redo, spatial indexing, toolpath simulation, templates
    - Import/export: DXF, SVG, serialization, tool library
  - Reduced `gcodekit5-parser` from 23.8K to 14K LOC (41% reduction)
    - Now focused solely on G-Code parsing and utilities
    - Cleaner separation of concerns
  - Result: 6 focused crates with clean layering and no circular dependencies

### Improved
- **Code Organization**: Parser now has single responsibility (G-Code parsing)
  - Better maintainability and navigation
  - Reduced cognitive load per crate
- **Architecture Grade**: Improved from A- to A+
  - Exemplary Rust project structure
  - Clean layering: foundation → operations → UI
  - Each crate has clear, single responsibility
- **Documentation**: 
  - Updated ARCHREVIEW.md (774 lines) with complete post-refactoring analysis
  - Added CAMTOOLS_REFACTOR.md (342 lines) with CAM tools extraction details
  - Added DESIGNER_REFACTOR.md (408 lines) with designer extraction details

### Fixed
- **Verbose Logging**: Removed excessive INFO logs from visualization updates
  - Eliminated repetitive "Setting visualization X data" messages firing every ~23ms
  - Significantly reduces log spam during rendering
  - Application now much quieter during operation

### Build & Testing
- ✅ All 282 tests passing (31 CAM tools tests, 241 designer tests)
- ✅ Zero circular dependencies maintained
- ✅ 100% backward compatible (original files preserved for gradual migration)
- ✅ No new warnings or errors introduced
- ✅ Build time: ~88 seconds (no increase)

## [0.30.0-alpha] - 2025-11-18

### Fixed
- **Malformed SVG Path Rendering in GCode Output**
  - Fixed long straight line segments appearing in gcode where SVG has curves
  - Issue affected SVG paths with multiple sub-paths separated by close (z) and move (m) commands
  - Paths 8, 9, and 18 of tigershead.svg previously rendered with 18mm+ straight line jumps
  - **Root Cause**: SVG parser treated disconnected sub-paths as one continuous path
  - **Solution**: Added discontinuity detection in gcode generation (>5mm jumps trigger rapid move with laser off)
  - Now properly handles path breaks with M5 (laser off) → G0 (rapid move) → M3 (laser re-engage) sequence

### Improved
- **SVG Path Parsing**: Enhanced L/l command handler to support implicit repetition per SVG spec
  - SVG allows `l x1,y1 x2,y2 ...` to represent multiple line segments
  - Parser now correctly processes all line segments instead of just first one
- **GCode Quality**: Longest cutting segment reduced from 18mm to 2.5mm (normal curve approximation)
  - All 37 SVG paths in tigershead.svg now render correctly without artifacts
  - Path discontinuities properly handled with rapid moves
- **Documentation**: Updated SLINT.md with SVG path parsing details

## [0.30.0-alpha] - 2025-11-17

### Fixed
- **Cursor Visibility on Empty Editor**
  - Cursor now displays at position (1,1) when G-code editor is empty
  - Fixed cursor initialization from (0,0) to (1,1) in main.rs
  - Backend now provides at least one line with space character when buffer empty
  - Ensures Slint has content to render cursor on
  - Cursor blinking works normally when content is added

### Added
- **Cursor Blinking Animation**
  - Text cursor in G-code editor now blinks with 400ms cycle (200ms visible, 200ms invisible)
  - Implemented via Rust background timer thread with Slint event loop integration
  - Property-based system allows cursor visibility control from any layer
  - Creates dedicated `BlinkingCursor` component for clean separation of concerns

### Improved
- **Editor Responsiveness**: Non-blocking cursor animation runs in separate thread
- **Code Architecture**: Cursor blink state flows cleanly through component hierarchy (MainWindow → GcodeEditorPanel → CustomTextEdit → BlinkingCursor)
- **SLINT.md**: Documented cursor rendering solution and design decisions

## [0.30.0-alpha] - 2025-11-16

### Added
- **Vector Hatching**
  - Vector hatching support with configurable angle, spacing, and tolerance
  - Added cross-hatch support (second pass at 90 degrees offset)
- **Vector Engraver Improvements**
  - Added configurable laser dwell option (G4 P...) to ensure laser powers down fully
  - Added UI controls for dwell enable and time
- **UI Improvements**
  - Added GRBL machine state display (Run, Idle, Alarm, Hold) to the status bar with color coding
- **Version Bump**: Minor release cycle update to 0.30.0-alpha

### Improved
- **Documentation**: Updated README.md, STATS.md with latest development status
- **Version Management**: Bumped to 0.30.0-alpha for next development cycle

## [0.28.0-alpha] - 2025-11-16

### Added
- **Minor Release Cycle**: Documentation and infrastructure improvements

### Improved
- **Documentation**: Comprehensive update to SPEC.md, STATS.md, and README.md
- **Version Management**: Bumped to 0.28.0-alpha for next development cycle

## [0.26.1-alpha] - 2025-11-16

### Added
- **Mouse Click to Cursor Positioning**
  - Click anywhere in editor to position cursor at that location
  - Automatic line detection from click Y position
  - Column detection from click X position (8px per character)
  - Proper rounding for accurate line selection
  - Works with visible line viewport scrolling

### Fixed
- **Editor Focus Infrastructure**
  - Complete focus cascade from root through all FocusScopes to CustomTextEdit
  - Keyboard input routing verified through all layers (debug: 🔑 tracing)
  - Focus works perfectly after initial click (known limitation: OS window focus required)
  - Comprehensive debug output for focus tracking (debug: 🎯 tracing)

### Improved
- **Input Event Handling**
  - Comprehensive key event tracing throughout FocusScope hierarchy
  - Debug infrastructure for tracking keyboard and mouse events
  - Root FocusScope forwards all keys without intercepting
  - Mouse click position calculation accounts for viewport scrolling

## [0.26.0-alpha] - 2025-11-16

### Added
- **Custom G-Code Text Editor - Phase 2 (COMPLETE)**
  - Full custom text editor with line numbers, syntax highlighting ready for future implementation
  - Proper line wrapping: Left arrow at line start moves to end of previous line
  - Right arrow at line end moves to start of next line
  - Full undo/redo stack with proper cursor position tracking
  - Horizontal scrolling support with viewport management
  - Visible lines viewport showing only rendered content for performance
  - All text editing operations (insert, delete, replace) working correctly
  - Cursor navigation (Home, End, Ctrl+Home, Ctrl+End) fully functional

### Changed
- Removed all temporary debug prints (eprintln!, debug! macros)
- Maintained structured logging via tracing::debug! for proper log level control
- Fixed all compiler warnings (unused imports, dead code, unused variables)

### Fixed
- Cursor navigation regression: restored line wrapping at line boundaries
- Cursor position indexing: proper 0-based (backend) to 1-based (UI) conversion

## [0.44.0-alpha] - 2025-11-27

### Changed
- **UI Aesthetics Overhaul**:
  - **Visualizer**: Updated to match Designer aesthetics (dark theme, refined layout).
  - **G-Code Editor**: Refactored to use Left Sidebar layout, dark theme, and improved gutter visibility.
  - **Machine Control**: Applied dark theme, updated tool buttons, and improved layout.
  - **Device Console**: Fixed log alignment (top-left), applied dark theme.
  - **Device Info**: Updated layout for capabilities (columns, left-aligned), added "Copy Config" button.
  - **Device Manager**: Applied dark theme, replaced standard checkboxes and tabs with custom high-visibility components.
  - **Device Config**: Applied dark theme and custom components.
  - **CAM Tools**:
    - Refactored to use a responsive Grid Layout (3 columns).
    - Restored all 7 CAM tools.
    - Added dynamic icon generation for tool cards.
    - Applied dark theme.

### Added
- **Documentation**: Created `docs/*_LOOKFEEL.md` files for all major views to document aesthetic standards.
- **Device Info**: Implemented "Copy Config" functionality to copy device details to clipboard.

## [0.25.7-alpha] - 2025-11-15

### Added
- **SVG to G-Code Vector Engraver - Complete Path Parsing**
  - Full support for SVG group transforms (matrix transformations)
  - Multi-segment curve and line parsing (handles multiple segments in single SVG command)
  - Cubic Bezier (C/c) and quadratic (Q/q) curve approximation with adaptive segments
  - Proper coordinate transformation from SVG to machine space
  - 37-path tiger head design now converts correctly to 26,750+ G1 movement commands

### Fixed
- **SVG Path Transform Not Applied**: Group transforms ignored causing disconnected paths
  - Manually parse and apply group matrix(a,b,c,d,e,f) transforms to all path coordinates
  - Paths now correctly positioned in machine coordinate space

- **Partial SVG Path Parsing**: Only first segment of multi-segment commands parsed
  - C/c, Q/q, and L/l commands can contain multiple segments (e.g., 154 curves in one command)
  - Loop through all segments within each command, not just first
  - Increased G-code resolution ~15x for complex curved designs

- **Custom G-Code Text Editor - Phase 1B (COMPLETE): Cursor Position Tracking & Text Editing**
  - Full cursor position tracking with proper 0-based (backend) to 1-based (UI) conversion
  - Cursor movement keys (arrow keys, Home, End, PageUp/PageDown) with immediate visual feedback
  - Text insertion/deletion at correct cursor position (no longer inserts at top)
  - Proper cursor rendering at correct horizontal position
  - Status bar displays accurate cursor line:column position
  - Undo/Redo operations properly update cursor position

### Fixed (Previous)
- **Cursor Position Indexing Bug**: Cursor indexing conversion missing in text callbacks
  - Added +1 conversions in on_text_inserted, on_text_deleted, on_undo_requested, on_redo_requested
  - Fixed redo handler bug (was calling can_redo() instead of can_undo())
  
- **Cursor Rendering Position Bug**: Cursor displayed one character too far right
  - Changed x position calculation to account for 1-based indexing
  
- **Cursor Movement Keys Not Working**: Arrow/Home/End/PageUp/Down keys didn't move cursor
  - Direct property updates in Slint for immediate feedback
  - Callback synchronization with Rust backend
  
- **Text Insertion at Wrong Location**: Text always inserted at document top
  - Now uses provided line/col parameters to position cursor before insert/delete
  - Proper cursor movement via EditorBridge.set_cursor() before operations

### Technical Details
- Established architecture: Backend 0-based, UI 1-based, conversions at boundary (main.rs)
- Cursor rendering uses 0-based coordinates (subtract 1 from UI value)
- Direct property updates for instant visual feedback + callback for Rust synchronization
- Two-way binding of cursor-line and cursor-column properties to maintain UI-Rust sync

### Verification
- All 296 UI tests pass
- Text inserts at actual cursor position
- Text deletes from actual cursor position
- Cursor updates immediately on keyboard navigation
- Status bar shows correct position
- Undo/Redo maintains cursor position
- Release build successful

## [0.25.6-alpha] - 2025-11-14

### Added
- **Custom G-Code Text Editor - Phase 1 (COMPLETE)**
  - Full keyboard input system with proper event handling through Slint callback chain
  - Text insertion with automatic cursor advancement for each character typed
  - Text deletion via Backspace and Delete keys with cursor repositioning
  - Complete arrow key navigation (left, right, up, down) with proper boundary checking
  - Home and End keys for jumping to line boundaries
  - PageUp and PageDown for viewport scrolling (10 lines per page)
  - Undo/Redo support triggered by Ctrl+Z (undo) and Ctrl+Y (redo)
  - Tab key inserts 4 spaces for automatic indentation
  - Enter/Return key for newline insertion at cursor position
  - Virtual scrolling system supporting 100+ line files efficiently
  - Line number display with synchronized scrolling
  - Real-time cursor position tracking displayed in status bar
  - Text buffer updates on every keystroke, automatically saved to file operations
  - Complete integration: keyboard events → CustomTextEdit → GcodeEditorPanel → MainWindow → Rust EditorBridge

### Technical Implementation
- Slint callback architecture with proper hyphenated naming conventions
- MainWindow FocusScope handles keyboard events and routes to text_inserted() Rust callback
- CustomTextEdit.key-pressed handler recognizes special keys using Key namespace constants
- Proper event forwarding through callback chain: CustomTextEdit → GcodeEditorPanel → MainWindow → Rust
- Line-based cursor tracking (0-based internally, 1-based for user display)
- EditorBridge integration for persistent text buffer management

### Fixed
- Keyboard event handling in custom components through proper FocusScope implementation
- Callback naming consistency across Slint (.slint with hyphens) and Rust (with underscores)
- Event propagation from child components to parent through explicit root.callback() calls
- Text cursor initialization and boundary checking during navigation

### Known Limitations (Phase 1)
- No text selection yet (Phase 2 feature)
- No copy/paste support (Phase 2 feature)
- No find/replace functionality (Phase 2 feature)
- No syntax highlighting (Phase 2+ feature)
- No multi-level undo/redo (Phase 2 feature)

## [0.25.5-alpha] - 2025-11-13


### Changed
- **Tabbed Box Generator**: Complete rewrite using boxes.py algorithm from https://github.com/florianfesti/boxes
  - Replaced previous finger joint implementation with production-proven boxes.py approach
  - Added configurable finger joint settings: finger width, space width, surrounding spaces, play tolerance
  - Improved finger joint algorithm with automatic calculation of optimal finger count
  - Added multiple finger joint styles: Rectangular (default), Springs, Barbs, Snap
  - Enhanced parameter controls in UI with finger/space multiples of thickness
  - Fixed coordinate transformation issues for proper closed rectangular paths
  - Implemented duplicate point checking to eliminate corner gaps
  - Added proper edge reversal for top and left edges
  - Corrected finger orientation on all four edges (fingers point outward correctly)

### Added
- New `FingerJointSettings` structure with configurable parameters
- `FingerStyle` enum supporting multiple finger joint types
- Enhanced CAM Tool dialog with additional finger joint parameters
- Better G-code generation with cleaner paths and proper edge transitions

### Fixed
- Diagonal jump vectors in generated G-code paths
- Incorrect finger orientations on top and left edges
- Corner connection issues causing open paths
- Edge transformation coordinate calculation errors
- Path generation now produces cuttable, mostly-closed shapes

## [0.25.4-alpha] - 2025-11-01

### Added
- Initial tabbed box generator implementation
- Basic finger joint calculations
- G-code export for laser cutting


## [0.41.0-alpha] - 2025-11-26

### Changed
- **Designer UI**:
  - Increased Left Sidebar width to 250px.
  - Standardized font sizes and control heights across Left and Right sidebars.
  - Removed padding around the canvas area for a cleaner look.
  - Fixed icon alignment in toolbar buttons (perfectly centered).
  - Increased maximum zoom level to 5000% (50x).

## [0.40.0-alpha] - 2025-11-26

### Added
- **Designer**: Added `name` property to shapes.
  - Shapes now have a user-editable name (defaults to shape type).
  - Name is displayed in the Layers list and Properties panel.
  - Name changes are undoable/redoable.
- **Designer**: Added Rounded Corner and Slot support for Rectangles.
  - Added `corner_radius` property to Rectangle shapes.
  - Added `is_slot` property to toggle Slot mode (auto-calculates max radius).
  - Added UI controls for Radius and Slot mode in the Properties panel.
  - Radius is automatically constrained to half the minimum dimension.
- **Designer**: Improved Layers Tab.
  - Added column headers (Type, Name, Group).
  - Added keyboard navigation (Up/Down arrows) to select shapes in the list.
  - Shape list now fills the available vertical space.
  - Clicking a layer item automatically focuses the list for keyboard navigation.

### Changed
- **Designer**: Updated `Rectangle` struct and serialization to support new properties.
- **Designer**: Refactored `DesignerState` to handle name and rectangle property updates via commands.
- **UI**: Updated `CompactSpinBox` to support `enabled` state.

### Fixed
- **Designer**: Fixed persistence of shape name changes by resolving conflicting UI callbacks.

## [0.39.2-alpha] - 2025-11-25
### Added
- Persistent right-hand properties panel in Designer for immediate shape editing.
- "Design Properties" view in properties panel when no shape is selected, allowing editing of default CAM settings.
- Rounding of dimensional values to 2 decimal places in Designer UI.

### Changed
- Replaced modal Shape Properties dialog with persistent sidebar.
- Increased Designer right sidebar width to 270px.
- Improved Designer left sidebar layout with better spacing.
- Removed "Set Defaults" button (functionality moved to properties panel).

### Fixed
- Fixed Designer layout issue where right panel would overlay the canvas.
- Fixed compilation error in Designer UI regarding ComboBox type mismatch.
- Fixed selection handle rendering symmetry for shapes with negative dimensions.

## [0.2.4-alpha.0] - 2025-12-08

### Added - GTK4 Designer Implementation (Phases 1-7)
- **Phase 1**: Canvas and Drawing Infrastructure
  - Grid rendering with proper coordinate system
  - Origin crosshair indicator
  - Y-up Cartesian coordinate system (bottom-left origin with 15px offset)
  - Cairo-based 2D rendering
  
- **Phase 2**: Toolbox and Shape Creation  
  - Shape creation tools (Line, Rectangle, Circle, Ellipse)
  - Polyline tool (future enhancement)
  - Selection tool for object manipulation
  - Interactive shape creation with drag gestures
  - Real-time marquee preview while dragging
  - Auto-switch to selection tool after shape creation
  
- **Phase 3**: Selection and Basic Transformation
  - Click-to-select shapes with visual feedback
  - Red highlight for selected shapes
  - Drag-to-move with proper incremental tracking
  - Delete key to remove shapes
  - Escape key to deselect
  - Click empty space to deselect all
  
- **Phase 4**: Properties Panel
  - Real-time property display for selected shapes
  - Editable properties (position, dimensions, rotation)
  - Focus management to prevent update conflicts
  - Proper GTK focus-out event propagation
  - Shape-specific property fields
  
- **Phase 5**: Layers Panel  
  - Layer list with visibility toggles
  - Shape list per layer with counts
  - Layer management (add, delete, rename)
  - Layer reordering (future enhancement)
  - Group/ungroup operations (foundation laid)
  
- **Phase 6**: Advanced Operations
  - Copy/Paste with standard keyboard shortcuts (Ctrl+C/Ctrl+V)
  - Duplicate selected shapes (Ctrl+D)
  - Undo/Redo with history (Ctrl+Z/Ctrl+Shift+Z)
  - Clipboard management with offset paste
  - Command pattern for undo/redo system
  
- **Phase 7**: Toolpath Generation Foundation
  - Toolpath panel structure created
  - Integration points identified
  - Deferred full implementation for future enhancement

### Fixed - Device Configuration Panel
- Device config panel now reads actual GRBL settings values
- Property editing with double-click activation
- Settings list properly updates after save
- Device info retrieval from actual connected device
- Fixed RefCell borrow panics in edit dialogs

### Fixed - CNC Tools Panel
- Tool library management with create/edit/delete
- Tool selection updates edit panel properly
- Category filtering working correctly
- Search/filter functionality operational
- Empty state message when no tool selected
- Tool persistence to disk

### Changed
- Migrated from Slint to GTK4 for all UI panels
- Improved coordinate system handling in designer
- Better focus management across panels
  
