# Windows Build and Packaging Guide

## Prerequisites

1. **MSYS2 with UCRT64 environment** installed at `C:\msys64`
2. **WiX Toolset** installed (for creating MSI installer)
3. **Rust toolchain** for Windows (MSVC target)

## Required MSYS2 Packages

Install the following packages in MSYS2 UCRT64 terminal:

```bash
pacman -S mingw-w64-ucrt-x86_64-gtk4 \
          mingw-w64-ucrt-x86_64-libadwaita \
          mingw-w64-ucrt-x86_64-gtksourceview5 \
          mingw-w64-ucrt-x86_64-gettext \
          mingw-w64-ucrt-x86_64-toolchain
```

## Build Process

### Step 1: Build the Release Binary

```powershell
# From project root
cargo build --release
```

This creates `target\release\gcodekit5.exe`

### Step 2: Bundle GTK4 Runtime

Run the bundling script to copy all required DLLs and data files:

```powershell
.\scripts\windows-bundle-gtk.ps1 -TargetDir "target\release" -MsysPath "C:\msys64\ucrt64"
```

This script will:
- Create `target\release\bin\` directory with all DLLs and the executable
- Copy GTK4, Libadwaita, and GtkSourceView5 DLLs
- Copy GLib schemas to `target\release\share\glib-2.0\schemas\`
- Copy icons to `target\release\share\icons\`
- Copy GtkSourceView language specs and styles
- Copy GDK-Pixbuf loaders

After bundling, verify the structure:
```
target\release\
├── bin\
│   ├── gcodekit5.exe
│   ├── libgtk-4-1.dll
│   ├── libadwaita-1-0.dll
│   ├── libgtksourceview-5-0.dll
│   └── ... (all other DLLs)
├── share\
│   ├── glib-2.0\schemas\
│   ├── icons\
│   └── gtksourceview-5\
└── lib\
    └── gdk-pixbuf-2.0\
```

### Step 3: Test the Bundle Locally

Before creating the installer, test the bundled application:

```powershell
cd target\release\bin
.\gcodekit5.exe
```

If you see errors about missing DLLs, check:
1. The DLL is in the `bin\` directory
2. The DLL exists in `C:\msys64\ucrt64\bin\`
3. Add missing DLLs to the bundling script if needed

### Step 4: Create MSI Installer

Using cargo-wix:

```powershell
cargo wix --nocapture
```

Or manually with WiX toolset:

```powershell
candle.exe -arch x64 -ext WixUIExtension wix\main.wxs
light.exe -ext WixUIExtension -out target\wix\gcodekit5.msi target\wix\main.wixobj
```

The MSI installer will include:
- The executable and all DLLs from `target\release\bin\`
- GTK4 data files from `target\release\share\`
- GDK-Pixbuf loaders from `target\release\lib\`

## Troubleshooting

### Missing DLL Errors

If the installed application shows "The code execution cannot proceed because [dll-name] was not found":

1. **Find the missing DLL** in MSYS2:
   ```powershell
   # In MSYS2 UCRT64 terminal
   find /ucrt64/bin -name "*missing-name*"
   ```

2. **Add to bundling script** in `scripts/windows-bundle-gtk.ps1`:
   ```powershell
   $DllsToCopy = @(
       # ... existing DLLs ...
       "libmissing-dll.dll",
   )
   ```

3. **Rebuild the bundle and installer**

### Common Missing DLLs

Based on typical GTK4/Adwaita applications, these are often missed:
- `libdatrie-1.dll` - For Thai text support
- `libthai-0.dll` - For Thai text support  
- `libgraphite2.dll` - For advanced font rendering
- `libunistring-5.dll` - Unicode string operations
- `libidn2-0.dll` - Internationalized domain names
- `libffi-7.dll` or `libffi-8.dll` - Foreign function interface
- `libcroco-0.6-3.dll` - CSS parsing (for SVG)

### DLL Dependency Analysis

To find all dependencies of a DLL or executable on Windows:

1. Use **Dependencies.exe** (modern dependency walker):
   ```
   https://github.com/lucasg/Dependencies
   ```

2. Open `target\release\bin\gcodekit5.exe` in Dependencies
3. Look for red entries (missing DLLs)
4. Add them to the bundling script

### Icon/Theme Issues

If icons don't appear:
1. Verify `target\release\share\icons\` contains Adwaita icons
2. Verify `target\release\share\glib-2.0\schemas\` has schema files
3. Run schema compilation if needed:
   ```powershell
   glib-compile-schemas.exe target\release\share\glib-2.0\schemas\
   ```

### GtkSourceView Language Support Missing

If syntax highlighting doesn't work:
1. Verify `target\release\share\gtksourceview-5\language-specs\` exists
2. Verify `target\release\share\gtksourceview-5\styles\` exists
3. Check that `.lang` and `.style` files are present

## Environment Variables

The application may need these environment variables set (usually automatic):
- `GDK_PIXBUF_MODULEDIR` - Points to pixbuf loaders
- `XDG_DATA_DIRS` - Points to share directory
- `GTK_THEME` - GTK theme name (optional)

## Distribution Checklist

Before distributing the MSI:

- [ ] Built with `--release` profile
- [ ] All DLLs bundled (no missing DLL errors)
- [ ] Icons and themes work correctly
- [ ] Syntax highlighting works in GCode editor
- [ ] Application starts without console window
- [ ] Tested on clean Windows 11 system
- [ ] Tested on clean Windows 10 system
- [ ] MSI installs to correct location
- [ ] Start menu shortcut works
- [ ] Uninstaller removes all files
- [ ] Version number is correct in installer

## File Size Expectations

A typical bundled GTK4 application with Libadwaita:
- Executable: 5-15 MB
- DLLs: 50-100 MB
- Data files (icons, schemas, etc.): 20-50 MB
- Total bundle: ~80-165 MB
- MSI installer: ~80-165 MB (compressed)

## Automation

Consider creating a PowerShell build script that combines all steps:

```powershell
# build-windows-release.ps1
param(
    [switch]$SkipBuild = $false
)

$ErrorActionPreference = "Stop"

if (-not $SkipBuild) {
    Write-Host "Building release binary..."
    cargo build --release
}

Write-Host "Bundling GTK4 runtime..."
.\scripts\windows-bundle-gtk.ps1

Write-Host "Testing bundle..."
$TestProcess = Start-Process -FilePath "target\release\bin\gcodekit5.exe" -PassThru
Start-Sleep -Seconds 5
if (-not $TestProcess.HasExited) {
    Write-Host "Application started successfully" -ForegroundColor Green
    $TestProcess.Kill()
} else {
    Write-Host "Application failed to start" -ForegroundColor Red
    exit 1
}

Write-Host "Creating MSI installer..."
cargo wix --nocapture

Write-Host "`nBuild complete!" -ForegroundColor Green
Write-Host "MSI location: target\wix\gcodekit5-*.msi"
```

## References

- GTK4 Windows guide: https://gtk-rs.org/gtk4-rs/stable/latest/book/installation_windows.html
- Libadwaita: https://gnome.pages.gitlab.gnome.org/libadwaita/
- WiX Toolset: https://wixtoolset.org/
- MSYS2: https://www.msys2.org/
