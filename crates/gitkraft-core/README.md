# gitkraft-core

Shared, framework-free core logic for **GitKraft** — a Git IDE built in Rust.

This crate provides domain models, git operations, repository management, and
utility helpers that are consumed by both the GUI (`gitkraft-gui`) and TUI
(`gitkraft-tui`) front-ends. Persisted settings (themes, session, dashboard) live in
`gitkraft-settings`; forge access (GitHub, GitLab, Gitea) lives in `gitkraft-platforms` and `gitkraft-hub`.

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
* Git operations go through the `git2` crate. The one exception is network transport
  (`remotes::transport`: clone, fetch, push), which runs the system `git` so your SSH
  config, agent and credential helpers apply, and turns failures into readable messages.

## License

MIT