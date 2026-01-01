# DLL Dependency Checker for GCodeKit5 Windows Build
# Scans the bundled application for missing DLL dependencies

param(
    [string]$BundlePath = "target\release\bin",
    [string]$MsysPath = "C:\msys64\ucrt64\bin"
)

$ErrorActionPreference = "Continue"

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "    DLL Dependency Checker           " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Check if bundle exists
if (-not (Test-Path $BundlePath)) {
    Write-Host "Error: Bundle path not found: $BundlePath" -ForegroundColor Red
    Write-Host "Run the bundling script first: .\scripts\windows-bundle-gtk.ps1" -ForegroundColor Yellow
    exit 1
}

$ExePath = Join-Path $BundlePath "gcodekit5.exe"
if (-not (Test-Path $ExePath)) {
    Write-Host "Error: gcodekit5.exe not found in bundle" -ForegroundColor Red
    exit 1
}

Write-Host "Checking bundle: $BundlePath" -ForegroundColor White
Write-Host ""

# Get all DLLs in bundle
$BundledDlls = Get-ChildItem -Path $BundlePath -Filter "*.dll" | Select-Object -ExpandProperty Name
Write-Host "Found $($BundledDlls.Count) DLLs in bundle" -ForegroundColor Green
Write-Host ""

# Function to get DLL dependencies (simplified - requires dumpbin or similar)
function Get-DllDependencies {
    param($FilePath)
    
    # Try using dumpbin if available (Visual Studio)
    $Dumpbin = Get-Command dumpbin -ErrorAction SilentlyContinue
    if ($Dumpbin) {
        $Output = & dumpbin /dependents $FilePath 2>&1 | Select-String "\.dll"
        return $Output | ForEach-Object { 
            if ($_ -match '(\S+\.dll)') { 
                $Matches[1] 
            }
        }
    }
    
    return @()
}

# Try to analyze with dumpbin
$Dumpbin = Get-Command dumpbin -ErrorAction SilentlyContinue
if (-not $Dumpbin) {
    Write-Host "Note: dumpbin not found (install Visual Studio for detailed analysis)" -ForegroundColor Yellow
    Write-Host "Using basic file checks instead..." -ForegroundColor Yellow
    Write-Host ""
}

# Check for common missing DLLs
$CommonDlls = @(
    # Core GTK4
    "libgtk-4-1.dll",
    "libgdk_pixbuf-2.0-0.dll",
    "libgio-2.0-0.dll",
    "libglib-2.0-0.dll",
    "libgobject-2.0-0.dll",
    
    # Adwaita
    "libadwaita-1-0.dll",
    
    # SourceView
    "libgtksourceview-5-0.dll",
    
    # Graphics
    "libcairo-2.dll",
    "libpango-1.0-0.dll",
    "libharfbuzz-0.dll",
    "libfreetype-6.dll",
    "libpng16-16.dll",
    
    # System
    "libintl-8.dll",
    "libiconv-2.dll",
    "libwinpthread-1.dll",
    "libgcc_s_seh-1.dll",
    "libstdc++-6.dll",
    
    # Additional
    "libgraphite2.dll",
    "libunistring-5.dll",
    "libidn2-0.dll",
    "libffi-8.dll",
    "zlib1.dll"
)

Write-Host "Checking common required DLLs..." -ForegroundColor Yellow
$MissingDlls = @()
$FoundDlls = @()

foreach ($dll in $CommonDlls) {
    $InBundle = $BundledDlls -contains $dll
    $InMsys = Test-Path (Join-Path $MsysPath $dll)
    
    if ($InBundle) {
        Write-Host "  ✓ $dll" -ForegroundColor Green
        $FoundDlls += $dll
    } else {
        Write-Host "  ✗ $dll" -ForegroundColor Red
        $MissingDlls += $dll
        
        if ($InMsys) {
            Write-Host "    (available in MSYS2)" -ForegroundColor Yellow
        } else {
            Write-Host "    (NOT found in MSYS2 - may need different name)" -ForegroundColor Magenta
        }
    }
}

Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "             Summary                  " -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Total DLLs in bundle: $($BundledDlls.Count)" -ForegroundColor White
Write-Host "Common DLLs found:    $($FoundDlls.Count)/$($CommonDlls.Count)" -ForegroundColor White
Write-Host ""

if ($MissingDlls.Count -eq 0) {
    Write-Host "✓ All common DLLs are present!" -ForegroundColor Green
} else {
    Write-Host "⚠ Missing $($MissingDlls.Count) common DLLs" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "To fix, add these to scripts/windows-bundle-gtk.ps1:" -ForegroundColor Yellow
    Write-Host ""
    foreach ($dll in $MissingDlls) {
        Write-Host "    `"$dll`"," -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "To test the bundle, run:" -ForegroundColor Cyan
Write-Host "    .\$BundlePath\gcodekit5.exe" -ForegroundColor White
Write-Host ""

# Try to detect if Dependencies.exe is available
$DependenciesPath = "C:\Program Files\Dependencies\Dependencies.exe"
if (Test-Path $DependenciesPath) {
    Write-Host "For detailed analysis, run Dependencies.exe:" -ForegroundColor Cyan
    Write-Host "    & '$DependenciesPath' '$ExePath'" -ForegroundColor White
} else {
    Write-Host "For detailed analysis, install Dependencies:" -ForegroundColor Cyan
    Write-Host "    https://github.com/lucasg/Dependencies" -ForegroundColor White
}

Write-Host ""
