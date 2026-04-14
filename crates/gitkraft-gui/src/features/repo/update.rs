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

        Message::RefreshRepo => with_repo!(state, loading, "Refreshing…".into(), |path| {
            commands::refresh_repo(path)
        }),

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

            // Persist the updated session and refresh recent repos from disk.
            let open_tabs = state.open_tab_paths();
            let active = state.active_tab;
            Task::batch([
                commands::load_recent_repos_async(),
                commands::save_session_async(open_tabs, active),
            ])
        }

        Message::RepoRestoredAt(tab_index, result) => {
            handle_repo_loaded_at(state, tab_index, result)
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

        Message::MoreCommitsLoaded(result) => handle_more_commits_loaded(state, result),

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
            tab.commit_display = compute_commit_display(&tab.commits);
            tab.commit_scroll_offset = 0.0;
            tab.has_more_commits = true;
            tab.is_loading_more_commits = false;

            // Record the repo open AND persist the full session in one DB
            // write, on a background thread so redb I/O doesn't block the UI.
            let open_tabs = state.open_tab_paths();
            let active = state.active_tab;
            Task::batch([
                commands::record_repo_and_save_session_async(path, open_tabs, active),
                iced::widget::scrollable::scroll_to(
                    crate::features::commits::view::commit_log_scroll_id(active),
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                ),
            ])
        }
        Err(e) => {
            let tab = state.active_tab_mut();
            tab.error_message = Some(e);
            tab.status_message = None;
            Task::none()
        }
    }
}

/// Like `handle_repo_loaded` but writes into a specific tab index.
/// Used for parallel startup restore; does NOT record a repo open.
fn handle_repo_loaded_at(
    state: &mut GitKraft,
    tab_index: usize,
    result: Result<RepoPayload, String>,
) -> Task<Message> {
    if tab_index >= state.tabs.len() {
        return Task::none(); // tab closed before restore completed
    }
    state.tabs[tab_index].is_loading = false;
    match result {
        Ok(payload) => {
            let path = payload
                .info
                .workdir
                .clone()
                .unwrap_or_else(|| payload.info.path.clone());
            let tab = &mut state.tabs[tab_index];
            tab.current_branch = payload.info.head_branch.clone();
            tab.repo_path = Some(path);
            tab.repo_info = Some(payload.info);
            tab.branches = payload.branches;
            tab.commits = payload.commits;
            tab.graph_rows = payload.graph_rows;
            tab.unstaged_changes = payload.unstaged;
            tab.staged_changes = payload.staged;
            tab.stashes = payload.stashes;
            tab.remotes = payload.remotes;
            tab.selected_commit = None;
            tab.selected_diff = None;
            tab.commit_message.clear();
            tab.error_message = None;
            tab.status_message = Some("Repository loaded.".into());
            tab.commit_display = compute_commit_display(&tab.commits);
            tab.commit_scroll_offset = 0.0;
            tab.has_more_commits = true;
            tab.is_loading_more_commits = false;
            // Already in recent_repos — no need to re-record.
            Task::none()
        }
        Err(e) => {
            let tab = &mut state.tabs[tab_index];
            tab.error_message = Some(e);
            tab.status_message = None;
            Task::none()
        }
    }
}

/// Append a newly loaded commit page to the active tab's commit log.
fn handle_more_commits_loaded(
    state: &mut GitKraft,
    result: Result<crate::message::CommitPage, String>,
) -> Task<Message> {
    let tab = state.active_tab_mut();
    tab.is_loading_more_commits = false;
    match result {
        Ok(page) => {
            let prev_count = tab.commits.len();
            let new_total = page.commits.len();
            // If the server returned no new commits, we've hit the end.
            tab.has_more_commits = new_total > prev_count;
            if new_total > prev_count {
                // Only compute display strings for the newly added commits.
                let new_display = compute_commit_display(&page.commits[prev_count..]);
                tab.commit_display.extend(new_display);
            }
            tab.commits = page.commits;
            tab.graph_rows = page.graph_rows;
        }
        Err(e) => {
            tab.status_message = Some(format!("Failed to load more commits: {e}"));
        }
    }
    Task::none()
}

/// Pre-compute display strings for the commit log so the view function
/// never allocates strings on the hot rendering path.
fn compute_commit_display(commits: &[gitkraft_core::CommitInfo]) -> Vec<(String, String)> {
    commits
        .iter()
        .map(|c| {
            let summary = if c.summary.chars().count() > 60 {
                let mut s: String = c.summary.chars().take(59).collect();
                s.push('…');
                s
            } else {
                c.summary.clone()
            };
            let time = gitkraft_core::utils::relative_time(c.time);
            (summary, time)
        })
        .collect()
}
