### Recent Updates (1.2.0-beta.2) - refactor: consolidate Sender trait, enforce protocol consistency, and dead code cleanup (2026-10-15)
- This update consolidates communication architecture, enforces strict protocol validation, and removes obsolete code. All changes have been verified in real-world environments

 ***Key Changes***

 - **Repetier Compatibility:**
	- Transparent Translation: The processor automatically converts G10 L20 (Zero All) to G92.
	- Pen Plotter Mode: Maps laser commands (M3/M4 → G0 Z0, M5 → G0 Z5) and discards redundant trailing M5 commands.
- **Non-functional protocols are indicated as (not supported)** and cannot be selected. The code for            these protocols has been left in the firmware for future implementations.
- **Logging & Telemetry:**
	- Centralized command logging directly within the Sender implementations, eliminating duplicate or inaccurate logs in the UI.
- **Communication Abstraction:**
	- Introduced a unified Sender trait (gcodekit5-communication/src/firmware/sender.rs) with flow control methods (send_gcode, pause, resume, stop, unlock, is_streaming).
	- Implemented by GrblSender and RepetierSender (formerly DirectSender, moved to core).
	- The UI (MachineControlView) now manages the active sender agnostically via Rc<RefCell<Option<Rc<dyn Sender>>>>.
- **Protocol Management & Consistency:**
	- Added init_commands to ProtocolProfile (removing hardcoded $10=47 and the redundant identity_query).
	- Enforced strict validation: Case-insensitive detection of expanded signatures (GRBL) and automatic connection rejection on protocol mismatch.
	- Injects 0x18 on connection to force the firmware boot banner.
	
- **Dead Code Cleanup:**
	- Various files and functions

**Verification Tests (Real Hardware)**

- Ray5 (GRBL): Full streaming, pause/resume, and recovery cycle (E-Stop → Unlock → Resume work without resetting).
- ANET A8 (Repetier): Connection, streaming flow control with resends, and functional pen plotter mode using a marker.

### Recent Updates (1.1.0-beta.1) - Multiple modifications and adjustments
- The grid adjustment checkbox now forces the cursor to move with a snapping point based on the adjustment parameter. Objects take values ​​in multiples of this adjustment value (Snap).
- In 3D mode, the Z coordinate is implemented to generate 3D G-code with passes based on the machining parameters in the Global or specific properties of each object.
- The 2D viewer is removed, and the 3D viewer is always used for all G-code, whether 2D or 3D. Camera positions are used for this purpose.
- The safety height parameter is generated from the entered value and the material thickness.
- A warning dialog box has been added for when G-code is generated outside the machine limits. The warning is also retained within the G-code itself.
- When saving as, the last path is used.
- Play/pause/stop buttons and playback speed values ​​have been added to the viewer for simulating the tool's path on screen.
- The shapes in the Shape Gallery have been corrected, and parametric mode has been added to the timing belt pulley, which previously lacked it.
- Some errors in creating chamfers and radii on shape objects have been fixed.

### Recent Updates (1.0.0-alpha.1) - Multiple modifications and adjustments
- Triangle generation in the designer has been corrected. Triangles can now be generated in all four positions and true symmetry can be applied. Previously, they could only be generated in one position.
- The bounds of the triangles are now generated correctly and updated when the triangle is rotated.
- The G-code and the Visualizer correctly reproduce the position of the triangles.
- The triangles are now correctly saved in the gckd file
- The display establishes a color scale based on the power S to preview the movements
- In the designer, the bounds for the created lines are corrected. They now take into account the size and rotation of the line, which they didn't do before.
- The Pause and Resume function in laser mode, which turned off the laser during some movements, has been fixed.
- Help is implemented via F1 with explanatory images for each program module
- The program translation has been expanded, including the help section.
- A Warning is added to the G-code when the path goes outside the machine area (negative coordinates)
- The positioning of texts and their grips is corrected.
- Other visual modifications


## [1.0.0-alpha.1] - 2026-05-21

### Added
- **individual laser parameters for each object**: In addition to the laser working parameters, it is now also possible to define the speed, power and passes properties individually for the desired objects while maintaining the global properties for the rest of the objects.

## [0.54.0-alpha.1] - 2026-05-06

### Added
- **Raster image import in Designer**: Full support for importing PNG, JPG, BMP, GIF, TIFF images
  - Image integration into GCKD file format for persistent storage
  - Halftone threshold controls for image preprocessing
  - High-speed streaming with 40-line window and overscan for laser engraving
- **Active machine display**: Machine name shown in status bar for quick identification
- **Translations**: CAM Tools, designer tool icons, and partial UI translations

### Changed
- **Raster image viewer display**: Improved raster rendering quality and performance in Viewer
- **Viewer optimization**: Performance improvements for rendering raster overlay data
- **Vectorized G-code simplification**: Cleaner, more compact G-code output
- **Path engraving improvements**: Better toolpath generation for vector paths
- **Feed rate, terminal, and status bar**: Various UI/UX improvements
- **GTK 4.14 compatibility**: Full sync with latest GTK4 API requirements

### Fixed
- Pause/stop management errors in Designer during streaming operations
- Stop/Reset logic: new flow disabling pause in raster mode
- Rotation, aspect ratio, and menu bugs in Designer
- Polygon width properties and visibility
- SVG and DXF import scaling and G-code generation quality
