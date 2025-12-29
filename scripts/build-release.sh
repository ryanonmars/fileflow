#!/bin/bash
set -e

# Load .env file if it exists
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

# Verify signing key is set
if [ -z "$TAURI_SIGNING_PRIVATE_KEY" ]; then
    echo "ERROR: TAURI_SIGNING_PRIVATE_KEY is not set!"
    echo "Please set it in .env file or export it:"
    echo "  export TAURI_SIGNING_PRIVATE_KEY=\"your-key\""
    exit 1
fi

echo "✓ Signing key is set"
echo "Building release..."

# Run the build
npm run tauri:build

# Sign the app bundle
echo ""
echo "Signing app bundle..."
codesign --force --deep --sign - src-tauri/target/release/bundle/macos/FileFlow.app
codesign --verify --verbose src-tauri/target/release/bundle/macos/FileFlow.app
echo "✓ App signed and verified"

# Rebuild DMG with the signed app inside
echo ""
echo "Rebuilding DMG with signed app..."
APP_PATH="src-tauri/target/release/bundle/macos/FileFlow.app"
DMG_PATH="src-tauri/target/release/bundle/dmg/FileFlow.dmg"
TEMP_DMG="/tmp/FileFlow_rebuild.dmg"

# Convert DMG to read-write
hdiutil convert "$DMG_PATH" -format UDRW -o "$TEMP_DMG" 2>&1 | tail -3

# Mount it
MOUNT_POINT="/Volumes/FileFlow"
hdiutil detach "$MOUNT_POINT" -force 2>/dev/null || true
hdiutil attach "$TEMP_DMG" -mountpoint "$MOUNT_POINT" -nobrowse 2>&1 | tail -3
sleep 2

# Replace app with signed version
rm -rf "$MOUNT_POINT/FileFlow.app"
cp -R "$APP_PATH" "$MOUNT_POINT/FileFlow.app"

# Re-sign app inside DMG
codesign --force --deep --sign - "$MOUNT_POINT/FileFlow.app"

# Unmount
hdiutil detach "$MOUNT_POINT" -force 2>&1 | tail -2

# Convert back to compressed DMG
rm -f "$DMG_PATH"
hdiutil convert "$TEMP_DMG" -format UDZO -o "$DMG_PATH" 2>&1 | tail -3
rm -f "$TEMP_DMG"
echo "✓ DMG rebuilt with signed app"

echo ""
echo "✓ Build complete!"
echo ""
echo "Files created:"
echo "  - DMG: src-tauri/target/release/bundle/dmg/FileFlow.dmg"
echo "  - App bundle: src-tauri/target/release/bundle/macos/FileFlow.app.tar.gz"
echo "  - Signature: src-tauri/target/release/bundle/macos/FileFlow.app.tar.gz.sig"
echo ""
echo "Checking for latest.json..."
LATEST_JSON="src-tauri/target/release/bundle/macos/latest.json"
TAR_GZ="src-tauri/target/release/bundle/macos/FileFlow.app.tar.gz"
SIG="src-tauri/target/release/bundle/macos/FileFlow.app.tar.gz.sig"

if [ ! -f "$TAR_GZ" ] || [ ! -f "$SIG" ]; then
    echo "  ERROR: Cannot generate latest.json - missing tar.gz or sig file"
    exit 1
fi

# Always regenerate latest.json to ensure it has the correct version
echo "  Regenerating latest.json with current version..."

# Get version from tauri.conf.json
VERSION=$(grep -m1 '"version"' src-tauri/tauri.conf.json | cut -d'"' -f4)
TAR_GZ_FILE=$(basename "$TAR_GZ")
PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Read signature - the .sig file content should be used directly (it's already base64)
# Remove any trailing newlines/whitespace
SIGNATURE=$(cat "$SIG" | tr -d '\n' | tr -d ' ')

# Generate latest.json
# URL points to "latest" release since all versions check that endpoint
DOWNLOAD_URL="https://github.com/ryanonmars/fileflow/releases/download/latest/${TAR_GZ_FILE}"

cat > "$LATEST_JSON" <<EOF
{
  "version": "${VERSION}",
  "notes": "Release ${VERSION}",
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "darwin-aarch64": {
      "signature": "${SIGNATURE}",
      "url": "${DOWNLOAD_URL}"
    }
  }
}
EOF

echo "  ✓ Generated latest.json"
echo ""
echo "=== latest.json contents ==="
cat "$LATEST_JSON"

echo ""
echo "⚠️  IMPORTANT: After downloading from GitHub, users need to remove quarantine:"
echo "   xattr -cr ~/Downloads/FileFlow.dmg"
echo "   or right-click the app in the DMG and select 'Open'"

