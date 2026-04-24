# GitKraft — Agent Rules

## ⚠️ MANDATORY: Tests After Every Feature

**After implementing any feature, bug fix, or refactor, you MUST write tests before considering the task complete.**

- For **`gitkraft-core`**: add `#[cfg(test)] mod tests` inline in the changed file (`types.rs` or `ops.rs`).
- For **`gitkraft-tui`**: add `#[cfg(test)] mod tests` inline in the changed file (`app.rs`, `events.rs`, `features/<name>/events.rs`, etc.).
- For **`gitkraft-gui`**: add `#[cfg(test)] mod tests` inline in `state.rs` or the relevant feature file (`update.rs`, `view.rs`). Test state transitions and update logic; pure rendering is harder to test so focus on the update layer.
- **Run `cargo test` (or `cargo test --manifest-path ...`) after adding tests** and confirm they pass before finishing.
- If you realise tests are missing after the fact, write them immediately — do not move on to the next task without them.
- **Never respond "done" or summarise a completed feature without mentioning which tests were added.**


## ⚠️ MANDATORY: Check Formatting After Every Change

**After every implementation, refactor, or fix — before considering the task complete — you MUST run:**

```
cargo fmt --all
```

Then verify there are no remaining issues with:

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Rules:
- **Never leave unformatted code.** If `cargo fmt --all` changes any file, the task is not done until those changes are committed or at least applied.
- **Clippy warnings are errors.** The CI treats `-D warnings` as hard failures. Fix every warning before finishing — do not suppress with `#[allow(...)]` unless there is a documented reason.
- **Run order**: `cargo fmt --all` → `cargo clippy` → `cargo test --lib` — in that order. A clean result on all three is the definition of "done".
- **Never respond "done" without confirming `cargo fmt --all` produced no diff and `cargo clippy` produced no warnings.**

## Project Overview

GitKraft is a Git IDE written entirely in Rust, shipping two front-ends (desktop GUI + terminal UI) from a single Cargo workspace. All Git logic lives in the shared `gitkraft-core` crate — zero Git operations belong in the UI layer.

```
gitkraft-gui ──┐
               ├──▶ gitkraft-core ──▶ git2, redb, chrono, serde
gitkraft-tui ──┘
```

## Workspace Layout

| Crate | Path | Purpose |
|---|---|---|
| `gitkraft-core` | `crates/gitkraft-core/` | Framework-free library: Git ops, persistence, themes |
| `gitkraft` (GUI) | `crates/gitkraft-gui/` | Desktop GUI built on Iced (Elm Architecture) |
| `gitkraft-tui` | `crates/gitkraft-tui/` | Terminal UI built on Ratatui + Crossterm |

## Architecture Rules

### Core Crate (`gitkraft-core`)

- **Feature modules** live under `src/features/<name>/` with a consistent structure:
  - `types.rs` — Plain data structs (`#[derive(Debug, Clone, Serialize, Deserialize)]`), no git2 dependency.
  - `ops.rs` — Functions that take `&git2::Repository` + params and return `anyhow::Result<T>`. All git2 calls live here.
  - `mod.rs` — Re-exports `types` and `ops`.
- **Error handling**: Use `anyhow::Result` with `.context("descriptive message")` on every fallible call. No custom error enums — `thiserror` is in `Cargo.toml` but `anyhow` is the standard.
- **No UI code** in core. If you need to add Git functionality, it goes in core first.
- Functions receive `&Repository` as a parameter — no global state.
- Convert git2 types to owned domain types immediately (e.g. `CommitInfo::from_git2_commit`).

### GUI Crate (`gitkraft-gui`) — Elm Architecture

The GUI follows **The Elm Architecture** (TEA): State → View → Message → Update.

- **State** (`state.rs`): `GitKraft` (app-wide) + `RepoTab` (per-tab repo workspace).
- **Message** (`message.rs`): Single flat `enum Message` with ~80 variants grouped by feature.
- **Update** (`update.rs`): `GitKraft::update(&mut self, msg) -> Task<Message>` delegates to `features::<name>::update::update()`.
- **View** (`view.rs`): `GitKraft::view(&self) -> Element<Message>` composes feature sub-views.

**Feature module pattern** under `src/features/<name>/`:
- `update.rs` — `pub fn update(state: &mut GitKraft, msg: Message) -> Task<Message>`
- `view.rs` — View functions returning `Element<Message>`
- `commands.rs` — Async `Task<Message>` factories using the `git_task!` macro to run blocking core calls off-thread.

