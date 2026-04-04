# gitkraft workspace — task runner
# Install just:      cargo install just
# Install git-cliff: cargo install git-cliff
# Usage: just <task>
# ── Default ───────────────────────────────────────────────────────────────────

default:
    @just --list

# ── Tool checks ───────────────────────────────────────────────────────────────

_check-git-cliff:
    @command -v git-cliff >/dev/null 2>&1 || { \
        echo "❌ git-cliff not found. Install with: cargo install git-cliff"; exit 1; \
    }

# Check nu (nushell) is available
_check-nu:
    @command -v nu >/dev/null 2>&1 || { \
        echo "❌ nu (nushell) not found. Install: https://www.nushell.sh"; exit 1; \
    }

# Install all recommended development tools
install-tools:
    @echo "Installing development tools…"
    @command -v git-cliff >/dev/null 2>&1 || cargo install git-cliff
    @command -v nu >/dev/null 2>&1 && echo "✅ nu found" || echo "⚠ nu (nushell) not found. Install: https://www.nushell.sh"
    @echo "✅ All tools installed!"

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the entire workspace (dev)
build:
    cargo build --workspace

# Build only the core library (dev)
build-core:
    cargo build -p gitkraft-core

# Build only the GUI crate (dev)
build-gui:
    cargo build -p gitkraft

# Build only the TUI crate (dev)
build-tui:
    cargo build -p gitkraft-tui

# Build release binaries for GUI and TUI
build-release:
    cargo build --release -p gitkraft
    cargo build --release -p gitkraft-tui

# ── Run ───────────────────────────────────────────────────────────────────────

# Launch the Iced desktop GUI
run-gui:
    cargo run -p gitkraft

# Launch the Ratatui terminal UI
run-tui:
    cargo run -p gitkraft-tui

# Alias: default run launches the GUI
run: run-gui

# ── Test ──────────────────────────────────────────────────────────────────────

# Run the full workspace test suite
test:
    cargo test --workspace --locked --all-features --all-targets

# Test only the core library
test-core:
    cargo test -p gitkraft-core --all-features

# Test only the GUI crate
test-gui:
    cargo test -p gitkraft --all-features

# Test only the TUI crate
test-tui:
    cargo test -p gitkraft-tui --all-features

# Run Nu script tests
test-nu: _check-nu
    nu scripts/tests/run_all.nu

# Run both Rust and Nu tests
test-all-nu: test test-nu
    @echo "✅ All Rust and Nu tests passed!"

# ── Code quality ──────────────────────────────────────────────────────────────

# Check without building
check:
    cargo check --workspace

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy across the workspace
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated

# Run all quality checks (fmt, clippy, test, nu) — must pass before a release
check-all: fmt-check clippy test test-nu
    @echo "✅ All checks passed!"

# ── Documentation ─────────────────────────────────────────────────────────────

# Generate and open docs for the GUI crate
doc-gui:
    cargo doc --no-deps -p gitkraft --open

# Generate and open docs for the TUI crate
doc-tui:
    cargo doc --no-deps -p gitkraft-tui --open

# Generate docs for the full workspace (no browser)
doc:
    cargo doc --no-deps --workspace

# ── Changelog ─────────────────────────────────────────────────────────────────

# Regenerate the full CHANGELOG.md from all tags
changelog: _check-git-cliff
    @echo "Generating full changelog…"
    git-cliff --output CHANGELOG.md
    @echo "✅ CHANGELOG.md updated."

# Prepend only unreleased commits to CHANGELOG.md
changelog-unreleased: _check-git-cliff
    git-cliff --unreleased --prepend CHANGELOG.md
    @echo "✅ Unreleased changes prepended."

# Preview changelog for the next release without writing the file
changelog-preview: _check-git-cliff
    @git-cliff --unreleased

# ── Version bump ─────────────────────────────────────────────────────────────

# Bump the workspace version, regenerate Cargo.lock + CHANGELOG.md, commit and tag.
bump version: check-all _check-git-cliff _check-nu
    nu scripts/bump_version.nu --yes {{ version }}

# ── Publish (crates.io) ───────────────────────────────────────────────────────

# Run the full pre-publish readiness check (fmt, clippy, tests, docs, dry-run)
check-publish: _check-nu
    nu scripts/check_publish.nu

# Dry-run publish for all three crates (in dependency order)
publish-dry: check-all
    @echo "Dry-run: gitkraft-core"
    cargo publish --dry-run -p gitkraft-core
    @echo "Dry-run: gitkraft (GUI)"
    cargo publish --dry-run -p gitkraft
    @echo "Dry-run: gitkraft-tui"
    cargo publish --dry-run -p gitkraft-tui

# Publish all three in dependency order: core → gui → tui.
publish: check-all publish-core publish-gui publish-tui
    @echo "✅ gitkraft-core, gitkraft, and gitkraft-tui published to crates.io!"

