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
#        nu scripts/ci/update_flathub.nu prepare <version> --out <dir>
#          (the manifest and cargo-sources.json Flathub builds, without pushing;
#          .github/workflows/flatpak.yml builds and lints them)
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

# Writes <out>/<id>.yml (pinned to the tag's commit) and <out>/cargo-sources.json
# (from the tag's Cargo.lock), what the Flathub app repo holds for that version.
def "main prepare" [version: string, --out: string] {
    let v = $version | str replace --regex '^v' ''
    let commit = ^git rev-parse $"v($v)^{commit}" | str trim
    let tmp = mktemp -d
    mkdir $out
    http get --raw $GENERATOR | save $"($tmp)/generator.py"
    ^git show $"v($v):Cargo.lock" | save $"($tmp)/Cargo.lock"
    ^python3 $"($tmp)/generator.py" $"($tmp)/Cargo.lock" -o $"($out)/cargo-sources.json"
    pin (open --raw $MANIFEST) $v $commit | save --force $"($out)/($APP_ID).yml"
    rm -rf $tmp
    print -e $"✅ ($out): manifest at v($v) \(($commit | str substring 0..<7)) and cargo-sources.json"
}

def main [version: string] {
    let token = $env.FLATHUB_TOKEN? | default ""
    if ($token | is-empty) {
        print "flathub=skipped"
        return
    }
    let v = $version | str replace --regex '^v' ''
    let tmp = mktemp -d
    main prepare $v --out $"($tmp)/files"
    let work = $"($tmp)/flathub"
    ^git clone -q $"https://x-access-token:($token)@github.com/flathub/($APP_ID).git" $work
    cp $"($tmp)/files/($APP_ID).yml" $"($work)/($APP_ID).yml"
    cp $"($tmp)/files/cargo-sources.json" $"($work)/cargo-sources.json"
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
