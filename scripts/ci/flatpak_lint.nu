#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — Flathub's linter on a manifest and a built repo
# ──────────────────────────────────────────────────────────────────────────────
# Runs flatpak-builder-lint from org.flatpak.Builder (what Flathub's review bot
# runs) on the manifest and on the OSTree repo flatpak-builder exported. Fails on
# errors; warnings are printed.
#
# Usage: nu scripts/ci/flatpak_lint.nu <manifest> <repo>
# ──────────────────────────────────────────────────────────────────────────────

# The lint invocations: [kind, path] pairs as flatpak-run arguments.
export def lint-commands [manifest: string, repo: string]: nothing -> list<list<string>> {
    [[manifest $manifest] [repo $repo]] | each {|k|
        [run --command=flatpak-builder-lint org.flatpak.Builder --exceptions $k.0 $k.1]
    }
}

def main [manifest: string, repo: string] {
    ^flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    ^flatpak install --user -y --noninteractive flathub org.flatpak.Builder
    mut failed = []
    for c in (lint-commands $manifest $repo) {
        print -e $"▶ flatpak-builder-lint ($c | last 2 | str join ' ')"
        let r = do { ^flatpak ...$c } | complete
        print $r.stdout
        if $r.exit_code != 0 { $failed = $failed ++ [($c | get 4)] }
    }
    if ($failed | is-not-empty) { error make {msg: $"flatpak-builder-lint failed: ($failed | str join ', ')"} }
    print -e "✅ flatpak-builder-lint: no errors"
}
