<div align="center">

# ⚡ GitKraft

**A Git IDE written entirely in Rust — desktop GUI & terminal UI**

[![Crates.io](https://img.shields.io/crates/v/gitkraft.svg)](https://crates.io/crates/gitkraft)
[![docs.rs](https://docs.rs/gitkraft-core/badge.svg)](https://docs.rs/gitkraft-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

---

GitKraft ships two front-ends from a single Rust workspace:

| Binary | Use case |
|--------|----------|
| `gitkraft` | Desktop GUI — mouse, drag-to-resize panes, commit graph |
| `gitkraft-tui` | Terminal UI — keyboard-driven, great for SSH & headless machines |

Both share **`gitkraft-core`** — a framework-free library built on [`git2`](https://crates.io/crates/git2). Zero Git operations live in the UI layer.

## Preview

### Terminal UI

![TUI Demo](crates/gitkraft-tui/examples/vhs/generated/tui-demo.gif)
![TUI Welcome](crates/gitkraft-tui/examples/vhs/generated/tui-welcome.gif)

## Features

- **Branch management** — create, checkout, delete, rename (local & remote)
- **Commit log with graph** — canvas DAG in GUI, box-drawing in TUI
- **Diff viewer** — working-dir, staged, and per-commit diffs with coloured hunks
- **Staging area** — stage/unstage files or all at once, discard changes
- **Commit creation** — write a message and commit from the IDE
- **Stash management** — save, pop, drop with optional messages
- **Multi-tab** — open multiple repos in tabs (GUI & TUI), sessions persisted
- **Remote operations** — fetch, push, pull, remote branch checkout/delete
- **Context menus (GUI)** — right-click for checkout, rebase, merge, reset, revert, tag, copy SHA
- **UI zoom (GUI)** — Ctrl+/- to scale 50%–200%, persisted
- **Directory browser (TUI)** — press `o` to browse and open repos
- **27 colour themes** — Dracula, Nord, Catppuccin, Tokyo Night, Kanagawa, and more
- **Virtual scrolling** — smooth performance with large histories
- **Two-phase diff loading** — file list appears instantly, diffs load per-file
- **Draggable pane dividers (GUI)** — layout saved automatically
- **Persisted settings** — theme, layout, recent repos, open tabs

## Installation

```sh
# Desktop GUI
cargo install gitkraft

# Terminal UI
cargo install gitkraft-tui
```

Or download pre-built binaries from the [Releases page](https://github.com/sorinirimies/gitkraft/releases).

## Keyboard Shortcuts

### TUI

| Key | Action |
|-----|--------|
| **←/→** | Switch panes |
| **↑/↓** | Navigate within pane |
| **j/k** | Vim-style navigation |
| **h/l** | Switch files in diff |
| **Enter** | Load diff / view file |
| **s/u** | Stage / unstage file |
| **S/U** | Stage / unstage all |
| **c** | Commit |
| **d** | Discard (press twice) |
| **z/Z** | Stash save / pop |
| **o** | Browse & open repo |
| **N/W** | New tab / close tab |
| **]/[** | Next / previous tab |
| **r/f** | Refresh / fetch |
| **T/O** | Theme / options |
| **q** | Quit |

### GUI

| Key | Action |
|-----|--------|
| **Ctrl/Cmd + +** | Zoom in |
| **Ctrl/Cmd + -** | Zoom out |
| **Ctrl/Cmd + 0** | Reset zoom |

Right-click branches or commits for the full context menu.

## Building from Source

```sh
git clone https://github.com/sorinirimies/gitkraft.git
cd gitkraft
cargo build --release

# Run
cargo run --release -p gitkraft       # GUI
cargo run --release -p gitkraft-tui   # TUI
cargo run --release -p gitkraft-tui -- /path/to/repo
```

**Prerequisites:** Rust 1.80+, C compiler, cmake, pkg-config (Linux), libssl-dev.

## Themes

27 built-in themes, persisted per-user and shared between GUI and TUI:

> Default · Grape · Ocean · Sunset · Forest · Rose · Mono · Neon · Dracula · Nord · Solarized Dark/Light · Gruvbox Dark/Light · Catppuccin Latte/Frappé/Macchiato/Mocha · Tokyo Night/Storm/Light · Kanagawa Wave/Dragon/Lotus · Moonfly · Nightfly · Oxocarbon

## Development

```sh
just build            # build workspace
just run              # run GUI
just run-tui          # run TUI
just test             # run all tests
just check-all        # fmt + clippy + test + nu tests
just release 0.6.0    # bump, tag, push (runs all checks)
just push-all         # push to GitHub + Gitea
just install-tools    # install git-cliff + nushell
```

CI runs on both **GitHub Actions** and **Gitea Actions** with automated nightly dependency updates.

## Architecture

```
gitkraft-gui ──┐
               ├──▶ gitkraft-core ──▶ git2, redb, chrono, serde
gitkraft-tui ──┘
```

The GUI follows **The Elm Architecture** (State → View → Message → Update). Both front-ends are thin wrappers around `gitkraft-core`.

## Contributing

1. Fork → branch → make changes → ensure `just check-all` passes → PR

## License

[MIT](LICENSE)

---

<div align="center">

Made with 🦀 by [Sorin Irimies](https://github.com/sorinirimies)

</div>