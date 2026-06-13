#!/usr/bin/env bash
set -euo pipefail
# Usage: ./packaging/macos/create_dmg.sh <version>
# Creates a macOS DMG containing both GitKraft binaries.
# Requires: create-dmg (brew install create-dmg)

VERSION="$1"
DIST="dist"
APP_NAME="GitKraft"
BUNDLE="${DIST}/${APP_NAME}.app"

mkdir -p "$DIST"

# ── Create .app bundle for the GUI ───────────────────────────────────────────
mkdir -p "${BUNDLE}/Contents/MacOS"
mkdir -p "${BUNDLE}/Contents/Resources"

# Copy universal binary (lipo-merged) or individual arch binary
if [ -f "target/universal-apple-darwin/release/gitkraft" ]; then
    cp "target/universal-apple-darwin/release/gitkraft" "${BUNDLE}/Contents/MacOS/gitkraft"
elif [ -f "target/aarch64-apple-darwin/release/gitkraft" ]; then
    cp "target/aarch64-apple-darwin/release/gitkraft" "${BUNDLE}/Contents/MacOS/gitkraft"
else
    cp "target/x86_64-apple-darwin/release/gitkraft" "${BUNDLE}/Contents/MacOS/gitkraft"
fi
chmod +x "${BUNDLE}/Contents/MacOS/gitkraft"

# Copy TUI binary alongside the .app for terminal usage
if [ -f "target/universal-apple-darwin/release/gitkraft-tui" ]; then
    cp "target/universal-apple-darwin/release/gitkraft-tui" "${DIST}/gitkraft-tui"
elif [ -f "target/aarch64-apple-darwin/release/gitkraft-tui" ]; then
    cp "target/aarch64-apple-darwin/release/gitkraft-tui" "${DIST}/gitkraft-tui"
else
    cp "target/x86_64-apple-darwin/release/gitkraft-tui" "${DIST}/gitkraft-tui"
fi

# Info.plist
cat > "${BUNDLE}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>gitkraft</string>
    <key>CFBundleIdentifier</key>
    <string>com.sorinirimies.gitkraft</string>
    <key>CFBundleName</key>
    <string>GitKraft</string>
    <key>CFBundleDisplayName</key>
    <string>GitKraft</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF

echo "✅ Created ${APP_NAME}.app bundle"

# ── Create DMG ────────────────────────────────────────────────────────────────
OUTPUT="${DIST}/gitkraft-${VERSION}-macos.dmg"

if command -v create-dmg &>/dev/null; then
    create-dmg \
        --volname "GitKraft ${VERSION}" \
        --volicon "packaging/macos/gitkraft.icns" \
        --window-pos 200 120 \
        --window-size 660 400 \
        --icon-size 128 \
        --icon "GitKraft.app" 160 185 \
        --hide-extension "GitKraft.app" \
        --app-drop-link 500 185 \
        --no-internet-enable \
        "$OUTPUT" \
        "$DIST/" \
    2>/dev/null || true
else
    # Fallback: plain hdiutil DMG (no fancy layout)
    hdiutil create -volname "GitKraft ${VERSION}" \
        -srcfolder "$DIST" \
        -ov -format UDZO \
        "$OUTPUT"
fi

echo "✅ Built ${OUTPUT}"
