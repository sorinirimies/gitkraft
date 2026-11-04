#!/usr/bin/env nu
# ── GitKraft · test_store_publishing.nu ──────────────────────────────
# The Flathub manifest pin and the Snap version.

use std/assert
use runner.nu *
use ../ci/update_flathub.nu [pin APP_ID]
use ../ci/snap_prepare.nu [set-version]

const ROOT = path self ../..

def "test flathub: the manifest pins the release tag and commit" [] {
    let text = "    sources:\n      - type: git\n        tag: v1.2.1\n        commit: 0000\n"
    assert equal (pin $text v1.3.0 abc123) "    sources:\n      - type: git\n        tag: v1.3.0\n        commit: abc123\n"
}

def "test flathub: the committed manifest has a tag and a commit to pin" [] {
    let m = open --raw ($ROOT | path join packaging flatpak $"($APP_ID).yml")
    let pinned = pin $m 9.9.9 feedface
    assert ($pinned | str contains "tag: v9.9.9")
    assert ($pinned | str contains "commit: feedface")
}

def "test snap: the version line is set, the rest stays" [] {
    assert equal (set-version "name: gitkraft\nversion: '0.0.0'\nbase: core24\n" v1.2.3) "name: gitkraft\nversion: '1.2.3'\nbase: core24\n"
}

def "test snap: both committed projects have a version line" [] {
    for s in [gitkraft gitkraft-tui] {
        let y = open --raw ($ROOT | path join packaging snap $s snap snapcraft.yaml)
        assert (set-version $y 4.5.6 | str contains "version: '4.5.6'") $s
    }
}

def main [] { run-tests }
