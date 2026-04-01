# gitkraft-core

Shared, framework-free core logic for **GitKraft** — a Git IDE built in Rust.

This crate provides domain models, git operations, repository management, and
utility helpers that are consumed by both the GUI (`gitkraft-gui`) and TUI
(`gitkraft-tui`) front-ends.

## Feature highlights

| Module       | Purpose                                                  |
|-------------|----------------------------------------------------------|
| `domain`    | Value types — `CommitInfo`, `BranchInfo`, `DiffInfo`, …  |
| `commands`  | Pure `git2` operations — open, commit, branch, diff, …   |
| `utils`     | Logging bootstrap, OID formatting, relative-time helper  |

## Design principles

* **No GUI / TUI dependencies** — this crate is front-end agnostic.
* Every public function returns `anyhow::Result<T>`.
* Domain types derive `Debug, Clone, Serialize, Deserialize` where sensible.
* All git operations go through the `git2` crate — no shelling out.

## License

MIT