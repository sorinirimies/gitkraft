# Changelog

All notable changes to this project will be documented in this file.

## 1.1.4 - 2026-06-26
### 🐛 Bug Fixes
- fix(ci): resolve all 4 release build failures
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.3...v1.1.4
## 1.1.3 - 2026-06-26
### ♻️ Refactor
- Refactor key event handlers to reduce nesting and improve clarity
- Refactor GUI view helpers and TUI commit diff loading
- Refactor row! macro usage for consistency and readability
- Refactor header toolbar into left and right item rows
- Refactor staging selection logic and context menu handling
- Refactor staging file lists to use DiffFileEntry type
- refactor: harden safety, memory, and reliability across all crates
### ✨ Features
- feat(gui): diff search results against working tree
- feat(gui): 'Diff Selected' button for multi-file diff in search
- feat: auto-focus search input, add tests for new features
- feat: multi-platform packaging and distribution
### ➕ Added
- Add collapsible branch sections and overlay scrollbars to GUI
- Add loading spinner to login button
- Add tag creation actions and truncate-to-fit utility
- Add remote branch delete/checkout, icons module, and file list diff
- Add UI zoom support with keyboard shortcuts and status bar indicator
- Add tests for core types and refactor repo open logic
- Add VHS demo GIF tasks to justfile and improve musl build
- Add step to install Rust stable in release workflow
- Add support for Gitea Starscream remote in justfile
- Add loading spinner to login button
- Add push-all-force recipe to force-push main to all remotes
- Add commit search feature to GUI and TUI
- Add editor selection support to GUI and TUI
- Add split diff sub-pane navigation and multi-file select
- Add multi-file diff selection and viewing to GUI
- Add multi-commit selection and range diff support
- Add tests for cherry-pick and commit event handling
- Add reactive git-state watcher using notify crate
- Add animated loading spinner to GUI and TUI using tui-spinner
- Add skeleton loading screen using tui-skeleton
- Add theme background to all panes and update theme switching
- Add Cyberpunk theme and update theme count references
- Add 15 new themes and update theme picker to 43 total
- Add draggable dividers for staging and sidebar panels
- Add multi-file open and preview actions to context menus
- Add tests for tab and repo closing and switching scenarios
- Add utility functions for path, selection, and list navigation
- Add commit message and ref name validation with inline hints
- Add stash apply, force-push, and branch rename features stash apply
- Add pannable canvas commit graph and all-ref revwalk
- Add tests and GUI helpers for preview and popups
### 🐛 Bug Fixes
- fix doc comments
- fix(tui): open repo in new tab when current tab already has one
- fix(gui): prevent search dialog from dismissing when clicking diff content
- Fix branch button highlight and hide delete for current branch
- Fix tab targeting for repo refresh and deduplication
- fix: broken rustdoc link to private render_main function
- Fix tab closing behavior and add tests for CloseRepo and CloseTab
- Fix indentation of refs field in test data
- Fix workspace section header in Cargo.toml
- Fix CI concurrency to properly cancel stale runs
- Fix stale commit display and refresh race
- fix: resolve clippy errors blocking release gate
### 📈 Improvements
- Improve git watcher efficiency and add stash list tests
### 📚 Documentation
- docs: add VHS-generated preview GIFs tracked with Git LFS
- docs: remove non-UI GIFs from Preview section
- docs: hide cargo run from TUI tapes and remove tui-build GIF
- docs: add TUI theme selector and multi-repo tabs VHS tapes
- docs: rewrite tui-tabs tape to open a real second repo
- docs: regenerate all TUI VHS GIFs with updated layout
### 📦 Other Changes
- Reformat codebase with rustfmt and improve formatting checks
- Sort recent repos using sort_by_key with Reverse
- Remove Tokio and futures dependencies from TUI and switch to std::mpsc
- Simplify README to remove toolkit mentions and dependency tables
- Remove musl and aarch64 targets from release workflow
- Fail bump if version is unchanged
- Move release notes generation to scripts/ci and update workflow
- Replace Space::with_width/with_height with Space::new throughout GUI
- Replace dtolnay/rust-toolchain with manual rustup install in CI
- Make all multi-remote tasks continue on failure
- Bump version to 0.6.0 and update changelog
- Remove blank lines before dependencies in Cargo.lock
- Simplify deps-update workflow to push updates directly to main
- Bump version to 0.6.1 and update changelog
- Move repo state to per-tab struct and update all usages
- Hide context menu when copying text
- Replace GitHub release action with Gitea API script
- Simplify and update README features and layout sections
- Close context menu when checking out or deleting branch
- Remove tui-build.tape example from vhs directory
- ui(tui): full-height overlay panels and expanded staging Actions
- ui(gui): add close button to right panel headers in search overlay
- Release v0.8.1
- Release 0.8.5 with improved git watcher and UI features
- Preserve multi-selection across commit list refreshes
- Defer clearing multi-file diff state until diff load completes
- Preserve file selection and diffs across refresh if commit survives
- Show branch/tag/HEAD labels in commit log UI
- Remove unused functions and exports from core crate
- Show branch ahead/behind status in GUI and TUI
- Rename gitea_starscream remote to gitea-starscream
- Canonicalize repo paths and fix tab deduplication logic
- Remove unnecessary reference to menu_item in context_menu_panel
- Derive short OIDs from full IDs across core and UIs
### 🔄 CI
- ci: split release workflows by platform capability
### 🔄 Updated
- Update install-tools to install nu if missing
- Update release workflow to fix artifact handling and tool installs
- Update Iced to 0.14 and refactor for new widget APIs
- Update dependencies in Cargo.lock for rfd and iced_fonts
- Update tui-themes.gif
- Update dependencies in Cargo.lock
- Update dependencies in Cargo.lock
- Update badges and theme count in README
- Update download badges in README with clearer labels
- Update VHS demo GIFs and theme tape for new theme
- Update dependencies in Cargo.lock
### 🔧 Chores
- chore: bump version to 0.4.0
- chore: bump version to 0.4.1
- chore: bump version to 0.4.2
- chore: bump version to 0.4.3
- chore: bump version to 0.5.0
- chore: bump version to 0.5.1
- chore: bump version to 0.5.2
- chore: bump version to 0.5.3
- chore: bump version to 0.5.4
- chore: bump version to 0.5.5
- chore: bump version to 0.5.6
- chore: bump version to 0.5.7
- chore: bump version to 0.6.2
- chore: bump version to 0.6.3
- chore: bump version to 0.6.4
- chore: bump version to 0.6.5
- chore: bump version to 0.6.6
- chore(deps): nightly dependency upgrade 2026-04-22
- chore: add .zed/rules.md with project conventions for agents
- chore(deps): nightly dependency upgrade 2026-04-23
- chore: bump version to 0.7.0
- chore: bump version to 0.7.1
- chore: bump version to 0.7.2
- chore: bump version to 0.7.3
- chore: bump version to 0.7.4
- chore(deps): nightly dependency upgrade 2026-04-24
- chore: bump version to 0.7.7
- chore: bump version to 0.8.2
- chore(deps): nightly dependency upgrade 2026-04-28
- chore: bump version to 0.8.3
- chore: bump version to 0.8.4
- chore: bump version to 0.8.6
- chore(deps): nightly dependency upgrade 2026-04-30
- chore: bump version to 0.8.7
- chore: bump version to 0.8.8
- chore(deps): nightly dependency upgrade 2026-05-01
- chore(deps): nightly dependency upgrade 2026-05-02
- chore: bump version to 0.9.0
- chore: bump version to 0.9.1
- chore: bump version to 0.9.2
- chore: bump version to 0.9.3
- chore: bump version to 0.9.4
- chore: bump version to 0.9.5
- chore: bump version to 1.0.0
- chore: bump version to 1.0.1
- chore: bump version to 1.0.2
- chore: bump version to 1.0.3
- chore: bump version to 1.0.4
- chore(deps): nightly dependency upgrade 2026-05-12
- chore: bump version to 1.0.5
- chore: bump version to 1.0.6
- chore(deps): nightly dependency upgrade 2026-05-14
- chore(deps): nightly dependency upgrade 2026-05-16
- chore: bump version to 1.0.7
- chore: bump version to 1.0.9
- chore: bump version to 1.1.0
- chore: bump version to 1.1.1
- chore: bump version to 1.1.2
- chore: bump version to 1.1.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.9...v1.1.3
## 0.3.9 - 2026-04-14
### 📦 Other Changes
- Make branches sidebar width responsive
### 🔧 Chores
- chore: bump version to 0.3.9
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.8...v0.3.9
## 0.3.8 - 2026-04-14
### ♻️ Refactor
- Refactor async command handling with git_task! and with_repo! macros
### ➕ Added
- Add discard confirmation to GUI and improve TUI features
- Add context menu support for branches and commits in GUI
- Add git reset (soft, mixed, hard) to commit context menu
### 📦 Other Changes
- Persist and restore open tabs and active tab index
- Implement virtual scrolling and lazy loading for commit log
- Bump version to 0.3.7 and update doc code blocks
### 🔧 Chores
- chore: bump version to 0.3.8
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.6...v0.3.8
## 0.3.6 - 2026-04-12
### ➕ Added
- Add multi-repo tab bar with per-tab state management
### 🔧 Chores
- chore: bump version to 0.3.6
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.5...v0.3.6
## 0.3.5 - 2026-04-09
### 🔧 Chores
- chore: bump version to 0.3.5
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.4...v0.3.5
## 0.3.4 - 2026-04-05
### ♻️ Refactor
- Refactor release workflow to use bash for tag validation and publishing
### 🔧 Chores
- chore: bump version to 0.3.4
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.3...v0.3.4
## 0.3.3 - 2026-04-05
### 📦 Other Changes
- Strip leading v from version argument in release_prepare.nu
### 🔧 Chores
- chore: bump version to 0.3.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.2...v0.3.3
## 0.3.2 - 2026-04-05
### 🐛 Bug Fixes
- fix(ci): remove macOS x86_64 build target (no macos-13 runner available)
### 🔧 Chores
- chore: bump version to 0.3.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.1...v0.3.2
## 0.3.1 - 2026-04-04
### 🐛 Bug Fixes
- fix(ci): use vcpkg zlib on Windows to fix libz-sys build
### 📦 Other Changes
- Limit release workflow concurrency to one at a time
### 🔧 Chores
- chore: bump version to 0.3.1
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.0...v0.3.1
## 0.3.0 - 2026-04-04
### 🐛 Bug Fixes
- Fix artifact staging by passing matrix values via env
### 📦 Other Changes
- Merge pull request #3 from sorinirimies/dependabot/github_actions/actions/download-artifact-8
- Merge pull request #2 from sorinirimies/dependabot/github_actions/actions/checkout-6
- Merge pull request #1 from sorinirimies/dependabot/github_actions/actions/upload-artifact-7
### 🔄 CI
- ci(deps): bump actions/download-artifact from 4 to 8
- ci(deps): bump actions/checkout from 5 to 6
- ci(deps): bump actions/upload-artifact from 4 to 7
### 🔧 Chores
- chore: bump version to 0.3.0
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.2.3...v0.3.0
## 0.2.3 - 2026-04-04
### ➕ Added
- Add Dependabot and auto-merge for GitHub Actions updates
### 🔧 Chores
- chore: bump version to 0.2.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.2.2...v0.2.3
## 0.2.2 - 2026-04-04
### ➕ Added
- Add validate-tag recipe to check version tag format
### 📦 Other Changes
- Print validation messages to stderr instead of stdout
### 🔧 Chores
- chore: bump version to 0.2.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.2.1...v0.2.2
## 0.2.1 - 2026-04-04
### ➕ Added
- Add macOS targets to release workflow and fix doc links
### 🔧 Chores
- chore: bump version to 0.2.1
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.2.0...v0.2.1
## 0.2.0 - 2026-04-04
### 📦 Other Changes
- Remove Windows cross-compilation via cross from CI and release
### 🔧 Chores
- chore: bump version to 0.2.0
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.9...v0.2.0
## 0.1.9 - 2026-04-04
### ➕ Added
- Add Nushell script tests to quality checks and improve test cleanup
### 🔧 Chores
- chore: bump version to 0.1.9
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.8...v0.1.9
## 0.1.8 - 2026-04-04
### 📦 Other Changes
- Switch Windows cross-compilation from cargo-xwin to cross
### 🔧 Chores
- chore: bump version to 0.1.8
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.7...v0.1.8
## 0.1.7 - 2026-04-04
### ➕ Added
- Add Nushell CI/release scripts and Windows cross-check
### 🔧 Chores
- chore: bump version to 0.1.7
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.6...v0.1.7
## 0.1.6 - 2026-04-04
### ♻️ Refactor
- Refactor staging refresh to run asynchronously
### 🔧 Chores
- chore: bump version to 0.1.6
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.4...v0.1.6
## 0.1.4 - 2026-04-04
### 🔄 Updated
- Update dependencies in Cargo.lock
### 🔧 Chores
- chore: bump version to 0.1.4
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.3...v0.1.4
## 0.1.3 - 2026-04-04
### 🔧 Chores
- chore: bump version to 0.1.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.1.2...v0.1.3
## 0.1.2 - 2026-04-04
### ♻️ Refactor
- Refactor and simplify logic in core modules and GUI state
- Refactor test helpers and commit labeling logic
### ➕ Added
- Add theme picker UI for GUI and TUI with runtime switching
- Add unified theme and settings persistence to core, GUI, and TUI
- Add draggable pane dividers and repo close action to GUI
- Add CI, release, and dependency update workflows, scripts, and project
### 📦 Other Changes
- iniital implementation of the gitkraft gui and tui editor written fully
### 🔧 Chores
- chore: bump version to 0.1.2
