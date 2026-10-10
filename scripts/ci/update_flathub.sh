#!/usr/bin/env bash
set -euo pipefail
# Usage: FLATHUB_TOKEN=... ./scripts/ci/update_flathub.sh <version>
# Regenerates cargo-sources.json, pins tag/commit in the manifest and pushes to
# the Flathub app repo (github.com/flathub/io.github.sorinirimies.GitKraft).
# The repo must already exist (one-time submission PR to flathub/flathub).
VERSION="$1"
APP_ID="io.github.sorinirimies.GitKraft"
MANIFEST="packaging/flatpak/${APP_ID}.yml"
COMMIT=$(git rev-parse "v${VERSION}^{commit}")

pip install --quiet aiohttp toml tomlkit
curl -sSL -o /tmp/flatpak-cargo-generator.py \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
python3 /tmp/flatpak-cargo-generator.py Cargo.lock -o /tmp/cargo-sources.json

sed -i "s|^\(        tag: \).*|\1v${VERSION}|; s|^\(        commit: \).*|\1${COMMIT}|" "$MANIFEST"

WORK=$(mktemp -d)
git clone "https://x-access-token:${FLATHUB_TOKEN}@github.com/flathub/${APP_ID}.git" "$WORK"
cp "$MANIFEST" "$WORK/${APP_ID}.yml"
cp /tmp/cargo-sources.json "$WORK/cargo-sources.json"
cd "$WORK"
git config user.name  "gitkraft-release-bot"
git config user.email "gitkraft-release-bot@users.noreply.github.com"
git add -A
git diff --cached --quiet && { echo "Flathub already up to date."; exit 0; }
git commit -m "Update to v${VERSION}"
git push origin HEAD
echo "✅ Flathub updated to v${VERSION}"
