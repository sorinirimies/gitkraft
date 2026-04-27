# Changelog

All notable changes to this project will be documented in this file.

## [0.8.1] - 2026-04-27

### ✨ Features
- **Staging area range selection (TUI)** — Shift+↑/↓ or `J`/`K` while in the Staging pane extends the file selection range; numbered rank badges replace the plain `●` bullet when 2+ files are selected; anchor tracks the starting position so Shift+Up can shrink what Shift+Down expanded
- **Diff file list range selection fix (TUI)** — `J`/`K` now properly build an anchor-based range (using `ascending_range`), replace (not accumulate) the selection on each press, and trigger background diff loads for **all** selected files so the concatenated multi-file diff renders immediately rather than showing "Loading…" indefinitely
- **Keyboard enhancement (TUI)** — `PushKeyboardEnhancementFlags(DISAMBIGUATE_ESCAPE_CODES)` is enabled at startup on terminals that support it (Kitty, Alacritty, WezTerm, iTerm2 with xterm-keys); this makes Shift+arrow keys carry the SHIFT modifier flag; `J`/`K` uppercase aliases remain as a universal fallback for all other terminals
- **Files panel hint** — the Files panel title now shows `[J/K select]` when focused and single-file, and `[J/K shrink]` when a range is active
- **Blame view auto-close** — clicking any commit in the GUI commit log now automatically closes the blame overlay; same for `j`/`k`/`Enter` navigation in the TUI commit log

### 🐛 Bug Fixes
- **Diff file list Shift+Up/Down never worked** — the old `extend_file_selection` inserted only two individual items and never built a real range; no anchor was tracked; at the boundary it returned early with no feedback; all fixed
- **Multi-file concatenated diff was blank** — background loads were only triggered for the focused file; every other file in the selection stayed as "Loading…"; now all selected files are loaded in parallel
- **Staging J/K boundary now gives feedback** — pressing J at the last file (or K at the first) previously returned silently; now the anchor-to-current range is still applied so at least the current file gets selected
- **Plain j/k in staging no longer resets anchor while range is active** — previously navigating with j/k inside an existing multi-selection would reset the anchor, breaking subsequent Shift extends
- **GUI settings file keyboard shortcut** — `Ctrl/Cmd` shortcuts (including `Ctrl+,`) now fire regardless of widget focus; previously they were blocked when a text input had keyboard focus
- **GUI blame close button** — made visible with `toolbar_button` style and `[Esc]` label; `Esc` key now closes the blame overlay from anywhere in the GUI
- **TUI settings file opens browser** — `xdg-open` / `open` is no longer used as fallback for settings files (JSON files are often browser-associated); only the configured editor is used; if no editor is configured the file path is shown with a hint
- **TUI editor fallback** — `load_tui_settings` now inherits `editor_name` from the GUI's `settings.json` when the TUI has no editor configured, so users only need to configure their editor once

### 🔧 Refactoring / Internal
- **`RepoSnapshot` + `load_repo_snapshot` in core** — the identical 8-call repo loading sequence was duplicated verbatim in both frontends; moved to `gitkraft-core` as a single canonical function; both `load_repo_blocking` implementations now delegate to it in one line
- **`ascending_range(anchor, target)` utility in core** — the 5-line anchor-range computation was duplicated across GUI `SelectCommit`, TUI `select_commit_down/up`, and TUI `select_file_down/up`; now a shared `gitkraft_core::ascending_range` utility
- **Mirror function collapse (TUI)** — `navigate_down`/`navigate_up` in commit events collapsed to a single `navigate_to(app, closure)` helper; `select_commit_down`/`select_commit_up` collapsed to `extend_commit_selection`; `select_file_down`/`select_file_up` collapsed to `extend_file_selection`; `select_down`/`select_up` in staging collapsed to `extend_staging_selection`
- **`with_repo!` applied consistently (GUI)** — five handlers in `commits/update.rs` that used raw `if let Some(path) = repo_path` guards converted to use the existing `with_repo!` macro
- **Mandatory test rule in `.zed/rules.md`** — every feature must be accompanied by tests; rule is at the top of the file and cross-referenced from Common Pitfalls #1

## [0.8.0] - 2026-04-26

