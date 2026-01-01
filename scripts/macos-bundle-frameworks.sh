#!/bin/bash
# macOS Framework Bundling Script
# This script bundles GTK4, SourceView, and all dependencies into the app bundle

set -e

TARGET_ARCH="${1:-aarch64-apple-darwin}"
APP_BUNDLE="GCodeKit.app"
FRAMEWORKS_DIR="${APP_BUNDLE}/Contents/Frameworks"
LIB_DIR="${APP_BUNDLE}/Contents/Libs"
RESOURCES_DIR="${APP_BUNDLE}/Contents/Resources"

echo "Bundling frameworks and libraries for $TARGET_ARCH..."

# Create necessary directories
mkdir -p "$FRAMEWORKS_DIR"
mkdir -p "$LIB_DIR"
mkdir -p "$RESOURCES_DIR/share/icons"
mkdir -p "$RESOURCES_DIR/share/glib-2.0"

# Get Homebrew installation path
BREW_PREFIX=$(brew --prefix)

# Function to copy framework and update install names
copy_framework() {
    local fw_name=$1
    local fw_path="${BREW_PREFIX}/opt/${fw_name}"
    
    if [ -d "${fw_path}/Frameworks/${fw_name}.framework" ]; then
        echo "Copying ${fw_name}.framework..."
        cp -r "${fw_path}/Frameworks/${fw_name}.framework" "$FRAMEWORKS_DIR/"
        # Fix install names
        update_install_names "${FRAMEWORKS_DIR}/${fw_name}.framework"
    fi
}

# Function to copy dylib and update install names
copy_dylib() {
    local lib_name=$1
    local lib_path="${BREW_PREFIX}/opt/${lib_name}/lib"
    
    if [ -d "$lib_path" ]; then
        echo "Copying libraries from ${lib_name}..."
        find "$lib_path" -name "*.dylib" -o -name "*.so" | while read lib; do
            if [ -f "$lib" ]; then
                cp -f "$lib" "$LIB_DIR/" || true
            fi
        done
    fi
}

# Function to recursively update install names in a framework/binary
update_install_names() {
    local path=$1
    local binary_name=$(basename "$path" .framework)
    
    if [ -f "${path}/${binary_name}" ]; then
        local current_id=$(otool -D "${path}/${binary_name}" | tail -1)
        if [[ "$current_id" == /* ]]; then
            install_name_tool -id "@rpath/${binary_name}.framework/${binary_name}" "${path}/${binary_name}" || true
        fi
    fi
    
    # Update dependencies
    if [ -f "${path}/${binary_name}" ]; then
        local deps=$(otool -L "${path}/${binary_name}" | grep -v ":" | awk '{print $1}' | grep -v "^$" | grep "$(echo $BREW_PREFIX | sed 's/\//\\\//g')")
        echo "$deps" | while read dep; do
            if [ -n "$dep" ]; then
                local base_name=$(basename "$dep")
                install_name_tool -change "$dep" "@rpath/${base_name}" "${path}/${binary_name}" || true
            fi
        done
    fi
}

# Copy main frameworks
echo "Copying GTK4 framework..."
cp -r "${BREW_PREFIX}/opt/gtk4/lib/pkgconfig" "$RESOURCES_DIR/" || true

# Copy key libraries
copy_dylib "gtk4"
copy_dylib "gtksourceview5"
copy_dylib "adwaita-icon-theme"
copy_dylib "glib"
copy_dylib "gdk-pixbuf"
copy_dylib "cairo"
copy_dylib "pango"
copy_dylib "harfbuzz"
copy_dylib "gobject-introspection"
copy_dylib "gio"
copy_dylib "graphene"
copy_dylib "libpng"
copy_dylib "jpeg"
copy_dylib "webp"
copy_dylib "fontconfig"
copy_dylib "freetype"

# Copy icon theme
if [ -d "${BREW_PREFIX}/share/icons/Adwaita" ]; then
    echo "Copying icon theme..."
    cp -r "${BREW_PREFIX}/share/icons/Adwaita" "$RESOURCES_DIR/share/icons/" || true
fi

# Copy GLib schemas
if [ -d "${BREW_PREFIX}/share/glib-2.0/schemas" ]; then
    echo "Copying GLib schemas..."
    cp -r "${BREW_PREFIX}/share/glib-2.0/schemas" "$RESOURCES_DIR/share/glib-2.0/" || true
fi

# Update rpath in main binary
echo "Updating rpath in main executable..."
BINARY="${APP_BUNDLE}/Contents/MacOS/gcodekit5"

# Add rpaths for library discovery
install_name_tool -add_rpath "@executable_path/../Libs" "$BINARY" || true
install_name_tool -add_rpath "@executable_path/../Frameworks" "$BINARY" || true

# Fix library references
echo "Fixing library references..."
find "$LIB_DIR" -name "*.dylib" | while read lib; do
    # Update the library's own install name
    base_name=$(basename "$lib")
    current_id=$(otool -D "$lib" 2>/dev/null | tail -1)
    if [[ "$current_id" == /* ]]; then
        install_name_tool -id "@rpath/${base_name}" "$lib" || true
    fi
done

# Fix cross-library dependencies
echo "Fixing inter-library dependencies..."
find "$LIB_DIR" -name "*.dylib" | while read lib; do
    deps=$(otool -L "$lib" 2>/dev/null | grep -v ":" | awk '{print $1}' | grep "$(echo $BREW_PREFIX | sed 's/\//\\\//g')" | head -20)
    echo "$deps" | while read dep; do
        if [ -n "$dep" ] && [ "$dep" != "@rpath" ]; then
            base_name=$(basename "$dep")
            install_name_tool -change "$dep" "@rpath/${base_name}" "$lib" 2>/dev/null || true
        fi
    done
done

echo "Framework bundling completed successfully!"
