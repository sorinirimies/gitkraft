//! Async command helpers for repository operations.
//!
//! Each function returns an `iced::Task<Message>` that performs blocking git
//! work on a background thread via the [`git_task!`] macro, then maps the
//! result into a [`Message`] variant the update loop can handle.
//!
//! This module also contains async wrappers for persistence operations
//! (`record_repo_opened`, `load_settings`, `save_theme`) so that redb
//! database I/O never blocks the UI thread.

use std::path::PathBuf;

use iced::Task;

use crate::message::{Message, RepoPayload};

/// Open a folder-picker dialog and return the selected path.
pub fn pick_folder_open() -> Task<Message> {
    Task::perform(
        async {
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Open Git Repository")
                .pick_folder()
                .await;
            handle.map(|h| h.path().to_path_buf())
        },
        Message::RepoSelected,
    )
}

/// Open a folder-picker dialog for initialising a new repository.
pub fn pick_folder_init() -> Task<Message> {
    Task::perform(
        async {
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Choose folder for new repository")
                .pick_folder()
                .await;
            handle.map(|h| h.path().to_path_buf())
        },
        Message::RepoInitSelected,
    )
}

/// Load (open) a repository at `path` and gather all initial state.
pub fn load_repo(path: PathBuf) -> Task<Message> {
    git_task!(Message::RepoOpened, load_repo_blocking(&path))
}