### ✨ Features
- **Multi-file selection in GUI commit diff** — Shift+Click selects a range of files; combined diff shown with `══ filename ══` separators; numbered selection badges; context menu adapts to "N files selected"
- **Multi-file selection in TUI commit diff** — Shift+Up/Down for range selection; numbered badges; combined diff in the diff pane
- **Multi-commit range selection in GUI** — Shift+Click selects a commit range; numbered badges; range diff (combined net diff) displayed in the diff panel
- **Multi-commit range selection in TUI** — Shift+Up/Down for range selection; combined range diff auto-loaded; Space for toggle-select
- **Combined commit range diff** — both GUI and TUI show the net diff across all selected commits (`git diff oldest^ newest`)
- **Diff sub-pane navigation in TUI** — Right arrow enters the diff content area; Left arrow returns to the file list; sub-pane highlighted with active border
- **Cherry-pick commits from context menu** — GUI context menu for N selected commits includes "Cherry-pick N commits"
- **Checkout file from commit** — right-click a file in the commit diff list → "Checkout file from this commit" (restores the file to its committed state)
- **Unified file context menus** — commit diff file menus reorganised into Actions / Copy info / Open sections; "Copy filename", "Open in default program" added throughout; multi-file menus show "N files selected"
- **Settings file editor** — `Ctrl/Cmd+,` in GUI and `,` in TUI open `settings.json` / `tui-settings.json` in the configured editor (terminal editors suspend the TUI, GUI editors use platform activation); `,` also works from the Options panel and Welcome screen
- **Window geometry persistence in GUI** — window size and position are saved on every resize/move and restored on next launch
- **Blame view exit** — `Esc` key closes blame in GUI; close button is now prominently styled with `[Esc]` label; clicking a different commit automatically exits blame in both GUI and TUI

### 🐛 Bug Fixes
- **JSON persistence replaces redb** — settings are now stored in plain JSON (`settings.json` / `tui-settings.json`); redb was wiped on every version upgrade due to format incompatibility; atomic writes (write-tmp → rename) prevent corruption on crash
- **Separate TUI and GUI settings files** — `tui-settings.json` for TUI, `settings.json` for GUI; opening repos in one frontend no longer clobbers the other's session; TUI falls back to GUI's `editor_name` when no TUI editor is configured
- **Terminal editors in TUI** — Helix, Neovim, Vim etc. now work correctly by suspending the TUI (leave alternate screen), running the editor synchronously with a real TTY, then resuming; previously `Stdio::null()` caused silent failure
- **Helix binary resolution** — platform-aware: macOS tries `hx` first, Linux tries `helix` first; no runtime probing during the TUI event loop
- **GUI keyboard shortcuts fire regardless of widget focus** — `Ctrl/Cmd` shortcuts (including `Ctrl+,`) now work even when a text input has keyboard focus
- **Multi-repo tab screen restoration** — switching tabs with `[`/`]` now correctly shows Main or Welcome based on whether the target tab has a repo loaded
- **Diff auto-loads on commit navigation** — navigating the TUI commit log with `j`/`k` or arrow keys now automatically loads the diff for the selected commit
- **Browser not opened for JSON settings** — `xdg-open` / `open` is no longer used for settings files (JSON is often browser-associated); only the configured editor is used

### 🔧 Chores / Internal
- **Mandatory test rule added to `.zed/rules.md`** — every feature implementation must be accompanied by tests
- All new features covered by unit tests across `gitkraft-core`, `gitkraft-gui`, and `gitkraft-tui`

