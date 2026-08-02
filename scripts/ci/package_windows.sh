#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/package_windows.sh <version>
# Builds the Windows NSIS installer. Run on a Windows runner with NSIS installed.

VERSION="$1"
NSI="packaging/windows/installer.nsi"
DIST="dist"
ICON="packaging/windows/gitkraft.ico"

mkdir -p "$DIST"

# The NSIS script (MUI_ICON / MUI_UNICON) requires a real .ico file to exist
# at build time, but it is deliberately not committed to the repo (see
# packaging/windows/gitkraft.ico.txt). Generate a simple fallback icon here
# if one hasn't been provided, so the installer build never hard-fails on a
# missing file.
#
# NOTE: an earlier version of this script generated the fallback icon at
# runtime via PowerShell + System.Drawing (Bitmap -> GetHicon -> Icon.Save).
# That reliably produced a file NSIS's legacy icon loader rejected with a
# generic "can't open file" error, regardless of the bitmap size used (both
# 256x256 and 32x32 failed identically) — .NET's Icon.Save() on an icon
# created from GetHicon() is known to emit non-standard ICO data. Instead,
# embed a pre-built, verified-valid classic 16x16 32bpp ICO (BMP-encoded,
# not the Vista+ PNG-compressed format) as a base64 blob and decode it
# directly, removing the dependency on that broken code path entirely.
if [ ! -f "$ICON" ]; then
    echo "ℹ️  $ICON not found — writing a bundled fallback icon."
    base64 -d > "$ICON" <<'ICO_BASE64'
AAABAAEAEBAAAAEAIABoBAAAFgAAACgAAAAQAAAAIAAAAAEAIAAAAAAAQAQAAAAAAAAAAAAAAAAA
AAAAAAAbGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/
GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8b
GBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsY
GP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP88TOf/PEzn/zxM5/88TOf/PEzn
/zxM5/88TOf/PEzn/zxM5/88TOf/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/PEzn/zxM5/88TOf/
PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/zxM5/88
TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/8bGBj/GxgY/xsYGP8bGBj/GxgY/xsY
GP88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/GxgY/xsYGP8bGBj/GxgY
/xsYGP8bGBj/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/xsYGP8bGBj/
GxgY/xsYGP8bGBj/GxgY/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/8b
GBj/GxgY/xsYGP8bGBj/GxgY/xsYGP88TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn/zxM
5/88TOf/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/PEzn
/zxM5/88TOf/PEzn/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/zxM5/88TOf/PEzn/zxM5/88TOf/
PEzn/zxM5/88TOf/PEzn/zxM5/8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP88TOf/PEzn/zxM5/88
TOf/PEzn/zxM5/88TOf/PEzn/zxM5/88TOf/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsY
GP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY
/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/
GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8bGBj/GxgY/xsYGP8b
GBj/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
AAAAAAAAAAAAAA==
ICO_BASE64
    if [ ! -f "$ICON" ] || [ ! -s "$ICON" ]; then
        echo "❌ Fallback icon generation failed — $ICON still missing or empty."
        exit 1
    fi
    echo "✅ Wrote fallback icon at $ICON ($(wc -c < "$ICON" | tr -d ' ') bytes)"
fi

# Substitute version placeholder
sed "s/@VERSION@/${VERSION}/g" "$NSI" > "$DIST/installer_versioned.nsi"

echo "🔨 Building Windows installer with NSIS..."
# makensis is installed by choco to a fixed path not automatically on bash PATH
'/c/Program Files (x86)/NSIS/makensis.exe' "$DIST/installer_versioned.nsi"

echo "✅ Built dist/gitkraft-${VERSION}-windows-x86_64-setup.exe"
