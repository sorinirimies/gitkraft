#!/usr/bin/env nu

# ──────────────────────────────────────────────────────────────
#  GitKraft – Release Preparation Script
# ──────────────────────────────────────────────────────────────
#  Performs the full release checklist:
#    1. Validate new version
#    2. Bump workspace version in root Cargo.toml
#    3. Update gitkraft-core dependency version in crate manifests
#    4. Update README badge version
#    5. Run cargo fmt, clippy, test
#    6. Generate changelog with git-cliff
#    7. Build release notes
#    8. Commit all changes
#    9. Create a signed Git tag
#
#  Usage:
#    nu scripts/release_prepare.nu <new-version>
#
#  Example:
#    nu scripts/release_prepare.nu 0.5.0
# ──────────────────────────────────────────────────────────────

# ANSI colour helpers
def green [msg: string] { $"(ansi green)($msg)(ansi reset)" }
def red [msg: string] { $"(ansi red)($msg)(ansi reset)" }
def yellow [msg: string] { $"(ansi yellow)($msg)(ansi reset)" }
def cyan [msg: string] { $"(ansi cyan)($msg)(ansi reset)" }

# ── Step helpers ──────────────────────────────────────────────

def step [num: int, msg: string] {
    print $"(cyan $'[Step ($num)]') ($msg)"
}

# ── Version validation ────────────────────────────────────────

def validate_semver [version: string] {
    let parts = ($version | split row '.')
    if ($parts | length) != 3 {
        print (red $"Error: '($version)' is not a valid semver string \(expected MAJOR.MINOR.PATCH\)")
        exit 1
    }
    for part in $parts {
        let trimmed = ($part | str trim)
        if ($trimmed | is-empty) {
            print (red $"Error: '($version)' contains an empty segment")
            exit 1
        }
        # Ensure every character is a digit
        let digits = ($trimmed | split chars | where {|c| ($c | str trim) =~ '^[0-9]$' } | length)
        if $digits != ($trimmed | str length) {
            print (red $"Error: '($version)' contains non-numeric segment '($trimmed)'")
            exit 1
        }
    }
}

# ── Workspace version bump ────────────────────────────────────

def update_workspace_version [new_version: string] {
    let cargo_path = "Cargo.toml"
    let content = (open --raw $cargo_path)
    let updated = ($content | str replace --regex 'version\s*=\s*"[^"]+"' $'version      = "($new_version)"' )
    $updated | save --force $cargo_path
}

# ── Update gitkraft-core dependency version in root manifest ──

def update_core_dep_version [new_version: string] {
    let cargo_path = "Cargo.toml"
    let lines = (open --raw $cargo_path | lines)
    let updated_lines = ($lines | each {|line|
        if ($line | str trim | find --regex '^gitkraft-core\s*=' | is-not-empty) {
            $line | str replace --regex 'version\s*=\s*"[^"]+"' $'version = "($new_version)"'
        } else {
            $line
        }
    })
    $updated_lines | str join "\n" | save --force $cargo_path
}

# ── Update README badge ──────────────────────────────────────

def update_readme_badge [new_version: string] {
    let readme_path = "README.md"
    if not ($readme_path | path exists) {
        print (yellow "Warning: README.md not found – skipping badge update")
        return
    }
    let content = (open --raw $readme_path)
    # Update crates.io badge version if hard-coded
    let updated = ($content
        | str replace --regex 'crates/v/[0-9]+\.[0-9]+\.[0-9]+' $'crates/v/($new_version)'
    )
    $updated | save --force $readme_path
}

# ── Generate changelog ────────────────────────────────────────

def generate_changelog [new_version: string] {
    print (yellow "  Generating changelog with git-cliff …")
    if (which git-cliff | is-empty) {
        print (yellow "  Warning: git-cliff not found – skipping changelog generation")
        print (yellow "  Install it with: cargo install git-cliff")
        return
    }
    try {
        git-cliff --tag $"v($new_version)" --output CHANGELOG.md
        print (green "  ✔ CHANGELOG.md updated")
    } catch {
        print (yellow "  Warning: git-cliff failed – skipping changelog generation")
    }
}

# ── Build release notes ──────────────────────────────────────

