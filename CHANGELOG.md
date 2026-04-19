# Changelog

All notable changes to this project will be documented in this file.

## 0.5.6 - 2026-04-19
### 🔧 Chores
- chore: bump version to 0.5.6
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.5...v0.5.6
## 0.5.5 - 2026-04-19
### ♻️ Refactor
- Refactor key event handlers to reduce nesting and improve clarity
- Refactor GUI view helpers and TUI commit diff loading
- Refactor row! macro usage for consistency and readability
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
### 📦 Other Changes
- Reformat codebase with rustfmt and improve formatting checks
- Sort recent repos using sort_by_key with Reverse
- Remove Tokio and futures dependencies from TUI and switch to std::mpsc
- Simplify README to remove toolkit mentions and dependency tables
- Remove musl and aarch64 targets from release workflow
- Fail bump if version is unchanged
- Move release notes generation to scripts/ci and update workflow
- Replace Space::with_width/with_height with Space::new throughout GUI
### 🔄 Updated
- Update install-tools to install nu if missing
- Update release workflow to fix artifact handling and tool installs
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
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.9...v0.5.5
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