# Publish gitkraft-core (required by gui and tui)
publish-core:
    @echo "📦 Publishing gitkraft-core…"
    cargo publish -p gitkraft-core
    @echo "⏳ Waiting 30 s for the index to propagate…"
    sleep 30

# Publish gitkraft-gui (released as `gitkraft` on crates.io)
publish-gui:
    @echo "📦 Publishing gitkraft (GUI)…"
    cargo publish -p gitkraft

# Publish gitkraft-tui
publish-tui:
    @echo "📦 Publishing gitkraft-tui…"
    cargo publish -p gitkraft-tui

# Show what would be released without making any changes
release-preview: _check-git-cliff
    @echo "Current version: $(just version)"
    @echo ""
    @echo "Unreleased commits:"
    @git-cliff --unreleased
    @echo ""
    @echo "Workspace version:"
    @grep -A5 '^\[workspace\.package\]' Cargo.toml | grep '^version'
    @echo ""
    @echo "Published crates:  gitkraft (GUI)  •  gitkraft-tui (TUI)"
    @echo "Internal crate:    gitkraft-core   (publish = false)"

# ── Housekeeping ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Update all dependencies (Cargo.lock only)
update:
    cargo update

# Update dependencies, run the full quality gate, then commit and push if all green.
update-deps:
    @echo "⬆️  Updating dependencies…"
    cargo update
    @echo "🔍 Running quality gate…"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings -A deprecated
    cargo test --workspace --locked --all-features --all-targets
    @echo "✅ All checks passed — committing dependency updates…"
    git add Cargo.lock
    git diff --cached --quiet || git commit -m "chore: update dependencies"
    git push origin main
    @echo "✅ Dependency updates pushed to GitHub."

# Show outdated dependencies (requires cargo-outdated)
outdated:
    cargo outdated

# Show the current workspace version
version: _check-nu
    @nu scripts/version.nu

# Show all configured remotes
remotes:
    @git remote -v

# ── Git remotes & pushing ────────────────────────────────────────────────────

# Push the current branch to GitHub (origin)
push:
    git push origin main

# Push the current branch to Gitea
push-gitea:
    git push gitea main

# Push the current branch to both GitHub and Gitea
push-all:
    git push origin main
    git push gitea main
    @echo "✅ Pushed to both GitHub and Gitea!"

# Pull the current branch from GitHub (origin)
pull:
    git pull origin main

# Pull the current branch from Gitea
pull-gitea:
    git pull gitea main

# Pull the current branch from both remotes
pull-all:
    git pull origin main
    git pull gitea main
    @echo "✅ Pulled from both GitHub and Gitea!"

# Push all tags to GitHub
push-tags:
    git push origin --tags

# Push all tags to both remotes
push-tags-all:
    git push origin --tags
    git push gitea --tags
    @echo "✅ Tags pushed to both remotes!"

# ── Release workflows ─────────────────────────────────────────────────────────

# Bump, commit, tag, then push to GitHub — triggers Release workflow.
release version: (bump version)
    @echo "Pushing release v{{ version }} to GitHub…"
    git push --follow-tags origin main
    @echo "✅ Release v{{ version }} pushed — Release workflow will trigger automatically."
    @echo "   https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]//' | sed 's/\.git//')/actions"

# Bump, commit, tag, then push to Gitea only.
release-gitea version: (bump version)
    @echo "Pushing release v{{ version }} to Gitea…"
    git push --follow-tags gitea main
    @echo "✅ Release v{{ version }} live on Gitea."

# Bump, commit, tag, then push to both GitHub and Gitea.
release-all version: (bump version)
    @echo "Pushing release v{{ version }} to all remotes…"
    git push --follow-tags origin main
    git push --follow-tags gitea main
    @echo "✅ Release v{{ version }} pushed to GitHub and Gitea!"

# Push the latest commit and all tags to every remote (no bump).
push-release-all: check-all
    git push --follow-tags origin main
    git push --follow-tags gitea main
    @echo "✅ Latest commit + tags pushed to all remotes."

# Manually re-trigger the Release workflow for an existing tag via the gh CLI.
release-retrigger version:
    @command -v gh >/dev/null 2>&1 || { \
        echo "❌ GitHub CLI (gh) not found. Install from https://cli.github.com"; exit 1; \
    }
    @echo "Manually dispatching Release workflow for tag v{{ version }}…"
    gh workflow run release.yml --field tag=v{{ version }}
    @echo "✅ Dispatched — check progress at: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions"

# Force-sync Gitea with GitHub
sync-gitea:
    git push gitea main --force
    git push gitea --tags --force
    @echo "✅ Gitea force-synced with GitHub."

# Add a Gitea remote and optionally push — interactive (nu script)
setup-gitea url: _check-nu
    nu scripts/setup_gitea.nu {{ url }}

# Migrate this project to dual GitHub + Gitea hosting (interactive)
migrate-gitea: _check-nu
    nu scripts/migrate_to_gitea.nu
