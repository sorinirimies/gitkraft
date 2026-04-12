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
            let tab = state.active_tab_mut();
            tab.is_loading = true;
            tab.status_message = Some("Opening folder picker…".into());
            commands::pick_folder_open()
        }

        Message::InitRepo => {
            let tab = state.active_tab_mut();
            tab.is_loading = true;
            tab.status_message = Some("Opening folder picker for init…".into());
            commands::pick_folder_init()
        }

        Message::RepoSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Opening {}…", path.display()));
                commands::load_repo(path)
            } else {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.status_message = None;
                Task::none()
            }
        }

        Message::RepoInitSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Initializing {}…", path.display()));
                commands::init_repo(path)
            } else {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.status_message = None;
                Task::none()
            }
        }

        Message::RepoOpened(result) => handle_repo_loaded(state, result),

        Message::RefreshRepo => {
            let path = state.active_tab().repo_path.clone();
            if let Some(path) = path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some("Refreshing…".into());
                commands::refresh_repo(path)
            } else {
                Task::none()
            }
        }

        Message::RepoRefreshed(result) => handle_repo_loaded(state, result),

        Message::OpenRecentRepo(path) => {
            let tab = state.active_tab_mut();
            tab.is_loading = true;
            tab.status_message = Some(format!("Opening {}…", path.display()));
            commands::load_repo(path)
        }

        Message::CloseRepo => {
            // Replace the active tab with a fresh empty one, returning the
            // user to the welcome screen where the recent-repos list is visible.
            state.tabs[state.active_tab] = crate::state::RepoTab::new_empty();

            // Reset drag state (these fields remain on GitKraft).
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
    state.active_tab_mut().is_loading = false;

    match result {
        Ok(payload) => {
            // Derive the workdir path (preferred) or fall back to the .git path.
            let path = payload
                .info
                .workdir
                .clone()
                .unwrap_or_else(|| payload.info.path.clone());

            let tab = state.active_tab_mut();
            tab.current_branch = payload.info.head_branch.clone();
            tab.repo_path = Some(path.clone());
            tab.repo_info = Some(payload.info);
            tab.branches = payload.branches;
            tab.commits = payload.commits;
            tab.graph_rows = payload.graph_rows;
            tab.unstaged_changes = payload.unstaged;
            tab.staged_changes = payload.staged;
            tab.stashes = payload.stashes;
            tab.remotes = payload.remotes;

            // Reset transient UI state.
            tab.selected_commit = None;
            tab.selected_diff = None;
            tab.commit_message.clear();
            tab.error_message = None;
            tab.status_message = Some("Repository loaded.".into());

            // Record the repo open and refresh the recent-repos list
            // on a background thread so redb I/O doesn't block the UI.
            commands::record_repo_opened_async(path)
        }
        Err(e) => {
            let tab = state.active_tab_mut();
            tab.error_message = Some(e);
            tab.status_message = None;
            Task::none()
        }
    }
}
