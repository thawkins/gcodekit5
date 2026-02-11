# Windows Missing DLL Fix Summary

## Problem

When installing the GCodeKit5 MSI on Windows 11, users were seeing errors like:
```
The code execution cannot proceed because libadwaita-1-0.dll was not found.
```

This occurred because the MSI installer was only packaging the executable, not the required GTK4 runtime DLLs.

## Root Causes

1. **Incomplete bundling script** - Some required DLLs were not included in the list
2. **WiX installer not configured** - The `wix/main.wxs` file only packaged the `.exe`, not the DLLs
3. **No automated build process** - Manual steps were error-prone

## Solution Implemented

### 1. Updated Bundling Script

**File:** `scripts/windows-bundle-gtk.ps1`

Added missing DLL dependencies:
- `libdatrie-1.dll` - Thai text support
- `libthai-0.dll` - Thai text support
- `libcroco-0.6-3.dll` - CSS parsing for SVG
- `liblzo2-2.dll` - Compression
- `libgraphite2.dll` - Advanced font rendering
- `libunistring-5.dll` - Unicode operations
- `libidn2-0.dll` - Internationalized domain names
- `libffi-7.dll` - Foreign function interface
- `libssp-0.dll` - Stack smashing protection
- `libgirepository-1.0-1.dll` - GObject Introspection
- `libjasper-4.dll` - JPEG2000 support
- `libgif-7.dll` - GIF image support
- `libcrypto-3-x64.dll` - Cryptography
- `libssl-3-x64.dll` - SSL/TLS

### 2. Updated WiX Installer

**File:** `wix/main.wxs`

Modified to:
- Include references to generated WiX fragments
- Add component groups for GTK runtime files
- Properly structure directories for `bin/`, `share/`, and `lib/`

Key changes:
- Moved executable to `target\release\bin\gcodekit5.exe` (from `target\release\gcodekit5.exe`)
- Added `ComponentGroupRef` entries for harvested DLLs and data files
- Added proper directory structure for GTK runtime

### 3. Created Automated Build Script

**File:** `scripts/build-windows-release.ps1`

Complete automation script that:
1. Builds the release binary with `cargo build --release`
2. Runs the GTK bundling script
3. Tests the bundled application
4. Uses WiX `heat.exe` to harvest all DLLs and data files
5. Compiles WiX sources with `candle.exe`
6. Links MSI installer with `light.exe`
7. Provides detailed progress and validation

Benefits:
- Eliminates manual steps
- Ensures consistency
- Validates each stage
- Provides clear error messages

### 4. Created Documentation

**Files:**
- `docs/WINDOWS_BUILD.md` - Complete build and troubleshooting guide
- `docs/WINDOWS_QUICK_START.md` - Quick reference for building

Documentation includes:
- Prerequisites and setup
- Step-by-step build process
- Troubleshooting for missing DLLs
- Distribution checklist
- Common issues and solutions

## How It Works Now

### Old Process (Broken)
1. Run `cargo build --release`
2. Manually run bundling script
3. WiX only packages the `.exe`
4. **Result:** Missing DLLs on installation

### New Process (Fixed)
1. Run `.\scripts\build-windows-release.ps1`
2. Script builds binary
3. Script bundles all GTK runtime files to `target\release\bin\`, `share\`, `lib\`
4. Script uses `heat.exe` to automatically discover all files
5. Script compiles WiX with all discovered files
6. **Result:** Complete MSI with all dependencies

## File Structure After Build

```
target/
└── release/
    ├── bin/
    │   ├── gcodekit5.exe
    │   ├── libgtk-4-1.dll
    │   ├── libadwaita-1-0.dll
    │   ├── libgtksourceview-5-0.dll
    │   └── ... (70+ DLLs)
    ├── share/
    │   ├── glib-2.0/schemas/
    │   ├── icons/
    │   └── gtksourceview-5/
    └── lib/
        └── gdk-pixbuf-2.0/
```

## Testing the Fix

### On Development Machine

```powershell
# Build and bundle
.\scripts\build-windows-release.ps1

# Test the bundle directly
.\target\release\bin\gcodekit5.exe

# Install the MSI
.\target\wix\gcodekit5-*-x64.msi
```

### On Clean Windows VM

1. Copy MSI to clean Windows 11 VM
2. Install MSI
3. Run from Start Menu
4. Verify:
   - Application starts without DLL errors
   - Icons load correctly
   - Syntax highlighting works
   - All features function

## Adding New DLL Dependencies

If a new DLL error appears:

1. **Find the DLL** in MSYS2:
   ```powershell
   dir C:\msys64\ucrt64\bin\*missing*.dll
   ```

2. **Add to bundling script** (`scripts/windows-bundle-gtk.ps1`):
   ```powershell
   $DllsToCopy = @(
       # ... existing DLLs ...
       "libnewdependency.dll",
   )
   ```

3. **Rebuild**:
   ```powershell
   .\scripts\build-windows-release.ps1
   ```

4. **Test** the new MSI

## Benefits of This Solution

1. **Automated** - One command builds everything
2. **Complete** - All dependencies included automatically
3. **Maintainable** - Easy to add new DLLs
4. **Validated** - Tests at each stage
5. **Documented** - Clear instructions for contributors

## File Changes Summary

| File | Change | Purpose |
|------|--------|---------|
| `scripts/windows-bundle-gtk.ps1` | Added 14 missing DLLs | Include all GTK4 dependencies |
| `wix/main.wxs` | Added ComponentGroupRef entries | Package bundled DLLs in MSI |
| `scripts/build-windows-release.ps1` | New file | Automate entire build process |
| `docs/WINDOWS_BUILD.md` | New file | Complete build documentation |
| `docs/WINDOWS_QUICK_START.md` | New file | Quick reference guide |

## Next Steps

1. **Test the build** on your Windows development machine
2. **Test installation** on a clean Windows 11 VM
3. **Document any additional missing DLLs** encountered
4. **Update this summary** if new dependencies are needed

## References

- WiX Toolset: https://wixtoolset.org/
- GTK Windows Guide: https://www.gtk.org/docs/installations/windows/
- MSYS2: https://www.msys2.org/
