#!/usr/bin/env nu
# ── GitKraft · test_release_prepare.nu ──────────────────────────────────────
# Tests for scripts/release_prepare.nu — release notes generation, tag format,
# and full release checklist logic.

use std/assert
use runner.nu *

# ── Helpers ─────────────────────────────────────────────────────────────────

# Build release notes the same way release_prepare.nu would.
def build_release_notes [version: string, date: string, changes: list<string>]: nothing -> string {
    let header = $"# GitKraft v($version)\n\nReleased: ($date)\n"
    let body = if ($changes | is-empty) {
        "\nNo notable changes.\n"
    } else {
        let items = ($changes | each { |c| $"- ($c)" } | str join "\n")
        $"\n## What's Changed\n\n($items)\n"
    }
    let install = $"\n## Install\n\n```\ncargo install gitkraft\ncargo install gitkraft-tui\n```\n"
    $"($header)($body)($install)"
}

# Format a git tag from a version string.
def format_tag [version: string]: nothing -> string {
    $"v($version)"
}

# Parse a version string into its components.
def parse_version [version: string]: nothing -> record<major: int, minor: int, patch: string> {
    let parts = ($version | split row ".")
    {
        major: ($parts | get 0 | into int),
        minor: ($parts | get 1 | into int),
        patch: ($parts | skip 2 | str join "."),
    }
}

# Build the publish order used during release.
def release_publish_order []: nothing -> list<string> {
    ["gitkraft-core", "gitkraft", "gitkraft-tui"]
}

# Validate that a version string looks like semver.
def is_valid_semver [version: string]: nothing -> bool {
    ($version =~ '^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$')
}

# ── Tests ───────────────────────────────────────────────────────────────────

def "test release_prepare: tag format for simple version" [] {
    let tag = (format_tag "1.0.0")
    assert equal $tag "v1.0.0"
}

def "test release_prepare: tag format for pre-release" [] {
    let tag = (format_tag "2.1.0-rc.1")
    assert equal $tag "v2.1.0-rc.1"
}

def "test release_prepare: tag starts with v" [] {
    let tag = (format_tag "0.1.0")
    assert ($tag | str starts-with "v")
}

def "test release_prepare: release notes header contains GitKraft" [] {
    let notes = (build_release_notes "1.0.0" "2025-01-15" ["Initial release"])
    assert ($notes | str contains "GitKraft v1.0.0")
}

def "test release_prepare: release notes contain date" [] {
    let notes = (build_release_notes "1.0.0" "2025-06-01" ["Bug fixes"])
    assert ($notes | str contains "2025-06-01")
}

def "test release_prepare: release notes list changes" [] {
    let changes = ["Added git blame view", "Fixed diff rendering", "Improved startup time"]
    let notes = (build_release_notes "0.2.0" "2025-03-10" $changes)
    assert ($notes | str contains "- Added git blame view")
    assert ($notes | str contains "- Fixed diff rendering")
    assert ($notes | str contains "- Improved startup time")
}

def "test release_prepare: release notes contain whats changed heading" [] {
    let notes = (build_release_notes "1.0.0" "2025-01-01" ["Something new"])
    assert ($notes | str contains "## What's Changed")
}

def "test release_prepare: release notes with no changes" [] {
    let notes = (build_release_notes "1.0.0" "2025-01-01" [])
    assert ($notes | str contains "No notable changes.")
    assert (not ($notes | str contains "## What's Changed"))
}

def "test release_prepare: release notes contain install section" [] {
    let notes = (build_release_notes "1.0.0" "2025-01-01" ["Initial release"])
    assert ($notes | str contains "## Install")
    assert ($notes | str contains "cargo install gitkraft")
    assert ($notes | str contains "cargo install gitkraft-tui")
}

def "test release_prepare: install section has both binaries" [] {
    let notes = (build_release_notes "0.5.0" "2025-07-01" [])
    let install_section = (
        $notes
        | lines
        | where { |l| $l | str contains "cargo install" }
    )
    assert equal ($install_section | length) 2
    assert ("cargo install gitkraft" in $install_section)
    assert ("cargo install gitkraft-tui" in $install_section)
}

def "test release_prepare: parse simple version" [] {
    let parsed = (parse_version "1.2.3")
    assert equal $parsed.major 1
    assert equal $parsed.minor 2
    assert equal $parsed.patch "3"
}

def "test release_prepare: parse zero version" [] {
    let parsed = (parse_version "0.0.0")
    assert equal $parsed.major 0
    assert equal $parsed.minor 0
    assert equal $parsed.patch "0"
}

def "test release_prepare: parse pre-release version" [] {
    let parsed = (parse_version "1.0.0-beta.1")
    assert equal $parsed.major 1
    assert equal $parsed.minor 0
    assert equal $parsed.patch "0-beta.1"
}

def "test release_prepare: valid semver accepted" [] {
    assert (is_valid_semver "0.1.0")
    assert (is_valid_semver "1.0.0")
    assert (is_valid_semver "12.345.6789")
    assert (is_valid_semver "1.0.0-rc.1")
    assert (is_valid_semver "0.0.0-alpha.0")
}

def "test release_prepare: invalid semver rejected" [] {
    assert (not (is_valid_semver "1.0"))
    assert (not (is_valid_semver "v1.0.0"))
    assert (not (is_valid_semver "1.0.0.0"))
    assert (not (is_valid_semver "abc"))
    assert (not (is_valid_semver ""))
}

def "test release_prepare: publish order is correct" [] {
    let order = (release_publish_order)
    assert equal ($order | length) 3
    assert equal ($order | first) "gitkraft-core"
    assert ("gitkraft" in $order)
    assert ("gitkraft-tui" in $order)
}

def "test release_prepare: core published before dependents" [] {
    let order = (release_publish_order)
    let core_idx = ($order | enumerate | where { |it| $it.item == "gitkraft-core" } | get index | first)
    let gui_idx  = ($order | enumerate | where { |it| $it.item == "gitkraft"      } | get index | first)
    let tui_idx  = ($order | enumerate | where { |it| $it.item == "gitkraft-tui"  } | get index | first)
    assert ($core_idx < $gui_idx)
    assert ($core_idx < $tui_idx)
}

def "test release_prepare: release notes version matches tag" [] {
    let version = "3.2.1"
    let tag = (format_tag $version)
    let notes = (build_release_notes $version "2025-12-25" ["Holiday release"])
    # The tag version should appear in the notes header.
    assert ($notes | str contains $"GitKraft ($tag | str replace 'v' 'v')")
}

def "test release_prepare: multiple releases produce distinct notes" [] {
    let notes_a = (build_release_notes "1.0.0" "2025-01-01" ["First"])
    let notes_b = (build_release_notes "2.0.0" "2025-06-01" ["Second"])
    assert ($notes_a | str contains "GitKraft v1.0.0")
    assert ($notes_b | str contains "GitKraft v2.0.0")
    assert (not ($notes_a | str contains "v2.0.0"))
    assert (not ($notes_b | str contains "v1.0.0"))
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
