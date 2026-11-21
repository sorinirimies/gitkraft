#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Push rendered manifests to a package-manager repo
# ──────────────────────────────────────────────────────────────────────────────
# Copies every file under <dir> (laid out like the repo: Casks/, Formula/,
# bucket/) into github.com/<repo> and commits them together. Needs
# $env.TAP_GITHUB_TOKEN (contents: write on <repo>); without it the push is
# skipped with a warning, so the release still succeeds.
#
# Usage: nu scripts/ci/push_manifest.nu <repo> <dir> <version>
# ──────────────────────────────────────────────────────────────────────────────

export def remote_url [repo: string, token: string]: nothing -> string {
    $"https://x-access-token:($token)@github.com/($repo).git"
}

export def commit_message [app: string, version: string]: nothing -> string {
    $"($app): update to ($version | str replace --regex '^v' '')"
}

# The files under `dir`, relative to it.
export def files_under [dir: string]: nothing -> list<string> {
    do { cd $dir; ls **/* | where type == file | get name | sort }
}

def main [repo: string, dir: string, version: string, --app: string = "gitkraft"] {
    let token = $env.TAP_GITHUB_TOKEN? | default ""
    if $token == "" {
        print $"::warning::TAP_GITHUB_TOKEN is not set — skipped pushing to ($repo)"
        return
    }
    let src = $dir | path expand
    let files = files_under $src
    let work = mktemp -d
    ^git clone -q --depth 1 (remote_url $repo $token) $work
    cd $work
    for f in $files {
        mkdir ($f | path dirname)
        cp ($src | path join $f) $f
        ^git add $f
    }
    if (^git status --porcelain | str trim | is-empty) {
        print $"($repo): already at ($version)"
        return
    }
    ^git -c user.name=gitkraft-release -c user.email=gitkraft-release@users.noreply.github.com commit -q -m (commit_message $app $version)
    ^git push -q origin HEAD
    print $"✓ pushed ($files | str join ', ') to ($repo)"
}
