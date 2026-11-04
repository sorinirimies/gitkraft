#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Flathub update for a release
# ──────────────────────────────────────────────────────────────────────────────
# Regenerates cargo-sources.json (flatpak-cargo-generator), pins the release tag
# and commit in the manifest and pushes both to the Flathub app repo
# (github.com/flathub/io.github.sorinirimies.GitKraft, created by the one-time
# submission to flathub/flathub). Needs FLATHUB_TOKEN and python3 with aiohttp,
# toml and tomlkit; without the token it does nothing.
#
# Usage: nu scripts/ci/update_flathub.nu <version>
# ──────────────────────────────────────────────────────────────────────────────

export const APP_ID = "io.github.sorinirimies.GitKraft"
const MANIFEST = "packaging/flatpak/io.github.sorinirimies.GitKraft.yml"
const GENERATOR = "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py"

# The manifest with its source pinned to tag v<version> at `commit`.
export def pin [text: string, version: string, commit: string]: nothing -> string {
    let v = $version | str replace --regex '^v' ''
    $text
    | str replace --regex '(?m)^(\s+tag: ).*$' ('${1}v' + $v)
    | str replace --regex '(?m)^(\s+commit: ).*$' ('${1}' + $commit)
}

def main [version: string] {
    let token = $env.FLATHUB_TOKEN? | default ""
    if ($token | is-empty) {
        print "flathub=skipped"
        return
    }
    let v = $version | str replace --regex '^v' ''
    let commit = ^git rev-parse $"v($v)^{commit}" | str trim
    let tmp = mktemp -d
    http get --raw $GENERATOR | save $"($tmp)/generator.py"
    ^python3 $"($tmp)/generator.py" Cargo.lock -o $"($tmp)/cargo-sources.json"
    pin (open --raw $MANIFEST) $v $commit | save --force $MANIFEST
    let work = $"($tmp)/flathub"
    ^git clone -q $"https://x-access-token:($token)@github.com/flathub/($APP_ID).git" $work
    cp $MANIFEST $"($work)/($APP_ID).yml"
    cp $"($tmp)/cargo-sources.json" $"($work)/cargo-sources.json"
    cd $work
    ^git config user.name "gitkraft-release-bot"
    ^git config user.email "gitkraft-release-bot@users.noreply.github.com"
    ^git add -A
    if (do { ^git diff --cached --quiet } | complete).exit_code == 0 {
        print "flathub=unchanged"
        return
    }
    ^git commit -q -m $"Update to v($v)"
    ^git push -q origin HEAD
    print $"flathub=v($v)"
}
