//! Async command helpers for repository operations.
//!
//! Each function returns an `iced::Task<Message>` that performs blocking git
//! work on a background thread via the `git_task!` macro, then maps the
//! result into a [`Message`] variant the update loop can handle.
//!
//! This module also contains async wrappers for persistence operations
//! (`record_repo_opened`, `load_settings`, `save_theme`) so that settings
//! file I/O never blocks the UI thread.

use std::path::PathBuf;

use iced::Task;

use crate::macros::StringErr;
use crate::message::{Message, RepoPayload};

/// Open a folder-picker dialog and return the selected path.
pub(crate) fn pick_folder_open() -> Task<Message> {
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
pub(crate) fn pick_folder_init() -> Task<Message> {
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
pub(crate) fn init_repo(path: PathBuf) -> Task<Message> {
    git_task!(
        Message::RepoOpened,
        (|| {
            gitkraft_core::features::repo::init_repo(&path).str_err()?;
            load_repo_blocking(&path)
        })()
    )
}

/// Refresh all data for an already-open repository, preserving scroll depth.
pub(crate) fn refresh_repo(path: PathBuf, current_commit_count: usize) -> Task<Message> {
    git_task!(
        Message::RepoRefreshed,
        gitkraft_core::load_repo_snapshot_with_depth(&path, current_commit_count).str_err()
    )
}

/// Blocking helper shared by `load_repo` and `refresh_repo`.
///
/// Opens the repository and collects every piece of state the UI needs into a
/// single [`RepoPayload`].
pub(crate) fn load_repo_blocking(path: &std::path::Path) -> Result<RepoPayload, String> {
    gitkraft_core::load_repo_snapshot(path).str_err()
}

/// Get the working directory of a repository, returning a user-friendly error
/// for bare repositories.
pub(crate) fn workdir(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let repo = open_repo!(path);
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "bare repository has no working directory".to_string())
}

// ── Context-menu git commands ─────────────────────────────────────────────────

/// Push `branch` to `remote` then reload the repo.
pub(crate) fn push_branch_async(path: PathBuf, branch: String, remote: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| gitkraft_core::features::branches::push_branch(
        &wd, &branch, &remote
    ))
}

/// Force-push `branch` to `remote` (with --force-with-lease) then reload.
pub(crate) fn force_push_branch_async(
    path: PathBuf,
    branch: String,
    remote: String,
) -> Task<Message> {
    git_wd_then_reload!(path, |wd| {
        gitkraft_core::features::branches::force_push_branch(&wd, &branch, &remote)
    })
}

/// Pull the current branch from `remote` with `--rebase` then reload.
pub(crate) fn pull_rebase_async(path: PathBuf, remote: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| gitkraft_core::features::branches::pull_rebase(
        &wd, &remote
    ))
}

/// Rebase current HEAD onto `target` (branch name or OID) then reload.
pub(crate) fn rebase_onto_async(path: PathBuf, target: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| gitkraft_core::features::branches::rebase_onto(
        &wd, &target
    ))
}

/// Rename a local branch then reload.
pub(crate) fn rename_branch_async(
    path: PathBuf,
    old_name: String,
    new_name: String,
) -> Task<Message> {
    git_repo_then_reload!(path, |repo| {
        gitkraft_core::features::branches::rename_branch(&repo, &old_name, &new_name)
    })
}

/// Checkout a commit in detached HEAD mode then reload.
pub(crate) fn checkout_commit_async(path: PathBuf, oid: String) -> Task<Message> {
    git_repo_then_reload!(path, |repo| {
        gitkraft_core::features::repo::checkout_commit_detached(&repo, &oid)
    })
}

/// Revert a commit (`git revert --no-edit`) then reload.
pub(crate) fn revert_commit_async(path: PathBuf, oid: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| gitkraft_core::features::repo::revert_commit(
        &wd, &oid
    ))
}

/// Reset the current branch to `oid` using the given `mode`.
pub(crate) fn reset_to_commit_async(
    path: PathBuf,
    oid: String,
    mode: gitkraft_core::ResetMode,
) -> Task<Message> {
    git_wd_then_reload!(path, |wd| gitkraft_core::features::repo::reset_to_commit(
        &wd, &oid, mode
    ))
}

/// Merge `branch_name` into the current HEAD then reload.
pub(crate) fn merge_branch_async(path: PathBuf, branch_name: String) -> Task<Message> {
    git_repo_then_reload!(
        path,
        |repo| gitkraft_core::features::branches::merge_branch(&repo, &branch_name)
    )
}

/// Delete a remote branch using `git push --delete`.
pub(crate) fn delete_remote_branch_async(path: PathBuf, full_name: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| {
        gitkraft_core::features::branches::delete_remote_branch(&wd, &full_name)
    })
}

/// Checkout a remote branch by creating a local tracking branch.
pub(crate) fn checkout_remote_branch_async(path: PathBuf, full_name: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| {
        gitkraft_core::features::branches::checkout_remote_branch(&wd, &full_name)
    })
}

/// Create a lightweight tag `name` pointing at `oid` then reload.
pub(crate) fn create_tag_async(path: PathBuf, name: String, oid: String) -> Task<Message> {
    git_repo_then_reload!(path, |repo| gitkraft_core::features::branches::create_tag(
        &repo, &name, &oid
    ))
}

