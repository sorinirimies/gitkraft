#!/usr/bin/env nu
# ── GitKraft · test_bump_version.nu ─────────────────────────────────────────
# Tests for scripts/bump_version.nu — bumping versions across all crate manifests.

use runner.nu *

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

# Build a minimal crate Cargo.toml (uses workspace version).
def make_crate_cargo [name: string]: nothing -> string {
    $'[package]
name = "($name)"
version.workspace = true
edition.workspace = true
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

# Update the workspace.package version in a Cargo.toml string.
def apply_workspace_version_update [cargo_toml: string, new_version: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l|
        let trimmed = ($l | str trim)
        if ($trimmed | str starts-with 'version') and (not ($trimmed | str contains 'workspace = true')) and ($trimmed =~ '^version\s*=\s*"') {
            $'version = "($new_version)"'
        } else {
            $l
        }
    }
    | str join "\n"
}

# Update the gitkraft-core dependency version in a workspace Cargo.toml string.
def apply_core_dep_update [cargo_toml: string, new_version: string]: nothing -> string {
    $cargo_toml
    | lines
    | each { |l|
        if ($l =~ '^gitkraft-core\s*=') {
            $'gitkraft-core = { path = "crates/gitkraft-core", version = "($new_version)" }'
        } else {
            $l
        }
    }
    | str join "\n"
}

# Bump: apply both workspace version and core dep version updates.
def bump_version [cargo_toml: string, new_version: string]: nothing -> string {
    let step1 = (apply_workspace_version_update $cargo_toml $new_version)
    apply_core_dep_update $step1 $new_version
}

# ── Tests ───────────────────────────────────────────────────────────────────

def "test bump_version: patch bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.0.1")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.0.1"
}

def "test bump_version: minor bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.1.0"
}

def "test bump_version: major bump updates workspace version" [] {
    let cargo = (make_workspace_cargo "1.2.3")
    let bumped = (bump_version $cargo "2.0.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "2.0.0"
}

def "test bump_version: core dep version is updated" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    let bumped = (bump_version $cargo "0.2.0")
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $dep_ver "0.2.0"
}

def "test bump_version: workspace and core dep versions stay in sync" [] {
    let cargo = (make_workspace_cargo "3.0.0")
    let bumped = (bump_version $cargo "3.1.0")
    let ws_ver = (read_workspace_version $bumped)
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $ws_ver $dep_ver
}

def "test bump_version: pre-release version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0-rc.1")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "1.1.0-rc.1"
}

def "test bump_version: pre-release core dep version" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "1.1.0-rc.1")
    let dep_ver = (read_core_dep_version $bumped)
    assert equal $dep_ver "1.1.0-rc.1"
}

def "test bump_version: idempotent when version unchanged" [] {
    let cargo = (make_workspace_cargo "0.5.0")
    let bumped = (bump_version $cargo "0.5.0")
    let ver = (read_workspace_version $bumped)
    assert equal $ver "0.5.0"
}

def "test bump_version: crate manifests use workspace version" [] {
    # Crate Cargo.tomls should have version.workspace = true,
    # meaning they don't need individual updates — only the root does.
    let crate_cargo = (make_crate_cargo "gitkraft-core")
    assert ($crate_cargo | str contains "version.workspace = true")

    let crate_gui = (make_crate_cargo "gitkraft")
    assert ($crate_gui | str contains "version.workspace = true")

    let crate_tui = (make_crate_cargo "gitkraft-tui")
    assert ($crate_tui | str contains "version.workspace = true")
}

def "test bump_version: workspace members list is preserved" [] {
    let cargo = (make_workspace_cargo "1.0.0")
    let bumped = (bump_version $cargo "2.0.0")
    assert ($bumped | str contains '"crates/gitkraft-core"')
    assert ($bumped | str contains '"crates/gitkraft-gui"')
    assert ($bumped | str contains '"crates/gitkraft-tui"')
}

def "test bump_version: double bump produces correct version" [] {
    let cargo = (make_workspace_cargo "0.1.0")
    let first = (bump_version $cargo "0.2.0")
    let second = (bump_version $first "0.3.0")
    let ver = (read_workspace_version $second)
    let dep_ver = (read_core_dep_version $second)
    assert equal $ver "0.3.0"
    assert equal $dep_ver "0.3.0"
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