def build_release_notes [new_version: string] {
    let notes_path = $"RELEASE_NOTES_v($new_version).md"

    mut notes = $"# GitKraft v($new_version) Release Notes\n\n"
    $notes = $notes + $"## Installation\n\n"
    $notes = $notes + $"### From crates.io\n\n"
    $notes = $notes + $"```sh\n"
    $notes = $notes + $"# Desktop GUI\n"
    $notes = $notes + $"cargo install gitkraft\n\n"
    $notes = $notes + $"# Terminal UI\n"
    $notes = $notes + $"cargo install gitkraft-tui\n"
    $notes = $notes + $"```\n\n"
    $notes = $notes + $"### Pre-built binaries\n\n"
    $notes = $notes + $"Download from the [Releases page]\(https://github.com/sorinirimies/gitkraft/releases/tag/v($new_version)\).\n\n"
    $notes = $notes + $"## Crates\n\n"
    $notes = $notes + $"| Crate | Version | crates.io |\n"
    $notes = $notes + $"|-------|---------|----------|\n"
    $notes = $notes + $"| `gitkraft` | ($new_version) | [crates.io/crates/gitkraft]\(https://crates.io/crates/gitkraft\) |\n"
    $notes = $notes + $"| `gitkraft-tui` | ($new_version) | [crates.io/crates/gitkraft-tui]\(https://crates.io/crates/gitkraft-tui\) |\n"
    $notes = $notes + $"| `gitkraft-core` | ($new_version) | [crates.io/crates/gitkraft-core]\(https://crates.io/crates/gitkraft-core\) |\n\n"

    # Append changelog section if git-cliff is available
    if (which git-cliff | is-not-empty) {
        try {
            let cliff_notes = (git-cliff --tag $"v($new_version)" --unreleased --strip header | str trim)
            if ($cliff_notes | is-not-empty) {
                $notes = $notes + $"## What's Changed\n\n"
                $notes = $notes + $cliff_notes
                $notes = $notes + "\n"
            }
        }
    }

    $notes | save --force $notes_path
    print (green $"  ✔ Release notes written to ($notes_path)")
}

# ── Cargo checks ─────────────────────────────────────────────

def run_cargo_checks [] {
    print (yellow "  Running cargo fmt …")
    cargo fmt --all
    print (green "  ✔ cargo fmt")

    print (yellow "  Running cargo clippy …")
    try {
        cargo clippy --workspace -- -D warnings
        print (green "  ✔ cargo clippy passed")
    } catch {
        print (red "  ✘ cargo clippy found warnings/errors")
        exit 1
    }

    print (yellow "  Running cargo test …")
    try {
        cargo test --workspace
        print (green "  ✔ cargo test passed")
    } catch {
        print (red "  ✘ cargo test failed")
        exit 1
    }
}

# ── Git operations ────────────────────────────────────────────

def git_commit_and_tag [new_version: string] {
    print (yellow "  Staging all changes …")
    git add -A

    let commit_msg = $"chore: release v($new_version)"
    print (yellow $"  Committing: ($commit_msg)")
    git commit -m $commit_msg

    let tag = $"v($new_version)"
    print (yellow $"  Creating tag: ($tag)")
    git tag -a $tag -m $"GitKraft v($new_version)"

    print (green $"  ✔ Committed and tagged ($tag)")
    print ""
    print (cyan "  Next steps:")
    print $"    git push origin main"
    print $"    git push origin ($tag)"
}

# ── Main ──────────────────────────────────────────────────────

def main [raw_version: string] {
    # Strip leading 'v' prefix if present (e.g. v0.3.2 → 0.3.2)
    let new_version = ($raw_version | str replace --regex '^v' '')

    print ""
    print (cyan "══════════════════════════════════════════════════")
    print (cyan "  GitKraft Release Preparation")
    print (cyan "══════════════════════════════════════════════════")
    print ""

    let current_version = (open Cargo.toml | get workspace.package.version)
    print $"  Current version : (yellow $current_version)"
    print $"  New version     : (green $new_version)"
    print ""

    # Step 1 – Validate
    step 1 "Validating version …"
    validate_semver $new_version
    if $new_version == $current_version {
        print (red $"Error: new version ($new_version) is the same as the current version")
        exit 1
    }
    print (green $"  ✔ Version ($new_version) is valid")
    print ""

    # Step 2 – Bump workspace version
    step 2 "Bumping workspace version …"
    update_workspace_version $new_version
    print (green $"  ✔ workspace.package.version → ($new_version)")
    print ""

    # Step 3 – Update gitkraft-core dependency version
    step 3 "Updating gitkraft-core dependency version …"
    update_core_dep_version $new_version
    print (green $"  ✔ gitkraft-core dependency → ($new_version)")
    print ""

    # Step 4 – Update README badge
    step 4 "Updating README badge …"
    update_readme_badge $new_version
    print (green "  ✔ README badge updated")
    print ""

    # Step 5 – Cargo checks
    step 5 "Running cargo checks …"
    run_cargo_checks
    print ""

    # Step 6 – Generate changelog
    step 6 "Generating changelog …"
    generate_changelog $new_version
    print ""

    # Step 7 – Build release notes
    step 7 "Building release notes …"
    build_release_notes $new_version
    print ""

    # Step 8 – Commit
    step 8 "Committing changes …"
    git_commit_and_tag $new_version
    print ""

    print (green "══════════════════════════════════════════════════")
    print (green $"  GitKraft v($new_version) release prepared! 🚀")
    print (green "══════════════════════════════════════════════════")
    print ""
}
