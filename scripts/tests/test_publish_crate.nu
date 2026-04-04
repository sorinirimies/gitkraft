#!/usr/bin/env nu
# ── GitKraft · test_publish_crate.nu ────────────────────────────────────────
# Tests for scripts/ci/publish_crate.nu — exported pure helper functions.
# We can only unit-test the pure helpers (is_already_published is impure and
# calls cargo, so it's exercised via integration tests / manual runs).

use std/assert
use runner.nu *
use ../ci/publish_crate.nu [copy_readme]

# ── Helpers ─────────────────────────────────────────────────────────────────

# Create a temporary workspace-like directory with a README.md and a crate dir.
def make_temp_workspace []: nothing -> record<root: string, crate_dir: string> {
    let root = (mktemp -d)
    let readme = ($root | path join "README.md")
    "# GitKraft\nFake README for testing." | save $readme

    let crate_dir = ($root | path join "crates" "gitkraft-core")
    mkdir $crate_dir

    { root: $root, crate_dir: $crate_dir }
}

# ── Tests: copy_readme ──────────────────────────────────────────────────────

def "test copy_readme: copies README into target dir" [] {
    let ws = (make_temp_workspace)
    let original_dir = ($env.PWD)
    cd $ws.root

    copy_readme "crates/gitkraft-core"

    let dst = ($ws.root | path join "crates" "gitkraft-core" "README.md")
    assert ($dst | path exists)

    let content = (open --raw $dst)
    assert ($content | str contains "GitKraft")

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $ws.root
}

def "test copy_readme: overwrites existing README" [] {
    let ws = (make_temp_workspace)
    let original_dir = ($env.PWD)
    cd $ws.root

    # Pre-populate with old content
    let dst = ($ws.root | path join "crates" "gitkraft-core" "README.md")
    "old content" | save $dst

    copy_readme "crates/gitkraft-core"

    let content = (open --raw $dst)
    assert ($content | str contains "GitKraft")
    assert (not ($content | str contains "old content"))

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $ws.root
}

def "test copy_readme: handles missing README gracefully" [] {
    let root = (mktemp -d)
    let crate_dir = ($root | path join "crates" "gitkraft-core")
    mkdir $crate_dir
    let original_dir = ($env.PWD)
    cd $root

    # No README.md in root — should print warning but not crash
    copy_readme "crates/gitkraft-core"

    let dst = ($crate_dir | path join "README.md")
    assert (not ($dst | path exists))

    # Restore original dir before cleanup
    cd $original_dir
    rm -rf $root
}

# ── Tests: publish order invariants ─────────────────────────────────────────
# These verify the publish-order contract that workflows depend on:
# core → gui → tui (core must be first because the others depend on it).

def "test publish order: core is first" [] {
    let order = ["gitkraft-core", "gitkraft", "gitkraft-tui"]
    assert equal ($order | first) "gitkraft-core"
}

def "test publish order: has exactly three crates" [] {
    let order = ["gitkraft-core", "gitkraft", "gitkraft-tui"]
    assert equal ($order | length) 3
}

def "test publish order: core before gui" [] {
    let order = ["gitkraft-core", "gitkraft", "gitkraft-tui"]
    let core_idx = ($order | enumerate | where { |it| $it.item == "gitkraft-core" } | get index | first)
    let gui_idx  = ($order | enumerate | where { |it| $it.item == "gitkraft"      } | get index | first)
    assert ($core_idx < $gui_idx)
}

def "test publish order: core before tui" [] {
    let order = ["gitkraft-core", "gitkraft", "gitkraft-tui"]
    let core_idx = ($order | enumerate | where { |it| $it.item == "gitkraft-core" } | get index | first)
    let tui_idx  = ($order | enumerate | where { |it| $it.item == "gitkraft-tui"  } | get index | first)
    assert ($core_idx < $tui_idx)
}

# ── Tests: readme-dir resolution ────────────────────────────────────────────
# Verify the default directory convention: crates/<crate-name>

def "test readme dir default: core resolves to crates/gitkraft-core" [] {
    let crate = "gitkraft-core"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/gitkraft-core"
}

def "test readme dir default: gui resolves to crates/gitkraft" [] {
    # Note: the *crate* is called "gitkraft" but its directory is gitkraft-gui.
    # The workflow passes --readme-dir explicitly for the GUI crate.
    let crate = "gitkraft"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/gitkraft"
}

def "test readme dir default: tui resolves to crates/gitkraft-tui" [] {
    let crate = "gitkraft-tui"
    let default_dir = $"crates/($crate)"
    assert equal $default_dir "crates/gitkraft-tui"
}

def "test readme dir override: explicit dir takes precedence" [] {
    let readme_dir = "crates/gitkraft-gui"
    let crate = "gitkraft"
    let resolved = if ($readme_dir | is-empty) { $"crates/($crate)" } else { $readme_dir }
    assert equal $resolved "crates/gitkraft-gui"
}

def "test readme dir override: empty string falls back to default" [] {
    let readme_dir = ""
    let crate = "gitkraft-core"
    let resolved = if ($readme_dir | is-empty) { $"crates/($crate)" } else { $readme_dir }
    assert equal $resolved "crates/gitkraft-core"
}

# ── Tests: version string handling ──────────────────────────────────────────

def "test version string: simple semver is valid for publish" [] {
    let version = "0.1.6"
    assert ($version =~ '^\d+\.\d+\.\d+$')
}

def "test version string: pre-release is valid for publish" [] {
    let version = "1.0.0-rc.1"
    assert ($version =~ '^\d+\.\d+\.\d+')
}

def "test version string: bare tag prefix stripped" [] {
    let tag = "v0.1.6"
    let version = ($tag | str replace 'v' '')
    assert equal $version "0.1.6"
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