## 0.7.7 - 2026-04-24
### ➕ Added
- Add split diff sub-pane navigation and multi-file select
- Add multi-file diff selection and viewing to GUI
- Add multi-commit selection and range diff support
### 🔧 Chores
- chore(deps): nightly dependency upgrade 2026-04-24
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.7.4...v0.7.7
## 0.7.4 - 2026-04-23
### ✨ Features
- feat(gui): diff search results against working tree
- feat(gui): 'Diff Selected' button for multi-file diff in search
- feat: auto-focus search input, add tests for new features
### 🐛 Bug Fixes
- fix(tui): open repo in new tab when current tab already has one
- fix(gui): prevent search dialog from dismissing when clicking diff content
### 📚 Documentation
- docs: regenerate all TUI VHS GIFs with updated layout
### 📦 Other Changes
- ui(tui): full-height overlay panels and expanded staging Actions
- ui(gui): add close button to right panel headers in search overlay
### 🔧 Chores
- chore: bump version to 0.7.4
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.7.3...v0.7.4
## 0.7.3 - 2026-04-23
### 🐛 Bug Fixes
- fix doc comments
### 🔧 Chores
- chore: bump version to 0.7.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.7.2...v0.7.3
## 0.7.2 - 2026-04-23
### ➕ Added
- Add editor selection support to GUI and TUI
### 🔧 Chores
- chore: bump version to 0.7.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.7.1...v0.7.2
## 0.7.1 - 2026-04-23
### ➕ Added
- Add commit search feature to GUI and TUI
### 📚 Documentation
- docs: add VHS-generated preview GIFs tracked with Git LFS
- docs: remove non-UI GIFs from Preview section
- docs: hide cargo run from TUI tapes and remove tui-build GIF
- docs: add TUI theme selector and multi-repo tabs VHS tapes
- docs: rewrite tui-tabs tape to open a real second repo
### 📦 Other Changes
- Remove tui-build.tape example from vhs directory
### 🔄 Updated
- Update dependencies in Cargo.lock for rfd and iced_fonts
### 🔧 Chores
- chore(deps): nightly dependency upgrade 2026-04-22
- chore: add .zed/rules.md with project conventions for agents
- chore(deps): nightly dependency upgrade 2026-04-23
- chore: bump version to 0.7.0
- chore: bump version to 0.7.1
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.6...v0.7.1
## 0.6.6 - 2026-04-21
### 🔧 Chores
- chore: bump version to 0.6.6
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.5...v0.6.6
## 0.6.5 - 2026-04-21
### ♻️ Refactor
- Refactor header toolbar into left and right item rows
### 🔧 Chores
- chore: bump version to 0.6.5
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.4...v0.6.5
## 0.6.4 - 2026-04-21
### 📦 Other Changes
- Simplify and update README features and layout sections
- Close context menu when checking out or deleting branch
### 🔄 Updated
- Update Iced to 0.14 and refactor for new widget APIs
### 🔧 Chores
- chore: bump version to 0.6.4
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.3...v0.6.4
## 0.6.3 - 2026-04-20
### 🔧 Chores
- chore: bump version to 0.6.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.6.2...v0.6.3
## 0.6.2 - 2026-04-20
### ➕ Added
- Add loading spinner to login button
- Add push-all-force recipe to force-push main to all remotes
### 📦 Other Changes
- Replace dtolnay/rust-toolchain with manual rustup install in CI
- Make all multi-remote tasks continue on failure
- Bump version to 0.6.0 and update changelog
- Remove blank lines before dependencies in Cargo.lock
- Simplify deps-update workflow to push updates directly to main
- Bump version to 0.6.1 and update changelog
- Move repo state to per-tab struct and update all usages
- Hide context menu when copying text
- Replace GitHub release action with Gitea API script
### 🔧 Chores
- chore: bump version to 0.6.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.7...v0.6.2
## 0.5.7 - 2026-04-19
### 🔧 Chores
- chore: bump version to 0.5.7
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.6...v0.5.7
## 0.5.6 - 2026-04-19
### 🔧 Chores
- chore: bump version to 0.5.6
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.5...v0.5.6
## 0.5.5 - 2026-04-19
### ♻️ Refactor
- Refactor row! macro usage for consistency and readability
### ➕ Added
- Add support for Gitea Starscream remote in justfile
### 📦 Other Changes
- Move release notes generation to scripts/ci and update workflow
- Replace Space::with_width/with_height with Space::new throughout GUI
### 🔧 Chores
- chore: bump version to 0.5.5
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.4...v0.5.5
## 0.5.4 - 2026-04-17
### ♻️ Refactor
- Refactor GUI view helpers and TUI commit diff loading
### 📦 Other Changes
- Fail bump if version is unchanged
### 🔧 Chores
- chore: bump version to 0.5.4
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.3...v0.5.4
## 0.5.3 - 2026-04-17
### ➕ Added
- Add step to install Rust stable in release workflow
### 🔧 Chores
- chore: bump version to 0.5.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.2...v0.5.3
## 0.5.2 - 2026-04-17
### 📦 Other Changes
- Simplify README to remove toolkit mentions and dependency tables
- Remove musl and aarch64 targets from release workflow
### 🔧 Chores
- chore: bump version to 0.5.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.1...v0.5.2
## 0.5.1 - 2026-04-17
### ➕ Added
- Add VHS demo GIF tasks to justfile and improve musl build
### 🔧 Chores
- chore: bump version to 0.5.1
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.5.0...v0.5.1
## 0.5.0 - 2026-04-17
### 📦 Other Changes
- Remove Tokio and futures dependencies from TUI and switch to std::mpsc
### 🔄 Updated
- Update release workflow to fix artifact handling and tool installs
### 🔧 Chores
- chore: bump version to 0.5.0
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.4.3...v0.5.0
## 0.4.3 - 2026-04-17
### ♻️ Refactor
- Refactor key event handlers to reduce nesting and improve clarity
### 🔧 Chores
- chore: bump version to 0.4.3
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.4.2...v0.4.3
## 0.4.2 - 2026-04-17
### 📦 Other Changes
- Sort recent repos using sort_by_key with Reverse
### 🔄 Updated
- Update install-tools to install nu if missing
### 🔧 Chores
- chore: bump version to 0.4.2
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.4.1...v0.4.2
## 0.4.1 - 2026-04-17
### ➕ Added
- Add remote branch delete/checkout, icons module, and file list diff
- Add UI zoom support with keyboard shortcuts and status bar indicator
- Add tests for core types and refactor repo open logic
### 📦 Other Changes
- Reformat codebase with rustfmt and improve formatting checks
### 🔧 Chores
- chore: bump version to 0.4.1
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.4.0...v0.4.1
## 0.4.0 - 2026-04-16
### ➕ Added
- Add collapsible branch sections and overlay scrollbars to GUI
- Add loading spinner to login button
- Add tag creation actions and truncate-to-fit utility
### 🔧 Chores
- chore: bump version to 0.4.0
**Full Changelog**: https://github.com/sorinirimies/gitkraft/compare/v0.3.9...v0.4.0
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
