#!/usr/bin/env nu
# ── GitKraft · test_upgrade_deps.nu ─────────────────────────────────────────
# Tests for scripts/upgrade_deps.nu — commit_label, all_passed pure functions,
# and workspace invariant checks.

use runner.nu *
use ../upgrade_deps.nu [commit_label all_passed]

# ── Helpers ─────────────────────────────────────────────────────────────────

# Build a minimal workspace Cargo.toml with the given version.
def make_workspace_cargo [version: string]: nothing -> string {
    $'[workspace]
members = [
    "crates/gitkraft-core",
    "crates/gitkraft-gui",
    "crates/gitkraft-tui",
]
resolver = "2"

[workspace.package]
version = "($version)"
edition = "2021"

[workspace.dependencies]
gitkraft-core = { path = "crates/gitkraft-core", version = "($version)" }
'
}

# Read the workspace.package version from a Cargo.toml string.
def read_workspace_version [cargo_toml: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l| $l | str trim }
    | where { |l| $l | str starts-with 'version' }
    | where { |l| not ($l | str contains 'workspace = true') }
    | first
    | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
    | get ver
    | first
}

# Read the gitkraft-core dependency version from a workspace Cargo.toml string.
def read_core_dep_version [cargo_toml: string]: nothing -> string {
    let dep_line = (
        $cargo_toml
        | lines
        | where { |l| $l =~ '^gitkraft-core\s*=' }
        | first
    )
    $dep_line
    | parse --regex 'version\s*=\s*"(?P<ver>[^"]+)"'
    | get ver
    | first
}

# ── Tests: commit_label ─────────────────────────────────────────────────────

def "test commit_label: single crate" [] {
    let label = (commit_label ["serde"])
    assert equal $label "build(deps): upgrade serde"
}

def "test commit_label: two crates" [] {
    let label = (commit_label ["serde", "tokio"])
    assert equal $label "build(deps): upgrade serde, tokio"
}

def "test commit_label: three crates" [] {
    let label = (commit_label ["a", "b", "c"])
    assert equal $label "build(deps): upgrade a, b, c"
}

def "test commit_label: empty list" [] {
    let label = (commit_label [])
    assert equal $label "build(deps): upgrade "
}

def "test commit_label: single crate with version-like name" [] {
    let label = (commit_label ["iced_aw"])
    assert equal $label "build(deps): upgrade iced_aw"
}

# ── Tests: all_passed ────────────────────────────────────────────────────────

def "test all_passed: all zeros" [] {
    assert (all_passed [0, 0, 0])
}

def "test all_passed: single zero" [] {
    assert (all_passed [0])
}

def "test all_passed: empty list" [] {
    assert (all_passed [])
}

def "test all_passed: one failure" [] {
    assert (not (all_passed [0, 1, 0]))
}

def "test all_passed: all failures" [] {
    assert (not (all_passed [1, 1, 1]))
}

def "test all_passed: high exit code" [] {
    assert (not (all_passed [0, 127]))
}

def "test all_passed: single failure" [] {
    assert (not (all_passed [1]))
}

# ── Tests: workspace invariants ──────────────────────────────────────────────

def "test workspace invariant: versions in sync" [] {
    let cargo = (make_workspace_cargo "0.5.0")
    let ws_ver = (read_workspace_version $cargo)
    let dep_ver = (read_core_dep_version $cargo)
    assert equal $ws_ver $dep_ver
}

def "test workspace invariant: versions in sync after bump" [] {
    let version = "1.2.3"
    let cargo = (make_workspace_cargo $version)
    let ws_ver = (read_workspace_version $cargo)
    let dep_ver = (read_core_dep_version $cargo)
    assert equal $ws_ver $version
    assert equal $dep_ver $version
}

def "test workspace invariant: dep line exists" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    let dep_lines = (
        $cargo
        | lines
        | where { |l| $l =~ '^gitkraft-core\s*=' }
    )
    assert equal ($dep_lines | length) 1
}

def "test workspace invariant: dep line has path and version" [] {
    let cargo = (make_workspace_cargo "4.0.0")
    let dep_line = (
        $cargo
        | lines
        | where { |l| $l =~ '^gitkraft-core\s*=' }
        | first
    )
    assert ($dep_line | str contains 'path = "crates/gitkraft-core"')
    assert ($dep_line | str contains 'version = "4.0.0"')
}

def "test workspace invariant: all three members listed" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    assert ($cargo | str contains '"crates/gitkraft-core"')
    assert ($cargo | str contains '"crates/gitkraft-gui"')
    assert ($cargo | str contains '"crates/gitkraft-tui"')
}

def "test workspace invariant: pre-release versions stay in sync" [] {
    let cargo = (make_workspace_cargo "1.0.0-alpha.3")
    let ws_ver = (read_workspace_version $cargo)
    let dep_ver = (read_core_dep_version $cargo)
    assert equal $ws_ver "1.0.0-alpha.3"
    assert equal $dep_ver "1.0.0-alpha.3"
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
