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
