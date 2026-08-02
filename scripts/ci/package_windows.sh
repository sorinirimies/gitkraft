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
# missing file. This uses PowerShell + System.Drawing, which ship with every
# GitHub-hosted Windows runner, so it needs no extra tool installation.
if [ ! -f "$ICON" ]; then
    echo "ℹ️  $ICON not found — generating a fallback icon."
    # NOTE: kept at a classic small size (32x32). System.Drawing's ICO writer
    # switches to PNG-compressed icon entries for larger bitmaps (Vista+ ICO
    # format), which NSIS's legacy icon loader cannot read — it fails with a
    # generic "can't open file" error even though the file exists and is a
    # valid .ico. Staying at 32x32 forces classic BMP-encoded ICO data.
    powershell -NoProfile -Command "
        Add-Type -AssemblyName System.Drawing
        \$bmp = New-Object System.Drawing.Bitmap 32,32
        \$g = [System.Drawing.Graphics]::FromImage(\$bmp)
        \$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        \$g.Clear([System.Drawing.Color]::FromArgb(255, 24, 24, 27))
        \$brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 231, 76, 60))
        \$font = New-Object System.Drawing.Font('Consolas', 13, [System.Drawing.FontStyle]::Bold)
        \$sf = New-Object System.Drawing.StringFormat
        \$sf.Alignment = [System.Drawing.StringAlignment]::Center
        \$sf.LineAlignment = [System.Drawing.StringAlignment]::Center
        \$rect = New-Object System.Drawing.RectangleF 0,0,32,32
        \$g.DrawString('GK', \$font, \$brush, \$rect, \$sf)
        \$hIcon = \$bmp.GetHicon()
        \$icon = [System.Drawing.Icon]::FromHandle(\$hIcon)
        \$fs = New-Object System.IO.FileStream '$ICON', 'Create'
        \$icon.Save(\$fs)
        \$fs.Close()
        \$icon.Dispose()
        \$bmp.Dispose()
    "
    if [ ! -f "$ICON" ]; then
        echo "❌ Fallback icon generation failed — $ICON still missing."
        exit 1
    fi
    echo "✅ Generated fallback icon at $ICON ($(wc -c < "$ICON" | tr -d ' ') bytes)"
fi

# Substitute version placeholder
sed "s/@VERSION@/${VERSION}/g" "$NSI" > "$DIST/installer_versioned.nsi"

echo "🔨 Building Windows installer with NSIS..."
# makensis is installed by choco to a fixed path not automatically on bash PATH
'/c/Program Files (x86)/NSIS/makensis.exe' "$DIST/installer_versioned.nsi"

echo "✅ Built dist/gitkraft-${VERSION}-windows-x86_64-setup.exe"