/// Initialise a new repository at `path` and then load it.
pub fn init_repo(path: PathBuf) -> Task<Message> {
    git_task!(
        Message::RepoOpened,
        (|| {
            gitkraft_core::features::repo::init_repo(&path).map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Refresh all data for an already-open repository.
pub fn refresh_repo(path: PathBuf) -> Task<Message> {
    git_task!(Message::RepoRefreshed, load_repo_blocking(&path))
}

/// Blocking helper shared by `load_repo` and `refresh_repo`.
///
/// Opens the repository and collects every piece of state the UI needs into a
/// single [`RepoPayload`].
fn load_repo_blocking(path: &std::path::Path) -> Result<RepoPayload, String> {
    // Open the repository once and reuse the handle for every operation.
    // `list_stashes` needs `&mut`, so we declare the binding as `mut`.
    let mut repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;

    let info = gitkraft_core::features::repo::get_repo_info(&repo).map_err(|e| e.to_string())?;
    let branches =
        gitkraft_core::features::branches::list_branches(&repo).map_err(|e| e.to_string())?;
    let commits =
        gitkraft_core::features::commits::list_commits(&repo, 500).map_err(|e| e.to_string())?;
    let graph_rows = gitkraft_core::features::graph::build_graph(&commits);
    let unstaged =
        gitkraft_core::features::diff::get_working_dir_diff(&repo).map_err(|e| e.to_string())?;
    let staged =
        gitkraft_core::features::diff::get_staged_diff(&repo).map_err(|e| e.to_string())?;
    let remotes =
        gitkraft_core::features::remotes::list_remotes(&repo).map_err(|e| e.to_string())?;
    let stashes =
        gitkraft_core::features::stash::list_stashes(&mut repo).map_err(|e| e.to_string())?;

    Ok(RepoPayload {
        info,
        branches,
        commits,
        graph_rows,
        unstaged,
        staged,
        stashes,
        remotes,
    })
}

/// Get the working directory of a repository, returning a user-friendly error
/// for bare repositories.
fn workdir(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "bare repository has no working directory".to_string())
}

// ── Context-menu git commands ─────────────────────────────────────────────────

/// Push `branch` to `remote` then reload the repo.
pub fn push_branch_async(path: PathBuf, branch: String, remote: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::branches::push_branch(&wd, &branch, &remote)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Pull the current branch from `remote` with `--rebase` then reload.
pub fn pull_rebase_async(path: PathBuf, remote: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::branches::pull_rebase(&wd, &remote)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Rebase current HEAD onto `target` (branch name or OID) then reload.
pub fn rebase_onto_async(path: PathBuf, target: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::branches::rebase_onto(&wd, &target)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Rename a local branch then reload.
pub fn rename_branch_async(path: PathBuf, old_name: String, new_name: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::branches::rename_branch(&repo, &old_name, &new_name)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Checkout a commit in detached HEAD mode then reload.
pub fn checkout_commit_async(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::repo::checkout_commit_detached(&repo, &oid)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Revert a commit (`git revert --no-edit`) then reload.
pub fn revert_commit_async(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::repo::revert_commit(&wd, &oid).map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Reset the current branch to `oid` using the given `mode`
/// (`"soft"`, `"mixed"`, or `"hard"`).
pub fn reset_to_commit_async(path: PathBuf, oid: String, mode: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::repo::reset_to_commit(&wd, &oid, &mode)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Merge `branch_name` into the current HEAD then reload.
pub fn merge_branch_async(path: PathBuf, branch_name: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::branches::merge_branch(&repo, &branch_name)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Delete a remote branch using `git push --delete`.
pub fn delete_remote_branch_async(path: PathBuf, full_name: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::branches::delete_remote_branch(&wd, &full_name)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Checkout a remote branch by creating a local tracking branch.
pub fn checkout_remote_branch_async(path: PathBuf, full_name: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            gitkraft_core::features::branches::checkout_remote_branch(&wd, &full_name)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Create a lightweight tag `name` pointing at `oid` then reload.
pub fn create_tag_async(path: PathBuf, name: String, oid: String) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::branches::create_tag(&repo, &name, &oid)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

/// Create an annotated tag `name` with `message` pointing at `oid` then reload.
pub fn create_annotated_tag_async(
    path: PathBuf,
    name: String,
    message: String,
    oid: String,
) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::branches::create_annotated_tag(&repo, &name, &message, &oid)
                .map_err(|e| e.to_string())?;
            load_repo_blocking(&path)
        })()
    )
}

// ── Async persistence helpers ─────────────────────────────────────────────────

/// Record that a repo was opened and return the refreshed recent-repos list.
///
/// Runs `record_repo_opened` + `load_settings` on a background thread so that
/// the redb database I/O never blocks the Iced event loop.
pub fn record_repo_opened_async(path: std::path::PathBuf) -> Task<Message> {
    git_task!(
        Message::RepoRecorded,
        (|| {
            gitkraft_core::features::persistence::ops::record_repo_opened(&path)
                .map_err(|e| e.to_string())?;
            let settings = gitkraft_core::features::persistence::ops::load_settings()
                .map_err(|e| e.to_string())?;
            Ok(settings.recent_repos)
        })()
    )
}

/// Load the recent-repos list from persisted settings on a background thread.
pub fn load_recent_repos_async() -> Task<Message> {
    git_task!(
        Message::SettingsLoaded,
        (|| {
            let settings = gitkraft_core::features::persistence::ops::load_settings()
                .map_err(|e| e.to_string())?;
            Ok(settings.recent_repos)
        })()
    )
}

/// Save the theme preference on a background thread (fire-and-forget).
pub fn save_theme_async(theme_name: String) -> Task<Message> {
    git_task!(
        Message::ThemeSaved,
        gitkraft_core::features::persistence::ops::save_theme(&theme_name)
            .map_err(|e| e.to_string())
    )
}

/// Save layout preferences on a background thread (fire-and-forget).
pub fn save_layout_async(layout: gitkraft_core::LayoutSettings) -> Task<Message> {
    git_task!(
        Message::LayoutSaved,
        gitkraft_core::features::persistence::ops::save_layout(&layout).map_err(|e| e.to_string())
    )
}

/// Load layout preferences from persisted settings on a background thread.
pub fn load_layout_async() -> Task<Message> {
    git_task!(
        Message::LayoutLoaded,
        gitkraft_core::features::persistence::ops::get_saved_layout().map_err(|e| e.to_string())
    )
}

/// Load a repository at `path` directly into tab `tab_index`.
/// Used on startup to restore all saved tabs in parallel.
pub fn load_repo_at(tab_index: usize, path: PathBuf) -> Task<Message> {
    git_task!(
        move |result| Message::RepoRestoredAt(tab_index, result),
        load_repo_blocking(&path)
    )
}

/// Record a repo open AND save the full session in one DB write.
pub fn record_repo_and_save_session_async(
    path: PathBuf,
    open_tabs: Vec<PathBuf>,
    active_tab_index: usize,
) -> Task<Message> {
    git_task!(
        Message::RepoRecorded,
        gitkraft_core::features::persistence::ops::record_repo_and_save_session(
            &path,
            &open_tabs,
            active_tab_index,
        )
        .map_err(|e| e.to_string())
    )
}

/// Load the next page of commit history.
///
/// Fetches all commits up to `skip + count` from HEAD, rebuilds the full graph,
/// and returns a [`CommitPage`] for the update handler to swap in.
pub fn load_more_commits(path: PathBuf, skip: usize, count: usize) -> Task<Message> {
    let total = skip + count;
    git_task!(
        Message::MoreCommitsLoaded,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            let commits = gitkraft_core::features::commits::list_commits(&repo, total)
                .map_err(|e| e.to_string())?;
            let graph_rows = gitkraft_core::features::graph::build_graph(&commits);
            Ok(crate::message::CommitPage {
                commits,
                graph_rows,
            })
        })()
    )
}

/// Save the session (open tab paths + active tab index) asynchronously.
pub fn save_session_async(open_tabs: Vec<PathBuf>, active_tab_index: usize) -> Task<Message> {
    git_task!(
        Message::SessionSaved,
        gitkraft_core::features::persistence::ops::save_session(&open_tabs, active_tab_index)
            .map_err(|e| e.to_string())
    )
}
