# Windows Release Build Script
# Builds GCodeKit5 for Windows, bundles GTK4 runtime, and creates MSI installer

param(
    [switch]$SkipBuild = $false,
    [switch]$SkipBundle = $false,
    [switch]$SkipTest = $false,
    [string]$MsysPath = "C:\msys64\ucrt64",
    [string]$WixPath = "C:\Program Files (x86)\WiX Toolset v3.11\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  GCodeKit5 Windows Release Builder  " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Get version from Cargo.toml
$CargoToml = Get-Content "Cargo.toml" -Raw
if ($CargoToml -match 'version\s*=\s*"([^"]+)"') {
    $Version = $Matches[1]
    Write-Host "Building version: $Version" -ForegroundColor Green
} else {
    Write-Host "Error: Could not determine version from Cargo.toml" -ForegroundColor Red
    exit 1
}

# Step 1: Build release binary
if (-not $SkipBuild) {
    Write-Host ""
    Write-Host "Step 1: Building release binary..." -ForegroundColor Yellow
    Write-Host "This may take several minutes..." -ForegroundColor Gray
    
    $BuildStart = Get-Date
    $BuildProcess = Start-Process -FilePath "cargo" -ArgumentList "build", "--release" -NoNewWindow -Wait -PassThru
    $BuildEnd = Get-Date
    $BuildTime = ($BuildEnd - $BuildStart).TotalSeconds
    
    if ($BuildProcess.ExitCode -ne 0) {
        Write-Host "Build failed with exit code $($BuildProcess.ExitCode)" -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✓ Build completed in $([math]::Round($BuildTime, 1)) seconds" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Step 1: Skipping build (using existing binary)" -ForegroundColor Yellow
}

# Verify executable exists
if (-not (Test-Path "target\release\gcodekit5.exe")) {
    Write-Host "Error: gcodekit5.exe not found in target\release\" -ForegroundColor Red
    exit 1
}

# Step 2: Bundle GTK4 runtime
if (-not $SkipBundle) {
    Write-Host ""
    Write-Host "Step 2: Bundling GTK4 runtime..." -ForegroundColor Yellow
    
    # Clean existing bundle
    if (Test-Path "target\release\bin") {
        Write-Host "Cleaning existing bundle..." -ForegroundColor Gray
        Remove-Item -Path "target\release\bin" -Recurse -Force
    }
    if (Test-Path "target\release\share") {
        Remove-Item -Path "target\release\share" -Recurse -Force
    }
    if (Test-Path "target\release\lib") {
        Remove-Item -Path "target\release\lib" -Recurse -Force
    }
    
    # Run bundling script
    & ".\scripts\windows-bundle-gtk.ps1" -TargetDir "target\release" -MsysPath $MsysPath
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Bundling failed" -ForegroundColor Red
        exit 1
    }
    
    # Count bundled files
    $DllCount = (Get-ChildItem "target\release\bin\*.dll" -ErrorAction SilentlyContinue).Count
    Write-Host "✓ Bundled $DllCount DLL files" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Step 2: Skipping bundle (using existing)" -ForegroundColor Yellow
}

# Step 3: Test the bundle
if (-not $SkipTest) {
    Write-Host ""
    Write-Host "Step 3: Testing bundled application..." -ForegroundColor Yellow
    
    $TestProcess = Start-Process -FilePath "target\release\bin\gcodekit5.exe" -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 3
    
    if ($TestProcess.HasExited) {
        Write-Host "✗ Application exited immediately (exit code: $($TestProcess.ExitCode))" -ForegroundColor Red
        Write-Host "Check for missing DLL errors" -ForegroundColor Yellow
        exit 1
    } else {
        Write-Host "✓ Application started successfully" -ForegroundColor Green
        $TestProcess.Kill()
        Start-Sleep -Seconds 1
    }
} else {
    Write-Host ""
    Write-Host "Step 3: Skipping test" -ForegroundColor Yellow
}

# Step 4: Generate WiX fragments using heat.exe
Write-Host ""
Write-Host "Step 4: Generating WiX installer fragments..." -ForegroundColor Yellow

# Check if heat.exe is available
$HeatExe = if (Test-Path "$WixPath\heat.exe") { "$WixPath\heat.exe" } else { "heat.exe" }

try {
    # Test if heat.exe is accessible
    $null = & $HeatExe -? 2>&1
} catch {
    Write-Host "Warning: heat.exe not found. Install WiX Toolset to create MSI installer" -ForegroundColor Yellow
    Write-Host "Download from: https://wixtoolset.org/" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Bundle is ready at: target\release\" -ForegroundColor Green
    exit 0
}

# Create wix directory if it doesn't exist
if (-not (Test-Path "wix\generated")) {
    New-Item -ItemType Directory -Path "wix\generated" -Force | Out-Null
}

# Harvest DLL files from bin directory
Write-Host "  Harvesting DLL files..." -ForegroundColor Gray
& $HeatExe dir "target\release\bin" -nologo `
    -cg GtkRuntimeFiles `
    -gg -sfrag -srd -sreg `
    -dr Bin `
    -var "var.BinSource" `
    -out "wix\generated\gtk-dlls.wxs"

# Harvest share directory
if (Test-Path "target\release\share") {
    Write-Host "  Harvesting share files..." -ForegroundColor Gray
    & $HeatExe dir "target\release\share" -nologo `
        -cg GtkShareFiles `
        -gg -sfrag -srd -sreg `
        -dr Share `
        -var "var.ShareSource" `
        -out "wix\generated\gtk-share.wxs"
}

