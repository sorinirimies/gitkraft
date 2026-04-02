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

        _ => Task::none(),
    }
}

/// Shared handler for both `RepoOpened` and `RepoRefreshed` — they carry the
/// same payload and should update the same state fields.
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

            // Record the repo open in persisted settings (best-effort).
            let _ = gitkraft_core::features::persistence::ops::record_repo_opened(&path);

            // Refresh the in-memory recent repos list so the welcome screen
            // stays up-to-date if the user closes and re-opens a repo.
            if let Ok(settings) = gitkraft_core::features::persistence::ops::load_settings() {
                state.recent_repos = settings.recent_repos;
            }

            state.current_branch = payload.info.head_branch.clone();
            state.repo_path = Some(path);
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
        }
        Err(e) => {
            state.error_message = Some(e);
            state.status_message = None;
        }
    }

    Task::none()
}
