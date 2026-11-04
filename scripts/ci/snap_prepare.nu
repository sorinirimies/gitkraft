#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Snap project for a release
# ──────────────────────────────────────────────────────────────────────────────
# Copies packaging/snap/<snap>/snap to ./snap (where snapcraft looks) with its
# version set to the release.
#
# Usage: nu scripts/ci/snap_prepare.nu <gitkraft|gitkraft-tui> <version>
# ──────────────────────────────────────────────────────────────────────────────

# snapcraft.yaml with `version` set.
export def set-version [text: string, version: string]: nothing -> string {
    let v = $version | str replace --regex '^v' ''
    $text | str replace --regex "(?m)^version: .*$" $"version: '($v)'"
}

def main [snap: string, version: string] {
    let src = $"packaging/snap/($snap)/snap"
    if not ($src | path exists) {
        print -e $"❌ no snap project at ($src)"
        exit 1
    }
    rm -rf snap
    cp -r $src snap
    set-version (open --raw snap/snapcraft.yaml) $version | save --force snap/snapcraft.yaml
    print $"snap=($snap)"
}