**Key conventions**:
- Use **payload structs** to bundle async operation results into one message for atomic state updates.
- Core errors are `.map_err(|e| e.to_string())` into `Result<T, String>` at the boundary.
- After any Git mutation (push, rebase, merge, etc.), refresh by calling `load_repo_blocking` to rebuild the full `RepoPayload`.
- Never block the UI thread — all Git operations go through `commands.rs` tasks.

### TUI Crate (`gitkraft-tui`) — Immediate-Mode Loop

The TUI uses a standard ratatui loop running at ~30fps (33ms poll timeout).

**Feature module pattern** under `src/features/<name>/`:
- `events.rs` — Key event handling.
- `view.rs` — Rendering with ratatui widgets.
- `mod.rs` — State types.

**Key conventions**:
- All Git operations run on **background threads** via `std::sync::mpsc` channels (`bg_tx`/`bg_rx` on `App`).
- `poll_background()` drains the channel each tick and applies `BackgroundResult` variants to state.
- Event handling is hierarchical: `InputMode::Input` → Screen-level → `ActivePane`-specific.
- Per-repo state lives in `RepoTab`; multiple tabs via `Vec<RepoTab>` + `active_tab_index`.

## Coding Standards

- **Rust edition**: 2021, MSRV 1.80+.
- **Formatting**: `cargo fmt --all` — no custom rustfmt config.
- **Linting**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Warnings are errors in CI.
- **Dependencies**: Pin versions in `[workspace.dependencies]` in the root `Cargo.toml`. Each crate references them with `{ workspace = true }`.
- Prefer `just check-all` before committing (runs fmt + clippy + test + nu tests).

## Testing Patterns (see also the mandatory rule at the top of this file)

- **Unit tests for types**: Inline `#[cfg(test)] mod tests` in `types.rs`. Pure Rust, no repo needed. Use helper factory functions.
- **Integration tests for ops**: Inline in `ops.rs`. Use `tempfile::TempDir` + `Repository::init()` for throwaway repos. A `setup_repo_with_commit()` helper configures user.name/email, creates a file, stages, and commits.
- **GUI state/update tests**: Test `update()` handlers directly by constructing a `GitKraft` (or `RepoTab`) with known state, calling the handler, and asserting on the resulting state. No Iced runtime needed.
- **TUI event tests**: Construct an `App`, set relevant fields, call `handle_key()` or a feature function, and assert on state changes.
- **Test naming convention**: `<thing_under_test>_<scenario>_<expected_outcome>`, e.g. `select_diff_shift_click_extends_selection`.
- Run with `just test` (full workspace) or `just test-core` (core only).

## Dev Workflow (`justfile`)

| Command | Purpose |
|---|---|
| `just build` | Build workspace |
| `just run` | Run GUI |
| `just run-tui` | Run TUI |
| `just test` | Run all tests |
| `just check-all` | fmt + clippy + test + nu tests |
| `just fmt` | Format all code |
| `just clippy` | Lint with `-D warnings` |
| `just release 0.X.Y` | Bump, tag, push (runs all checks) |

## Common Pitfalls

1. **Don't skip tests.** Every feature implementation must be accompanied by tests. See the mandatory rule at the top of this file.
2. **Don't skip formatting.** Run `cargo fmt --all` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` after every change. See the mandatory rule at the top of this file.
3. **Don't put Git logic in the UI crates.** If you need a new Git operation, add it to `gitkraft-core/src/features/<name>/ops.rs` first, then call it from the GUI/TUI.
4. **Don't block the main thread.** GUI uses `Task`; TUI uses `mpsc` background threads.
5. **Don't add dependencies to individual crate `Cargo.toml` directly.** Add them to `[workspace.dependencies]` first.
6. **Don't use `thiserror` for new errors.** Stick with `anyhow::Result` + `.context()`.
7. **Match existing module structure.** Every feature follows the same file layout — don't invent new patterns.
8. **Keep the `Message` enum flat** in the GUI. Group variants with comments, not nested enums.
9. **GIF assets** are tracked with Git LFS (`*.gif` in `.gitattributes`). Use `vhs` to regenerate from `.tape` files.
10. **Theme code** is shared via `gitkraft-core`. Both front-ends use the same 27-theme palette — don't add themes to only one front-end.
11. **Don't leave dead code.** If a refactor makes existing functions or variants unreachable, remove them rather than annotating with `#[allow(dead_code)]`.