# Harvest lib directory
if (Test-Path "target\release\lib") {
    Write-Host "  Harvesting lib files..." -ForegroundColor Gray
    & $HeatExe dir "target\release\lib" -nologo `
        -cg GtkLibFiles `
        -gg -sfrag -srd -sreg `
        -dr Lib `
        -var "var.LibSource" `
        -out "wix\generated\gtk-lib.wxs"
}

Write-Host "✓ WiX fragments generated" -ForegroundColor Green

# Step 5: Create updated main.wxs that includes the fragments
Write-Host ""
Write-Host "Step 5: Building MSI installer..." -ForegroundColor Yellow

# Check if candle.exe and light.exe are available
$CandleExe = if (Test-Path "$WixPath\candle.exe") { "$WixPath\candle.exe" } else { "candle.exe" }
$LightExe = if (Test-Path "$WixPath\light.exe") { "$WixPath\light.exe" } else { "light.exe" }

# Compile WiX sources
Write-Host "  Compiling WiX sources..." -ForegroundColor Gray
& $CandleExe -nologo -arch x64 `
    -dVersion=$Version `
    -dProfile="release" `
    -dBinSource="target\release\bin" `
    -dShareSource="target\release\share" `
    -dLibSource="target\release\lib" `
    -ext WixUIExtension `
    -out "target\wix\\" `
    "wix\main.wxs" `
    "wix\generated\gtk-dlls.wxs" `
    "wix\generated\gtk-share.wxs" `
    "wix\generated\gtk-lib.wxs"

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ WiX compilation failed" -ForegroundColor Red
    exit 1
}

# Link to create MSI
Write-Host "  Linking MSI..." -ForegroundColor Gray
$MsiFile = "target\wix\gcodekit5-$Version-x64.msi"

& $LightExe -nologo `
    -ext WixUIExtension `
    -cultures:en-us `
    -out $MsiFile `
    "target\wix\main.wixobj" `
    "target\wix\gtk-dlls.wixobj" `
    "target\wix\gtk-share.wixobj" `
    "target\wix\gtk-lib.wixobj"

if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ MSI linking failed" -ForegroundColor Red
    exit 1
}

# Get file size
$MsiSize = [math]::Round((Get-Item $MsiFile).Length / 1MB, 2)
Write-Host "✓ MSI created: $MsiSize MB" -ForegroundColor Green

# Final summary
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "           Build Complete!            " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Version:  $Version" -ForegroundColor White
Write-Host "MSI File: $MsiFile" -ForegroundColor White
Write-Host "Size:     $MsiSize MB" -ForegroundColor White
Write-Host ""
Write-Host "Distribution Checklist:" -ForegroundColor Yellow
Write-Host "  [ ] Test installation on clean Windows 11" -ForegroundColor Gray
Write-Host "  [ ] Test installation on clean Windows 10" -ForegroundColor Gray
Write-Host "  [ ] Verify application starts without errors" -ForegroundColor Gray
Write-Host "  [ ] Verify icons and themes load correctly" -ForegroundColor Gray
Write-Host "  [ ] Verify syntax highlighting works" -ForegroundColor Gray
Write-Host "  [ ] Test uninstaller removes all files" -ForegroundColor Gray
Write-Host ""
