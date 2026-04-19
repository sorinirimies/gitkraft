<div align="center">

# ⚡ GitKraft

**A Git IDE written entirely in Rust — desktop GUI & terminal UI**

[![Crates.io](https://img.shields.io/crates/v/gitkraft.svg)](https://crates.io/crates/gitkraft)
[![docs.rs](https://docs.rs/gitkraft-core/badge.svg)](https://docs.rs/gitkraft-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

[Features](#features) · [Installation](#installation) · [Building from Source](#building-from-source) · [Architecture](#architecture) · [Themes](#themes) · [Development](#development) · [License](#license)

</div>

---

GitKraft is a pure-Rust Git IDE that ships two front-ends from a single workspace:
| Binary | Use case |
|--------|----------|
| `gitkraft` | Desktop GUI — mouse, drag-to-resize panes, canvas-based commit graph |
| `gitkraft-tui` | Terminal UI — great for SSH sessions, headless machines, or keyboard-only workflows |

Both binaries share **`gitkraft-core`**, a framework-free library that wraps [libgit2](https://libgit2.org) via the [`git2`](https://crates.io/crates/git2) crate. Zero Git operations live in the UI layer.

## GUI Layout

The desktop application starts **maximised** and presents a multi-pane layout. Every divider is **draggable** and the layout is **persisted** across sessions (via [redb](https://crates.io/crates/redb)).

```
┌──────────────────────────────────────────────┐
│  header toolbar                              │
├────────┬──────────────────┬──────────────────┤
│        │                  │                  │
│ side-  │  commit log      │  diff viewer     │
│ bar    │  (graph)         │                  │
│        │                  │                  │
├────────┴──────────────────┴──────────────────┤
│  staging area  (unstaged | staged | message) │
├──────────────────────────────────────────────┤
│  status bar                                  │
└──────────────────────────────────────────────┘
```

## TUI Layout

The terminal UI mirrors the GUI structure with a keyboard-driven interface:

```
┌──────────────────────────────────────────────┐
│  header bar  (repo │ branch │ shortcuts)     │
├────────┬──────────────┬──────────────────────┤
│        │              │                      │
│ side-  │  commit log  │  files + diff viewer │
│ bar    │              │                      │
│        │              │                      │
├────────┴──────────────┴──────────────────────┤
│  staging area  (unstaged │ staged │ actions) │
├──────────────────────────────────────────────┤
│  status bar                                  │
└──────────────────────────────────────────────┘
```

### TUI Keyboard Shortcuts

| Key | Context | Action |
|-----|---------|--------|
| ←/→ | Global | Switch panes |
| ↑/↓ | Any pane | Navigate within pane |
| j/k | Any pane | Vim-style up/down |
| h/l | Diff pane | Switch files |
| Enter | Commits | Load diff |
| Enter | Staging | View diff |
| s | Staging | Stage file |
| u | Staging | Unstage file |
| S/U | Staging | Stage/Unstage all |
| c | Staging | Commit |
| d | Staging | Discard (press twice) |
| z/Z | Global | Stash save/pop |
| o | Global | Browse & open repo |
| W | Main | Close repo |
| r | Main | Refresh |
| f | Main | Fetch |
| T | Main | Theme picker |
| O | Main | Options |
| Tab | Main | Cycle panes |
| q | Global | Quit |

### GUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Ctrl/Cmd + + | Zoom in |
| Ctrl/Cmd + - | Zoom out |
| Ctrl/Cmd + 0 | Reset zoom |

## Features

- **Pure Rust Git operations** — all Git work goes through `git2`; no shelling out to `git`.
- **Branch management** — list, create, checkout, and delete branches.
- **Remote branch management** — checkout, delete, and inspect remote-tracking branches via right-click context menu.
- **Commit log with graph visualisation** — canvas-rendered DAG in the GUI, box-drawing in the TUI.
- **Diff viewer** — working-directory diffs, staged diffs, and per-commit diffs with syntax-highlighted hunks.
- **Two-phase commit diff loading** — file list appears instantly, diffs load per-file on demand for fast responsiveness.
- **Commit files sidebar** — both GUI and TUI show a file list when viewing multi-file commit diffs.
- **Staging area** — stage / unstage individual files or all at once; discard working-directory changes.
- **Commit creation** — write a message and commit directly from the IDE.
- **Stash management** — save, pop, drop, and list stash entries with optional messages.
- **Remote listing & fetch** — view configured remotes and fetch from the primary remote.
- **Multi-tab support (GUI)** — open multiple repositories in tabs, session is persisted across restarts.
- **Context menus (GUI)** — right-click branches and commits for checkout, push, pull, rebase, merge, reset, revert, tag creation, copy SHA, etc.
- **UI zoom (GUI)** — Ctrl+/Ctrl- to zoom in/out (50%–200%), persisted across sessions.
- **Virtual scrolling** — commit log and diff views use virtual scrolling for smooth performance with large histories.
- **Interactive directory browser (TUI)** — press `o` to browse the filesystem and open a repo, with git repo detection.
- **27 built-in colour themes** — from Dracula and Nord to Catppuccin, Tokyo Night, Kanagawa, and more — persisted per-user.
- **Draggable pane dividers** — resize every panel; the layout is saved and restored automatically.
- **Recently opened repositories** — persisted list on the welcome screen for quick access.
- **Starts maximised** — the GUI opens full-screen by default.
- **TUI for terminal/SSH use** — the same core logic is available in a keyboard-driven terminal interface.

## Installation

### From crates.io

```sh
# Desktop GUI
cargo install gitkraft

# Terminal UI
cargo install gitkraft-tui
```

### Pre-built binaries

Download the latest release from the [GitHub Releases page](https://github.com/sorinirimies/gitkraft/releases).

## Building from Source

### Prerequisites

| Tool | Minimum version | Notes |
|------|-----------------|-------|
| **Rust** | 1.80+ (2021 edition) | Install via [rustup](https://rustup.rs) |
| **C compiler** | — | Required by `libgit2-sys` (usually pre-installed on Linux/macOS; MSVC on Windows) |
| **cmake** | 3.x | Required by `libgit2-sys` |
| **pkg-config** | — | Linux only |
| **libssl-dev** / **openssl** | — | For HTTPS remote operations |

### Clone & build

```sh
git clone https://github.com/sorinirimies/gitkraft.git
cd gitkraft

# Debug build (faster compilation)
cargo build

# Release build (optimised, LTO enabled)
cargo build --release

# Run the GUI
cargo run --release -p gitkraft

# Run the TUI
cargo run --release -p gitkraft-tui

# Run the TUI on a specific repo
cargo run --release -p gitkraft-tui -- /path/to/repo
```

## Architecture

### Workspace layout

```
gitkraft/
├── Cargo.toml                  ← workspace manifest
├── crates/
│   ├── gitkraft-core/          ★ shared Git logic — uses git2, no GUI/TUI deps
│   │   └── src/
│   │       ├── lib.rs
│   │       └── features/
│   │           ├── branches/    branch ops (list, create, delete, checkout)
│   │           ├── commits/     commit log, commit creation
│   │           ├── diff/        working dir diff, staged diff, commit diff
│   │           ├── graph/       commit graph visualisation
│   │           ├── log/         filtered commit search
│   │           ├── persistence/ settings & layout storage (redb)
│   │           ├── remotes/     remote listing
│   │           ├── repo/        open, init, repo info
│   │           ├── staging/     stage, unstage, discard
│   │           ├── stash/       stash save, pop, drop, list
│   │           └── theme/       27 built-in colour themes
│   │
│   ├── gitkraft-gui/           Desktop GUI application (binary: gitkraft)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── lib.rs
│   │       ├── state.rs        application state
│   │       ├── message.rs      all Message variants
│   │       ├── update.rs       TEA update function
│   │       ├── view.rs         layout: header, sidebar, commit log, diff, staging, status bar
│   │       ├── theme.rs        Theme integration
│   │       ├── features/       feature-specific views & updates
│   │       └── widgets/        reusable UI components (header, dividers)
│   │
│   └── gitkraft-tui/           Terminal UI application (binary: gitkraft-tui)
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── app.rs          TUI app state
│           ├── events.rs       keyboard event handler
│           ├── layout.rs       terminal layout renderer
│           ├── features/       feature-specific TUI modules
│           └── widgets/        reusable TUI widgets
│
├── scripts/                    Nushell automation scripts
│   ├── version.nu
│   ├── bump_version.nu
│   ├── check_publish.nu
│   ├── release_prepare.nu
│   ├── upgrade_deps.nu
│   ├── setup_gitea.nu
│   ├── migrate_to_gitea.nu
│   └── tests/
│       ├── runner.nu
│       ├── run_all.nu
│       └── test_*.nu
│
├── .github/workflows/          GitHub Actions CI/CD
│   ├── ci.yml
│   ├── release.yml
│   └── deps-update.yml
│
├── .gitea/workflows/           Gitea Actions CI/CD
│   ├── ci.yml
│   ├── release.yml
│   └── deps-update.yml
│
├── justfile                    Task runner (just)
├── cliff.toml                  git-cliff changelog config
├── LICENSE                     MIT
└── CHANGELOG.md
```

### The Elm Architecture (TEA)

The GUI crate follows **The Elm Architecture**:

```
           ┌──────────────┐
     ┌────▶│    State      │─────┐
     │     │  (state.rs)   │     │
     │     └──────────────┘     │
     │                          ▼
┌─────────┐              ┌──────────┐
│  Update  │◀─────────────│   View    │
│(update.rs│  Message     │ (view.rs) │
│)         │              │           │
└─────────┘              └──────────┘
```

| Component | File | Responsibility |
|-----------|------|----------------|
| **State** | `state.rs` | Single `GitKraft` struct holding *all* application state — repo info, branches, commits, diffs, staging, UI flags, pane dimensions, theme index, etc. |
| **Message** | `message.rs` | A flat `enum Message` with every possible event: user actions (`OpenRepo`, `StageFile`, `CheckoutBranch`), async results (`RepoOpened(Result<…>)`), and internal signals (`PaneDragMove`, `ThemeChanged`). |
| **Update** | `update.rs` | Pure `fn update(&mut self, message: Message) -> Task<Message>` that pattern-matches on the message, mutates state, and optionally returns an async `Task` whose result produces a new `Message`. |
| **View** | `view.rs` | Pure `fn view(&self) -> Element<Message>` that reads state and builds the entire widget tree. The framework diffs the tree and only redraws what changed. |

This architecture ensures a **unidirectional data flow**: the view never mutates state directly; it only emits `Message`s that the update function handles.

### Crate dependency map

```
  gitkraft-gui ──┐
                  ├──▶ gitkraft-core ──▶ git2, redb, chrono, serde …
  gitkraft-tui ──┘
```

Both front-ends depend only on `gitkraft-core` for Git operations; neither calls `git2` directly.

## Themes

GitKraft ships **27 built-in colour themes**. The active theme is persisted and applied to every widget automatically.

| # | Theme | # | Theme | # | Theme |
|---|-------|---|-------|---|-------|
| 0 | Default | 9 | Nord | 18 | Tokyo Night |
| 1 | Grape | 10 | Solarized Dark | 19 | Tokyo Night Storm |
| 2 | Ocean | 11 | Solarized Light | 20 | Tokyo Night Light |
| 3 | Sunset | 12 | Gruvbox Dark | 21 | Kanagawa Wave |
| 4 | Forest | 13 | Gruvbox Light | 22 | Kanagawa Dragon |
| 5 | Rose | 14 | Catppuccin Latte | 23 | Kanagawa Lotus |
| 6 | Mono | 15 | Catppuccin Frappé | 24 | Moonfly |
| 7 | Neon | 16 | Catppuccin Macchiato | 25 | Nightfly |
| 8 | Dracula | 17 | Catppuccin Mocha | 26 | Oxocarbon |

Themes are defined once in `gitkraft-core` and shared by both front-ends — change the theme in the GUI and the TUI will pick it up too.

## Development

### Task runner (`just`)

The project uses [`just`](https://github.com/casey/just) as a task runner. Install it with `cargo install just`.

```sh
just build            # cargo build --workspace
just run              # run the GUI
just run-tui          # run the TUI
just test             # cargo test --workspace
just clippy           # clippy with -D warnings
just fmt              # auto-format all code
just check-all        # full quality gate (fmt + clippy + test + nu)
just check-release    # quality gate + release build
just install-tools    # install git-cliff and nushell
just release 0.5.0    # bump, tag, push (runs all checks first)
just release-all 0.5.0 # release to GitHub + Gitea
just push-all         # push to both remotes
just changelog        # regenerate CHANGELOG.md
```

### Nushell scripts

The `scripts/` directory contains [Nushell](https://www.nushell.sh) automation scripts for versioning, release preparation, dependency upgrades, and a custom test runner:

| Script | Purpose |
|--------|---------|
| `version.nu` | Print the current workspace version |
| `bump_version.nu` | Bump the version across all crate manifests |
| `check_publish.nu` | Dry-run `cargo publish` for all crates |
| `release_prepare.nu` | Full release checklist (bump, changelog, tag) |
| `upgrade_deps.nu` | Upgrade workspace dependencies |
| `setup_gitea.nu` | Bootstrap a Gitea instance for mirroring |
| `migrate_to_gitea.nu` | Mirror the repo to Gitea |
| `tests/run_all.nu` | Run the full Nushell test suite |

### CI/CD

Continuous integration runs on both **GitHub Actions** and **Gitea Actions**:

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| `ci.yml` | Push / PR | `cargo fmt --check`, `cargo clippy`, `cargo test` |
| `release.yml` | Tag push | Build release binaries, create GitHub/Gitea release |
| `deps-update.yml` | Scheduled | Automated dependency update PRs |

### Dual Hosting

GitKraft is hosted on both **GitHub** and **Gitea**. Use `just push-all` to push to both remotes simultaneously, keeping them in sync. Release workflows run on both platforms so binaries are published to each.

### Changelog

The changelog is generated with [`git-cliff`](https://git-cliff.org) using the configuration in `cliff.toml`. See [CHANGELOG.md](CHANGELOG.md) for the full history.

### Running tests

```sh
# Rust tests
cargo test --workspace

# Nushell integration tests
nu scripts/tests/run_all.nu
```

## Minimum Supported Rust Version (MSRV)

The workspace uses the **Rust 2021 edition**. We recommend using the latest stable toolchain.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes and ensure `cargo clippy` and `cargo test` pass
4. Open a pull request

## License

GitKraft is licensed under the [MIT License](LICENSE).

---

<div align="center">

Made with 🦀 by [Sorin Irimies](https://github.com/sorinirimies)

</div>
