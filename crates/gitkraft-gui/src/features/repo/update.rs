//! Update logic for repository-level messages.

use iced::Task;

use crate::message::{Message, RepoPayload};
use crate::state::GitKraft;

use super::commands;

/// Handle all repository-related messages, returning a [`Task`] for any
/// follow-up async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::OpenRepo => {
            state.is_loading = true;
            state.status_message = Some("Opening folder picker…".into());
            commands::pick_folder_open()
        }

        Message::InitRepo => {
            state.is_loading = true;
            state.status_message = Some("Opening folder picker for init…".into());
            commands::pick_folder_init()
        }

        Message::RepoSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                state.status_message = Some(format!("Opening {}…", path.display()));
                commands::load_repo(path)
            } else {
                state.is_loading = false;
                state.status_message = None;
                Task::none()
            }
        }

        Message::RepoInitSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                state.status_message = Some(format!("Initializing {}…", path.display()));
                commands::init_repo(path)
            } else {
                state.is_loading = false;
                state.status_message = None;
                Task::none()
            }
        }

        Message::RepoOpened(result) => handle_repo_loaded(state, result),

        Message::RefreshRepo => {
            if let Some(path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some("Refreshing…".into());
                commands::refresh_repo(path)
            } else {
                Task::none()
            }
        }

        Message::RepoRefreshed(result) => handle_repo_loaded(state, result),

        Message::OpenRecentRepo(path) => {
            state.is_loading = true;
            state.status_message = Some(format!("Opening {}…", path.display()));
            commands::load_repo(path)
        }

        Message::CloseRepo => {
            // Clear all repository state, returning the user to the welcome
            // screen where the recent-repos list is visible.
            state.repo_path = None;
            state.repo_info = None;
            state.branches.clear();
            state.current_branch = None;
            state.commits.clear();
            state.selected_commit = None;
            state.graph_rows.clear();
            state.unstaged_changes.clear();
            state.staged_changes.clear();
            state.selected_diff = None;
            state.commit_diffs.clear();
            state.commit_message.clear();
            state.stashes.clear();
            state.remotes.clear();
            state.show_commit_detail = false;
            state.show_branch_create = false;
            state.new_branch_name.clear();
            state.stash_message.clear();
            state.error_message = None;
            state.status_message = None;
            state.is_loading = false;
            state.dragging = None;
            state.dragging_h = None;
            state.drag_initialized = false;
            state.drag_initialized_h = false;

            // Refresh recent repos from disk asynchronously.
            commands::load_recent_repos_async()
        }

        // ── Async persistence results ─────────────────────────────────────
        Message::RepoRecorded(result) => {
            if let Ok(recent) = result {
                state.recent_repos = recent;
            }
            // Errors are silently ignored — persistence is best-effort.
            Task::none()
        }

        Message::SettingsLoaded(result) => {
            if let Ok(recent) = result {
                state.recent_repos = recent;
            }
            Task::none()
        }

        _ => Task::none(),
    }
}

/// Shared handler for both `RepoOpened` and `RepoRefreshed` — they carry the
/// same payload and should update the same state fields.
///
/// Persistence (recording the repo open and refreshing the recent-repos list)
/// is dispatched as an async [`Task`] so the redb I/O never blocks the UI.
fn handle_repo_loaded(state: &mut GitKraft, result: Result<RepoPayload, String>) -> Task<Message> {
    state.is_loading = false;

    match result {
        Ok(payload) => {
            // Derive the workdir path (preferred) or fall back to the .git path.
            let path = payload
                .info
                .workdir
                .clone()
                .unwrap_or_else(|| payload.info.path.clone());

            state.current_branch = payload.info.head_branch.clone();
            state.repo_path = Some(path.clone());
            state.repo_info = Some(payload.info);
            state.branches = payload.branches;
            state.commits = payload.commits;
            state.graph_rows = payload.graph_rows;
            state.unstaged_changes = payload.unstaged;
            state.staged_changes = payload.staged;
            state.stashes = payload.stashes;
            state.remotes = payload.remotes;

            // Reset transient UI state.
            state.selected_commit = None;
            state.selected_diff = None;
            state.commit_message.clear();
            state.error_message = None;
            state.status_message = Some("Repository loaded.".into());

            // Record the repo open and refresh the recent-repos list
            // on a background thread so redb I/O doesn't block the UI.
            commands::record_repo_opened_async(path)
        }
        Err(e) => {
            state.error_message = Some(e);
            state.status_message = None;
            Task::none()
        }
    }
}
