# macOS Build and Distribution Guide

## Overview

GCodeKit5 now supports building and distributing on macOS (both Intel x86_64 and Apple Silicon ARM64 architectures) with automatic framework bundling via GitHub Actions.

## GitHub Actions Workflow

The workflow in `.github/workflows/release.yml` includes automated macOS builds that:

1. **Build for both architectures:**
   - `x86_64-apple-darwin` (Intel Macs)
   - `aarch64-apple-darwin` (Apple Silicon Macs)

2. **Automatic dependency management:**
   - Installs GTK4, SourceView, and all required libraries via Homebrew
   - Bundles frameworks and libraries into the app bundle
   - Creates distributable DMG files

3. **Release automation:**
   - Uploads DMG files to GitHub Releases
   - Maintains separate builds for each architecture

## Building Locally

### Prerequisites

```bash
# Install Homebrew dependencies
brew install gtk4 libgtksourceview5 adwaita-icon-theme librsvg libpng jpeg webp

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.shell/env

# Add ARM64 target if cross-compiling
rustup target add aarch64-apple-darwin
```

### Build for Current Architecture

```bash
# Debug build
timeout 600 cargo build

# Release build
timeout 600 cargo build --release
```

### Build for Specific Architecture

```bash
# Build for ARM64 (Apple Silicon)
timeout 600 cargo build --release --target aarch64-apple-darwin

# Build for x86_64 (Intel)
timeout 600 cargo build --release --target x86_64-apple-darwin
```

### Create App Bundle and DMG

After building, the GitHub Actions workflow automatically:

1. **Creates app bundle structure:**
   ```
   GCodeKit.app/
   ├── Contents/
   │   ├── MacOS/
   │   │   └── gcodekit5 (executable)
   │   ├── Resources/
   │   │   ├── AppIcon.png
   │   │   └── share/ (icons, schemas, etc.)
   │   ├── Frameworks/ (optional, for future expansion)
   │   ├── Libs/ (all dependencies)
   │   └── Info.plist
   ```

2. **Runs framework bundling:**
   ```bash
   bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin
   ```

3. **Creates distributable DMG:**
   ```bash
   bash scripts/macos-create-dmg.sh v0.44.0-alpha.1 macos-arm64
   ```

## Manual Build Process

If you need to build manually on macOS:

```bash
# 1. Build the release binary
timeout 600 cargo build --release --target aarch64-apple-darwin

# 2. Create app bundle
mkdir -p "GCodeKit.app/Contents/MacOS"
mkdir -p "GCodeKit.app/Contents/Resources"

# 3. Copy binary and resources
cp target/aarch64-apple-darwin/release/gcodekit5 "GCodeKit.app/Contents/MacOS/"
cp assets/Pictures/com.github.thawkins.gcodekit5.png "GCodeKit.app/Contents/Resources/AppIcon.png"

# 4. Create Info.plist (use the one from the workflow)

# 5. Bundle frameworks
bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin

# 6. Create DMG
bash scripts/macos-create-dmg.sh "v0.44.0" "macos-arm64"
```

## Framework Bundling Details

The `scripts/macos-bundle-frameworks.sh` script:

- Copies all GTK4 and SourceView libraries to `Contents/Libs/`
- Copies icon themes and GLib schemas to `Contents/Resources/`
- Updates `install_name_tool` references to point to bundled libraries
- Configures `@rpath` settings for library discovery at runtime

### Bundled Components

- **GTK4:** Core windowing and UI framework
- **SourceView5:** Syntax highlighting for code editing
- **GLib:** Core utilities and data structures
- **Cairo:** 2D graphics rendering
- **Pango:** Text layout and rendering
- **GdkPixbuf:** Image loading and manipulation
- **HarfBuzz:** Text shaping
- **Adwaita:** GNOME icon theme
- **Supporting libraries:** fontconfig, freetype, libpng, jpeg, webp, etc.

## DMG Distribution

The `scripts/macos-create-dmg.sh` script creates a professional DMG installer that:

- Includes a symbolic link to the Applications folder
- Sets icon view with appropriate sizing
- Positions app and Applications folder icons
- Uses UDZO compression for smaller file size
- Creates files matching the pattern: `gcodekit5-<version>-<arch>.dmg`

## Signing and Notarization (Future)

For production releases, consider adding:

1. **Code Signing:**
   ```bash
   codesign -s - --deep --force --verify --verbose=4 "GCodeKit.app"
   ```

2. **Notarization (required for distribution):**
   - Requires Apple Developer account
   - Command-line tools available via `altool` or `xcrun notarytool`

## Troubleshooting

### Library Not Found Errors

If the app fails to launch with library path errors:

1. Check that `@rpath` is set correctly:
   ```bash
   otool -L "GCodeKit.app/Contents/MacOS/gcodekit5"
   ```

2. Verify libraries exist in `Contents/Libs/`

3. Run the bundling script again:
   ```bash
   bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin
   ```

### Build Failures on M1/M2 Macs

Ensure you have the correct target installed:
```bash
rustup target add aarch64-apple-darwin
```

### GTK4 Not Found During Build

Make sure Homebrew GTK4 is installed:
```bash
brew install gtk4
export PKG_CONFIG_PATH="$(brew --prefix gtk4)/lib/pkgconfig:$PKG_CONFIG_PATH"
```

## CI/CD Pipeline

The GitHub Actions workflow automatically:

1. Triggers on version tags (e.g., `v0.44.0`)
2. Builds for both x86_64 and aarch64 architectures
3. Creates DMG files for distribution
4. Uploads to GitHub Releases
5. Includes changelog from `CHANGELOG.md`

To trigger a release build:

```bash
git tag -a v0.44.0 -m "Release 0.44.0"
git push origin v0.44.0
```

## Performance Optimization

Built binaries include:
- Release optimizations (`opt-level = 3`)
- Link-time optimization (LTO)
- Single codegen unit for maximum optimization

This results in smaller, faster binaries suitable for distribution.
