# macOS Build Quick Reference

## Release Build (Automated via GitHub Actions)

To trigger an automated macOS build via GitHub Actions:

```bash
# Create a version tag
git tag -a v0.44.0 -m "Release version 0.44.0"

# Push to GitHub
git push origin v0.44.0
```

This automatically triggers builds for:
- macOS x86_64 (Intel)
- macOS ARM64 (Apple Silicon)

Resulting DMG files are uploaded to GitHub Releases:
- `gcodekit5-v0.44.0-macos-x86_64.dmg`
- `gcodekit5-v0.44.0-macos-arm64.dmg`

## Local macOS Build

### One-Time Setup

```bash
# Install dependencies
brew install gtk4 libgtksourceview5 adwaita-icon-theme librsvg libpng jpeg webp

# Set environment variables
export PKG_CONFIG_PATH="$(brew --prefix gtk4)/lib/pkgconfig:$(brew --prefix libgtksourceview5)/lib/pkgconfig:$PKG_CONFIG_PATH"
```

### Build & Package

```bash
# Build for current architecture
timeout 600 cargo build --release

# Create app bundle
mkdir -p "GCodeKit.app/Contents/MacOS"
mkdir -p "GCodeKit.app/Contents/Resources"

cp target/release/gcodekit5 "GCodeKit.app/Contents/MacOS/"
cp assets/Pictures/com.github.thawkins.gcodekit5.png "GCodeKit.app/Contents/Resources/AppIcon.png"

# Use workflow's Info.plist template, then:
bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin
bash scripts/macos-create-dmg.sh "v0.44.0" "macos-arm64"
```

## What's Bundled

✅ GTK4 framework and libraries
✅ SourceView5 syntax highlighting
✅ Adwaita icons and themes
✅ GLib, Cairo, Pango, HarfBuzz
✅ GdkPixbuf, fontconfig, freetype
✅ Image libraries (libpng, jpeg, webp)

## Output Files

- `GCodeKit.app/` - App bundle (can be run or archived)
- `gcodekit5-v0.44.0-macos-arm64.dmg` - Distribution DMG

## Distribution

Users can:
1. Download the DMG from GitHub Releases
2. Double-click to mount
3. Drag GCodeKit to Applications folder
4. Run from Applications

No additional dependencies needed - all libraries are bundled!

## Cross-Compilation

```bash
# Build for Apple Silicon from Intel Mac
rustup target add aarch64-apple-darwin
timeout 600 cargo build --release --target aarch64-apple-darwin
bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin

# Build for Intel from Apple Silicon Mac
rustup target add x86_64-apple-darwin
timeout 600 cargo build --release --target x86_64-apple-darwin
bash scripts/macos-bundle-frameworks.sh x86_64-apple-darwin
```

## Troubleshooting

**Library not found errors:**
```bash
otool -L "GCodeKit.app/Contents/MacOS/gcodekit5"
```

**Rebuild framework bundle:**
```bash
bash scripts/macos-bundle-frameworks.sh aarch64-apple-darwin
```

**Check GTK4 installation:**
```bash
brew --prefix gtk4
echo $PKG_CONFIG_PATH
```
