# Windows GTK4 Bundle Script
# Bundles GTK4, GtkSourceView5, and Libadwaita DLLs with the application

param(
    [string]$TargetDir = "target\release",
    [string]$MsysPath = "C:\msys64\ucrt64"
)

$ErrorActionPreference = "Stop"

Write-Host "Bundling GTK4 runtime for Windows..."

# Create bin directory structure
$BinDir = "$TargetDir\bin"
$ShareDir = "$TargetDir\share"
$LibDir = "$TargetDir\lib"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $ShareDir | Out-Null
New-Item -ItemType Directory -Force -Path $LibDir | Out-Null

# Core GTK4 DLLs
$DllsToCopy = @(
    # GTK4 core
    "libgtk-4-1.dll",
    "libgdk_pixbuf-2.0-0.dll",
    "libgio-2.0-0.dll",
    "libglib-2.0-0.dll",
    "libgobject-2.0-0.dll",
    "libgmodule-2.0-0.dll",
    "libpango-1.0-0.dll",
    "libpangocairo-1.0-0.dll",
    "libpangowin32-1.0-0.dll",
    "libcairo-2.dll",
    "libcairo-gobject-2.dll",
    "libcairo-script-interpreter-2.dll",
    "libgraphene-1.0-0.dll",
    "libharfbuzz-0.dll",
    "libfribidi-0.dll",
    "libfontconfig-1.dll",
    "libfreetype-6.dll",
    "libpixman-1-0.dll",
    "libpng16-16.dll",
    "libjpeg-8.dll",
    "libtiff-6.dll",
    "libwebp-7.dll",
    "libepoxy-0.dll",
    "libintl-8.dll",
    "libiconv-2.dll",
    "libpcre2-8-0.dll",
    "libffi-8.dll",
    "zlib1.dll",
    "libbz2-1.dll",
    "libbrotlidec.dll",
    "libbrotlicommon.dll",
    "libexpat-1.dll",
    "liblzma-5.dll",
    "libzstd.dll",
    "libdeflate.dll",
    "libLerc.dll",
    "libjbig-0.dll",
    "libsharpyuv-0.dll",
    "libwebpdemux-2.dll",
    "libwebpmux-3.dll",
    
    # GtkSourceView5
    "libgtksourceview-5-0.dll",
    
    # Libadwaita
    "libadwaita-1-0.dll",
    
    # Additional dependencies
    "librsvg-2-2.dll",
    "libxml2-2.dll",
    
    # Windows-specific
    "libwinpthread-1.dll",
    "libgcc_s_seh-1.dll",
    "libstdc++-6.dll",
    
    # Additional graphics and system libraries
    "libdatrie-1.dll",
    "libthai-0.dll",
    "libcroco-0.6-3.dll",
    "liblzo2-2.dll",
    "libgraphite2.dll",
    "libunistring-5.dll",
    "libidn2-0.dll",
    "libffi-7.dll",
    "libssp-0.dll",
    
    # GObject Introspection (if needed)
    "libgirepository-1.0-1.dll",
    
    # Additional image format support
    "libjasper-4.dll",
    "libgif-7.dll",
    
    # SSL/Crypto (may be needed for some operations)
    "libcrypto-3-x64.dll",
    "libssl-3-x64.dll"
)

Write-Host "Copying DLLs from $MsysPath\bin..."

foreach ($dll in $DllsToCopy) {
    $srcPath = "$MsysPath\bin\$dll"
    if (Test-Path $srcPath) {
        Copy-Item $srcPath -Destination $BinDir -Force
        Write-Host "  Copied: $dll"
    } else {
        Write-Host "  Warning: Not found: $dll" -ForegroundColor Yellow
    }
}

# Copy the executable to bin directory
$ExePath = "$TargetDir\gcodekit5.exe"
if (Test-Path $ExePath) {
    Copy-Item $ExePath -Destination $BinDir -Force
    Write-Host "  Copied: gcodekit5.exe"
}

# Copy GLib schemas
$SchemasDir = "$ShareDir\glib-2.0\schemas"
New-Item -ItemType Directory -Force -Path $SchemasDir | Out-Null
$SrcSchemas = "$MsysPath\share\glib-2.0\schemas"
if (Test-Path $SrcSchemas) {
    Copy-Item "$SrcSchemas\*" -Destination $SchemasDir -Recurse -Force
    Write-Host "Copied GLib schemas"
}

# Copy icons (Adwaita)
$IconsDir = "$ShareDir\icons"
New-Item -ItemType Directory -Force -Path $IconsDir | Out-Null
$AdwaitaIcons = "$MsysPath\share\icons\Adwaita"
if (Test-Path $AdwaitaIcons) {
    Copy-Item $AdwaitaIcons -Destination $IconsDir -Recurse -Force
    Write-Host "Copied Adwaita icons"
}

# Copy hicolor icons
$HicolorIcons = "$MsysPath\share\icons\hicolor"
if (Test-Path $HicolorIcons) {
    Copy-Item $HicolorIcons -Destination $IconsDir -Recurse -Force
    Write-Host "Copied hicolor icons"
}

# Copy GtkSourceView language specs and styles
$SourceViewDir = "$ShareDir\gtksourceview-5"
New-Item -ItemType Directory -Force -Path $SourceViewDir | Out-Null
$SrcSourceView = "$MsysPath\share\gtksourceview-5"
if (Test-Path $SrcSourceView) {
    Copy-Item "$SrcSourceView\*" -Destination $SourceViewDir -Recurse -Force
    Write-Host "Copied GtkSourceView5 data"
}

# Copy GDK-Pixbuf loaders
$PixbufLoadersDir = "$LibDir\gdk-pixbuf-2.0\2.10.0\loaders"
New-Item -ItemType Directory -Force -Path $PixbufLoadersDir | Out-Null
$SrcLoaders = "$MsysPath\lib\gdk-pixbuf-2.0\2.10.0\loaders"
if (Test-Path $SrcLoaders) {
    Copy-Item "$SrcLoaders\*.dll" -Destination $PixbufLoadersDir -Force
    Write-Host "Copied GDK-Pixbuf loaders"
}

# Copy loaders.cache
$LoadersCache = "$MsysPath\lib\gdk-pixbuf-2.0\2.10.0\loaders.cache"
if (Test-Path $LoadersCache) {
    Copy-Item $LoadersCache -Destination "$LibDir\gdk-pixbuf-2.0\2.10.0\" -Force
    Write-Host "Copied loaders.cache"
}

Write-Host "`nGTK4 bundle complete!" -ForegroundColor Green
Write-Host "Bundle location: $TargetDir"
