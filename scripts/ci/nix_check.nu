#!/usr/bin/env nu
# ──────────────────────────────────────────────────────────────────────────────
# GitKraft — build the flake and check its binaries
# ──────────────────────────────────────────────────────────────────────────────
# `nix build` of the flake's package, then: every library each binary links
# resolves (the GUI's dlopen()ed ones are on its RUNPATH) and the command-line
# binaries start. Leaves flake.lock in place for the workflow to upload.
#
# Usage: nu scripts/ci/nix_check.nu
# ──────────────────────────────────────────────────────────────────────────────

use ../project.nu [BINS]

# The libraries the GUI dlopen()s, which ldd cannot see but its RUNPATH must hold.
export const DLOPENED = [libwayland-client libxkbcommon libvulkan libGL libX11]

def main [] {
    ^nix build .#gitkraft --print-build-logs
    let out = ^readlink -f result | str trim
    for b in $BINS {
        let exe = $"($out)/bin/($b.bin)"
        let ldd = ^ldd $exe | complete
        if ($ldd.stdout | str contains "not found") { error make {msg: $"($b.bin): ($ldd.stdout)"} }
        if $b.kind != "gui" {
            let r = do { ^$exe --version } | complete
            if $r.exit_code >= 126 { error make {msg: $"($b.bin) does not start"} }
            print $"($b.bin): ($r.stdout | str trim)"
        }
    }
    let rpath = ^patchelf --print-rpath $"($out)/bin/gitkraft" | str trim | split row ':'
    for lib in $DLOPENED {
        let found = $rpath | any {|d| glob $"($d)/($lib).so*" | is-not-empty }
        if not $found { error make {msg: $"gitkraft: ($lib) is not on its RUNPATH"} }
    }
    print -e $"✅ nix build: every binary loads; the GUI finds ($DLOPENED | str join ', ')"
}
