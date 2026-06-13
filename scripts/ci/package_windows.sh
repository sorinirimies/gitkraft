#!/usr/bin/env bash
set -euo pipefail
# Usage: ./scripts/ci/package_windows.sh <version>
# Builds the Windows NSIS installer. Run on a Windows runner with NSIS installed.

VERSION="$1"
NSI="packaging/windows/installer.nsi"
DIST="dist"

mkdir -p "$DIST"

# Substitute version placeholder
sed "s/@VERSION@/${VERSION}/g" "$NSI" > "$DIST/installer_versioned.nsi"

echo "🔨 Building Windows installer with NSIS..."
makensis "$DIST/installer_versioned.nsi"

echo "✅ Built dist/gitkraft-${VERSION}-windows-x86_64-setup.exe"
