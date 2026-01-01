#!/bin/bash
# macOS DMG Creation Script
# Creates a distributable DMG file with the application

set -e

VERSION="${1:-1.0.0}"
ARCH="${2:-macos-arm64}"

APP_BUNDLE="GCodeKit.app"
DMG_NAME="gcodekit5-${VERSION}-${ARCH}.dmg"
TEMP_DMG="gcodekit5-temp-${ARCH}.dmg"
MOUNT_POINT="/Volumes/GCodeKit-${ARCH}"

echo "Creating DMG installer: $DMG_NAME"

# Remove any existing DMG files
rm -f "$DMG_NAME" "$TEMP_DMG"

# Create a temporary DMG with 500MB size (adjust as needed)
hdiutil create -volname "GCodeKit" -srcfolder "$APP_BUNDLE" -ov -format UDRW -size 500m "$TEMP_DMG"

# Attach the DMG
hdiutil attach "$TEMP_DMG" -mountpoint "$MOUNT_POINT"

# Create a symbolic link to Applications folder for easy drag-drop install
ln -s /Applications "$MOUNT_POINT/Applications" || true

# Set icon view and position windows
osascript << EOF
tell application "Finder"
    activate
    set theWindow to (open (the POSIX file "${MOUNT_POINT}"))
    tell theWindow
        set current view to icon view
        set bounds to {0, 0, 640, 480}
        set icon size to 100
        set arrangement to not arranged
    end tell
    
    -- Position the app icon
    tell theWindow
        tell item 1 of theWindow
            set position to {100, 100}
        end tell
        tell item 2 of theWindow
            set position to {400, 100}
        end tell
    end tell
end tell
EOF

sleep 2

# Detach the DMG
hdiutil detach "$MOUNT_POINT"

# Convert to compressed DMG
echo "Compressing DMG..."
hdiutil convert "$TEMP_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_NAME"

# Clean up temporary DMG
rm -f "$TEMP_DMG"

echo "DMG created successfully: $DMG_NAME"
ls -lh "$DMG_NAME"
