# Windows Build Quick Start

## One-Command Build

From PowerShell in the project root:

```powershell
.\scripts\build-windows-release.ps1
```

This will:
1. Build the release binary
2. Bundle all GTK4/Libadwaita DLLs
3. Test the bundled application
4. Generate WiX installer fragments
5. Create the MSI installer

## Options

```powershell
# Skip the cargo build (use existing binary)
.\scripts\build-windows-release.ps1 -SkipBuild

# Skip bundling (use existing bundle)
.\scripts\build-windows-release.ps1 -SkipBundle

# Skip the test phase
.\scripts\build-windows-release.ps1 -SkipTest

# Custom MSYS2 path
.\scripts\build-windows-release.ps1 -MsysPath "D:\msys64\ucrt64"

# Custom WiX path
.\scripts\build-windows-release.ps1 -WixPath "C:\WiX\bin"
```

## Prerequisites

1. **Rust toolchain** (MSVC target)
2. **MSYS2** with UCRT64 environment at `C:\msys64`
3. **WiX Toolset** v3.11 or later

### Install MSYS2 Packages

```bash
pacman -S mingw-w64-ucrt-x86_64-gtk4 \
          mingw-w64-ucrt-x86_64-libadwaita \
          mingw-w64-ucrt-x86_64-gtksourceview5
```

## Output

The MSI installer will be created at:
```
target\wix\gcodekit5-<version>-x64.msi
```

## Troubleshooting

### Missing DLLs

If the application shows missing DLL errors after installation:

1. Find the DLL in MSYS2: `C:\msys64\ucrt64\bin\<dll-name>`
2. Add it to `scripts/windows-bundle-gtk.ps1` in the `$DllsToCopy` array
3. Re-run the build script

### Common Missing DLLs

Check [docs/WINDOWS_BUILD.md](WINDOWS_BUILD.md#common-missing-dlls) for a list of commonly missed dependencies.

### WiX Not Found

Download and install from: https://wixtoolset.org/

Make sure the WiX bin directory is in your PATH, or use the `-WixPath` parameter.

## Distribution

Before distributing the MSI:

1. Test on clean Windows 11 VM
2. Test on clean Windows 10 VM  
3. Verify all functionality works
4. Check file size is reasonable (~80-165 MB)

## See Also

- [docs/WINDOWS_BUILD.md](WINDOWS_BUILD.md) - Complete build guide
- [scripts/windows-bundle-gtk.ps1](../scripts/windows-bundle-gtk.ps1) - GTK bundling script
- [wix/main.wxs](../wix/main.wxs) - WiX installer configuration
