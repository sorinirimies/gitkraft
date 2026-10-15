# Changelog

All notable changes to this project will be documented in this file.

## [1.2.8] - 2026-10-15
### ✨ Features
- use tui-file-explorer (lightweight, no cli) for the o directory browser
- move Settings and About into the header next to Fetch/Open/Close; keep version in tab bar
### 🔄 CI
- prebuilt Nushell, cargo-deny + cargo-audit security workflow
### 🔧 Chores
- push/pull current branch instead of hardcoded main
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.6...v1.2.8
## [1.2.6] - 2026-10-14
### ✨ Features
- deeper responsive layout — adaptive commit columns, diff min width, compact toolbars, stacked staging, clamped file list
- responsive TUI layout, clamped GUI dialogs and sidebar scrim, System theme option (GUI + TUI)
- header shortcut hints adapt to terminal width
- vertical compact mode for short windows
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.5...v1.2.6
## [1.2.5] - 2026-10-14
### ✨ Features
- responsive layout with clamped panes and overlay sidebar on narrow windows
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.4...v1.2.5
## [1.2.4] - 2026-10-13
### 🔄 CI
- split fuzz into own workflow; add audit & fuzz badges to README
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.2...v1.2.4
## [1.2.2] - 2026-10-13
### 🐛 Bug Fixes
- ignore inotify access/lock events to stop refresh loop; chore(deps): update lockfile; ci: fix nightly deps toolchain setup
- avoid panics in graph column indexing and short_oid_str on multibyte input
### 💄 Style
- cargo fmt watcher
### 🔄 CI
- add snap and flathub publishing, icons, desktop/metainfo, AUR desktop entry
- add cargo-audit and cargo-fuzz targets with nightly workflows
- run cargo audit in release gate; add audit and fuzz-smoke to just check-release
- bump actions/upload-artifact from 4 to 7
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.1...v1.2.2
## [1.2.1] - 2026-10-10
### 🐛 Bug Fixes
- unresolved rustdoc link to macos
### 🔄 CI
- install alien, fakeroot and arm64 libc headers for release builds
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.2.0...v1.2.1
## [1.2.0] - 2026-10-09
### ✨ Features
- add version label, settings and about menu items to GUI and TUI
### 🐛 Bug Fixes
- add the macOS Window menu (window module) so Zoom, Move & Resize and the tiling shortcuts work
### 📚 Documentation
- regenerate VHS demo GIFs, add GUI screenshots + recording to README
- refresh README — fix stale info, expand keyboard shortcuts, no preview changes
### 🔧 Chores
- update dependencies
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.6...v1.2.0
## [1.1.6] - 2026-08-02
### 🔄 CI
- fix nu parse errors in package_linux.nu; add test coverage
- fix runtime bug in package_linux.nu; add Windows icon fallback
- fix Windows icon fallback size, AppImage FUSE, and rpm cross-arch build
- replace PowerShell/System.Drawing icon fallback with a verified static ICO
- resolve Windows installer icon to an absolute path
- drop rpmbuild --target, fix cross-arch aarch64 .rpm builds for real
- anchor all installer.nsi relative paths to the repo root via !cd
- replace rpmbuild with alien for .rpm packaging (cross-arch never worked)
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.5...v1.1.6
## [1.1.5] - 2026-08-02
### 🐛 Bug Fixes
- fix remaining 3 build failures (Nu date, Windows makensis path)
### 🔄 CI
- bump actions/checkout from 4 to 7
- bump actions/download-artifact from 4 to 8
- bump KSXGitHub/github-actions-deploy-aur from 2 to 3
- bump actions/upload-artifact from 4 to 7
### 🔧 Chores
- align git-cliff and changelog setup with tui-file-explorer
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.4...v1.1.5
## [1.1.4] - 2026-06-26
### 🐛 Bug Fixes
- resolve all 4 release build failures
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.3...v1.1.4
## [1.1.3] - 2026-06-26
### ♻️  Refactor
- harden safety, memory, and reliability across all crates
### ✨ Features
- multi-platform packaging and distribution
### 🐛 Bug Fixes
- resolve clippy errors blocking release gate
### 🔄 CI
- split release workflows by platform capability
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v1.1.2...v1.1.3
## [0.9.0] - 2026-05-02
### 🐛 Bug Fixes
- broken rustdoc link to private render_main function
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.8.8...v0.9.0
## [0.7.4] - 2026-04-23
### ✨ Features
- diff search results against working tree
- 'Diff Selected' button for multi-file diff in search
- auto-focus search input, add tests for new features
### 🐛 Bug Fixes
- open repo in new tab when current tab already has one
- prevent search dialog from dismissing when clicking diff content
### 📚 Documentation
- regenerate all TUI VHS GIFs with updated layout
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.7.3...v0.7.4
## [0.7.1] - 2026-04-23
### 📚 Documentation
- add VHS-generated preview GIFs tracked with Git LFS
- remove non-UI GIFs from Preview section
- hide cargo run from TUI tapes and remove tui-build GIF
- add TUI theme selector and multi-repo tabs VHS tapes
- rewrite tui-tabs tape to open a real second repo
### 🔧 Chores
- add .zed/rules.md with project conventions for agents
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.6...v0.7.1
## [0.3.2] - 2026-04-05
### 🐛 Bug Fixes
- remove macOS x86_64 build target (no macos-13 runner available)
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.1...v0.3.2
## [0.3.1] - 2026-04-04
### 🐛 Bug Fixes
- use vcpkg zlib on Windows to fix libz-sys build
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.0...v0.3.1
## [0.3.0] - 2026-04-04
### 🔄 CI
- bump actions/download-artifact from 4 to 8
- bump actions/checkout from 5 to 6
- bump actions/upload-artifact from 4 to 7
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.2.3...v0.3.0
## [0.1.2] - 2026-04-04
