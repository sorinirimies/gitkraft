#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/package_appimage.sh <version> <target>
VERSION="$1"
TARGET="$2"
DIST="dist"
ARCH=$(uname -m)

mkdir -p "$DIST"

for APP in gitkraft-tui gitkraft; do
    APPDIR="$DIST/AppDir-$APP"
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"

    BIN_SRC="target/$TARGET/release/$APP"
    if [ ! -f "$BIN_SRC" ]; then
        echo "⚠️  $BIN_SRC not found — skipping AppImage for $APP"
        continue
    fi

    cp "$BIN_SRC" "$APPDIR/usr/bin/$APP"
    chmod +x "$APPDIR/usr/bin/$APP"

    # Desktop entry
    DESKTOP_NAME="GitKraft TUI"
    DESKTOP_COMMENT="Terminal Git IDE"
    if [ "$APP" = "gitkraft" ]; then
        DESKTOP_NAME="GitKraft"
        DESKTOP_COMMENT="Desktop GUI Git IDE"
    fi

    cat > "$APPDIR/$APP.desktop" <<EOF
[Desktop Entry]
Name=$DESKTOP_NAME
Comment=$DESKTOP_COMMENT
Exec=$APP
Icon=$APP
Type=Application
Categories=Development;
EOF
    cp "$APPDIR/$APP.desktop" "$APPDIR/usr/share/applications/"

    # Minimal icon (create a placeholder if no real icon exists)
    if [ -f "assets/icons/$APP.png" ]; then
        cp "assets/icons/$APP.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/$APP.png"
        cp "assets/icons/$APP.png" "$APPDIR/$APP.png"
    else
        # Create a 1x1 transparent PNG as placeholder
        printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82' > "$APPDIR/$APP.png"
        cp "$APPDIR/$APP.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/$APP.png"
    fi

    # AppRun
    cat > "$APPDIR/AppRun" <<EOF
#!/bin/sh
exec "\$(dirname "\$0")/usr/bin/$APP" "\$@"
EOF
    chmod +x "$APPDIR/AppRun"

    OUTPUT="$DIST/${APP}-${VERSION}-${ARCH}.AppImage"
    ARCH="$ARCH" appimagetool "$APPDIR" "$OUTPUT" 2>/dev/null || \
    appimagetool "$APPDIR" "$OUTPUT"
    echo "✅ Built $OUTPUT"
done
