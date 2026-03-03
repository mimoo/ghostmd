#!/usr/bin/env bash
set -euo pipefail

# Build a macOS .app bundle for GhostMD.
# Usage: ./scripts/bundle-macos.sh [--release]

BUILD_TYPE="${1:---release}"
if [ "$BUILD_TYPE" = "--release" ]; then
    PROFILE="release"
    cargo build --release
else
    PROFILE="debug"
    cargo build
fi

APP_NAME="GhostMD"
BUNDLE_DIR="target/${APP_NAME}.app"
BINARY="target/${PROFILE}/ghostmd"

# Verify the binary exists
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    exit 1
fi

# Clean previous bundle
rm -rf "$BUNDLE_DIR"

# Create bundle structure
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

# Copy binary
cp "$BINARY" "$BUNDLE_DIR/Contents/MacOS/ghostmd"

# Create Info.plist
cat > "$BUNDLE_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>GhostMD</string>
    <key>CFBundleDisplayName</key>
    <string>GhostMD</string>
    <key>CFBundleIdentifier</key>
    <string>com.ghostmd.app</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>ghostmd</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Copy icon if it exists
if [ -f "assets/AppIcon.icns" ]; then
    cp "assets/AppIcon.icns" "$BUNDLE_DIR/Contents/Resources/AppIcon.icns"
fi

echo "Built ${BUNDLE_DIR}"
echo "To install: cp -r ${BUNDLE_DIR} /Applications/"