/// Create an annotated tag `name` with `message` pointing at `oid` then reload.
pub(crate) fn create_annotated_tag_async(
    path: PathBuf,
    name: String,
    message: String,
    oid: String,
) -> Task<Message> {
    git_repo_then_reload!(path, |repo| {
        gitkraft_core::features::branches::create_annotated_tag(&repo, &name, &message, &oid)
    })
}

/// Cherry-pick a commit onto the current branch then reload.
pub(crate) fn cherry_pick_async(path: PathBuf, oid: String) -> Task<Message> {
    git_wd_then_reload!(path, |wd| {
        gitkraft_core::features::commits::cherry_pick_commit(&wd, &oid)
    })
}

/// Create a local branch at a specific commit OID then reload.
pub(crate) fn create_branch_at_commit_async(
    path: PathBuf,
    name: String,
    oid: String,
) -> Task<Message> {
    git_repo_then_reload!(path, |repo| {
        gitkraft_core::features::branches::create_branch_at_commit(&repo, &name, &oid)
    })
}

/// Load the commit history for a single file on a background thread.
pub(crate) fn file_history_async(repo_path: PathBuf, file_path: String) -> Task<Message> {
    git_task!(
        Message::FileHistoryLoaded,
        (|| {
            let repo = open_repo!(&repo_path);
            let commits = gitkraft_core::file_history(&repo, &file_path, 500).str_err()?;
            Ok((file_path, commits))
        })()
    )
}

/// Load git-blame data for a single file on a background thread.
pub(crate) fn blame_file_async(repo_path: PathBuf, file_path: String) -> Task<Message> {
    git_task!(
        Message::FileBlameLoaded,
        (|| {
            let repo = open_repo!(&repo_path);
            let lines = gitkraft_core::blame_file(&repo, &file_path).str_err()?;
            Ok((file_path, lines))
        })()
    )
}

/// Delete a file from the working directory then refresh the staging area.
pub(crate) fn delete_file_async(repo_path: PathBuf, file_path: String) -> Task<Message> {
    git_wd_then_reload!(repo_path, |wd| gitkraft_core::delete_file(&wd, &file_path))
}

/// Execute any `CommitAction` against a specific commit then reload.
pub(crate) fn execute_commit_action_async(
    path: PathBuf,
    oid: String,
    action: gitkraft_core::CommitAction,
) -> Task<Message> {
    git_wd_then_reload!(path, |wd| action.execute(&wd, &oid))
}

/// Cherry-pick a list of commits (by OID) onto the current branch then reload.
pub(crate) fn cherry_pick_commits_async(path: PathBuf, oids: Vec<String>) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            for oid in &oids {
                gitkraft_core::features::commits::cherry_pick_commit(&wd, oid).str_err()?;
            }
            load_repo_blocking(&path)
        })()
    )
}

/// Revert a list of commits (by OID) in order then reload.
pub(crate) fn revert_commits_async(path: PathBuf, oids: Vec<String>) -> Task<Message> {
    git_task!(
        Message::GitOperationResult,
        (|| {
            let wd = workdir(&path)?;
            for oid in &oids {
                gitkraft_core::features::repo::revert_commit(&wd, oid).str_err()?;
            }
            load_repo_blocking(&path)
        })()
    )
}

// ── Async persistence helpers ─────────────────────────────────────────────────

/// Load the recent-repos list from persisted settings on a background thread.
pub(crate) fn load_recent_repos_async() -> Task<Message> {
    git_task!(
        Message::SettingsLoaded,
        (|| {
            let settings = gitkraft_core::features::persistence::ops::load_settings().str_err()?;
            Ok(settings.recent_repos)
        })()
    )
}

/// Save the theme preference on a background thread (fire-and-forget).
pub(crate) fn save_theme_async(theme_name: String) -> Task<Message> {
    git_task!(
        Message::ThemeSaved,
        gitkraft_core::features::persistence::ops::save_theme(&theme_name).str_err()
    )
}

/// Save layout preferences on a background thread (fire-and-forget).
pub(crate) fn save_layout_async(layout: gitkraft_core::LayoutSettings) -> Task<Message> {
    git_task!(
        Message::LayoutSaved,
        gitkraft_core::features::persistence::ops::save_layout(&layout).str_err()
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
pub(crate) fn record_repo_and_save_session_async(
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
        .str_err()
    )
}

/// Load the next page of commit history.
///
/// Fetches all commits up to `skip + count` from HEAD, rebuilds the full graph,
/// and returns a `CommitPage` for the update handler to swap in.
pub(crate) fn load_more_commits(path: PathBuf, skip: usize, count: usize) -> Task<Message> {
    let total = skip + count;
    git_task!(
        Message::MoreCommitsLoaded,
        (|| {
            let repo = open_repo!(&path);
            let commits = gitkraft_core::features::commits::list_commits(&repo, total).str_err()?;
            let graph_rows = gitkraft_core::features::graph::build_graph(&commits);
            Ok(crate::message::CommitPage {
                commits,
                graph_rows,
            })
        })()
    )
}

/// Persist the selected editor name (fire-and-forget).
pub(crate) fn save_editor_async(editor_name: String) -> Task<Message> {
    git_task!(
        Message::EditorSaved,
        gitkraft_core::features::persistence::save_editor(&editor_name).str_err()
    )
}

/// Save the session (open tab paths + active tab index) asynchronously.
pub(crate) fn save_session_async(
    open_tabs: Vec<PathBuf>,
    active_tab_index: usize,
) -> Task<Message> {
    git_task!(
        Message::SessionSaved,
        gitkraft_core::features::persistence::ops::save_session(&open_tabs, active_tab_index)
            .str_err()
    )
}
