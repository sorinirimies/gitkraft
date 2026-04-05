# Changelog

All notable changes to this project will be documented in this file.

## 0.3.4 - 2026-04-05
### ♻️ Refactor
- Refactor release workflow to use bash for tag validation and publishing
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
