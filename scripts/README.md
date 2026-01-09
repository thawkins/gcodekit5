# Build Scripts

This directory contains platform-specific build and packaging scripts for GCodeKit5.

## Windows Scripts

### `build-windows-release.ps1` ⭐ **Recommended**

Complete automated build script for Windows. This is the easiest way to create a Windows release.

```powershell
.\scripts\build-windows-release.ps1
```

**What it does:**
1. Builds the release binary (`cargo build --release`)
2. Bundles GTK4 runtime using `windows-bundle-gtk.ps1`
3. Tests the bundled application
4. Generates WiX installer fragments with `heat.exe`
5. Compiles and links the MSI installer

**Options:**
- `-SkipBuild` - Skip cargo build, use existing binary
- `-SkipBundle` - Skip GTK bundling, use existing bundle
- `-SkipTest` - Skip application test
- `-MsysPath` - Custom MSYS2 path (default: `C:\msys64\ucrt64`)
- `-WixPath` - Custom WiX Toolset path

**Output:** `target\wix\gcodekit5-<version>-x64.msi`

---

### `windows-bundle-gtk.ps1`

Bundles GTK4, Libadwaita, and GtkSourceView5 runtime files with the application.

```powershell
.\scripts\windows-bundle-gtk.ps1 -TargetDir "target\release" -MsysPath "C:\msys64\ucrt64"
```

**What it does:**
- Copies 70+ DLL files from MSYS2 to `target\release\bin\`
- Copies GLib schemas to `target\release\share\glib-2.0\schemas\`
- Copies icons (Adwaita, hicolor) to `target\release\share\icons\`
- Copies GtkSourceView language specs and styles
- Copies GDK-Pixbuf loaders to `target\release\lib\`

**When to use:**
- Testing the bundle before creating MSI
- Manual build process
- Called automatically by `build-windows-release.ps1`

---

### `check-dll-dependencies.ps1`

Diagnostic tool to verify all required DLLs are present in the bundle.

```powershell
.\scripts\check-dll-dependencies.ps1
```

**What it does:**
- Lists all DLLs in the bundle
- Checks for common required GTK4/Adwaita DLLs
- Reports missing DLLs with suggestions
- Provides guidance on using detailed analysis tools

**When to use:**
- After bundling, before creating MSI
- Troubleshooting missing DLL errors
- Verifying bundle completeness

---

## macOS Scripts

### `macos-bundle-frameworks.sh`

Bundles GTK4 frameworks for macOS application bundle.

```bash
./scripts/macos-bundle-frameworks.sh
```

See [docs/MACOS_BUILD.md](../docs/MACOS_BUILD.md) for details.

---

### `macos-create-dmg.sh`

Creates a DMG installer for macOS.

```bash
./scripts/macos-create-dmg.sh
```

---

## Linux/Unix Scripts

### `start-grblhal-sim.sh`

Starts the grblHAL simulator with a virtual TTY device for testing CNC communication.

```bash
./scripts/start-grblhal-sim.sh
```

**What it does:**
- Creates a virtual serial port at `target/temp/ttyGRBL`
- Starts the grblHAL simulator connected to the virtual port
- Logs step and block output to `target/temp/`
- Allows your application to connect as if to real hardware

**To stop:**
```bash
./scripts/stop-grblhal-sim.sh
# or
killall socat
```

**To connect from your application:**
- Use the path: `~/Projects/gcodekit5/target/temp/ttyGRBL`
- Baud rate: Any (virtual serial, speed doesn't matter)
- The simulator responds to standard GRBL/grblHAL commands

---

### `stop-grblhal-sim.sh`

Stops the grblHAL simulator virtual TTY device.

```bash
./scripts/stop-grblhal-sim.sh
```

---

### `update-po.sh`

Updates translation files (.po) from source code.

```bash
./scripts/update-po.sh
```

**What it does:**
- Extracts translatable strings from Rust source
- Updates `.pot` template
- Updates all `.po` language files

---

## Python Scripts

### `generate_test_stl.py`

Generates test STL files for development and testing.

```bash
python scripts/generate_test_stl.py
```

---

## Windows Build Quick Reference

### First Time Setup

1. Install prerequisites:
   - Rust (MSVC toolchain)
   - MSYS2 with UCRT64 environment
   - WiX Toolset v3.11+

2. Install MSYS2 packages:
   ```bash
   pacman -S mingw-w64-ucrt-x86_64-gtk4 \
             mingw-w64-ucrt-x86_64-libadwaita \
             mingw-w64-ucrt-x86_64-gtksourceview5
   ```

### Create Release

```powershell
# Complete build (recommended)
.\scripts\build-windows-release.ps1

# Or step-by-step
cargo build --release
.\scripts\windows-bundle-gtk.ps1
.\scripts\check-dll-dependencies.ps1
# Then use WiX tools to create MSI
```

### Troubleshooting

If you see missing DLL errors:

1. **Check what's missing:**
   ```powershell
   .\scripts\check-dll-dependencies.ps1
   ```

2. **Find the DLL in MSYS2:**
   ```powershell
   dir C:\msys64\ucrt64\bin\*missing*.dll
   ```

3. **Add to bundling script:**
   Edit `windows-bundle-gtk.ps1`, add DLL to `$DllsToCopy` array

4. **Rebuild:**
   ```powershell
   .\scripts\build-windows-release.ps1 -SkipBuild
   ```

---

## Documentation

- Windows Build Guide: [docs/WINDOWS_BUILD.md](../docs/WINDOWS_BUILD.md)
- Windows Quick Start: [docs/WINDOWS_QUICK_START.md](../docs/WINDOWS_QUICK_START.md)
- macOS Build Guide: [docs/MACOS_BUILD.md](../docs/MACOS_BUILD.md)

---

## Contributing

When adding new dependencies that require additional DLLs:

1. Test on Windows and identify missing DLLs
2. Add DLLs to `windows-bundle-gtk.ps1`
3. Update documentation
4. Test the complete build process
5. Submit PR with all changes

---

## License

All scripts in this directory are part of GCodeKit5 and licensed under MIT OR Apache-2.0.
