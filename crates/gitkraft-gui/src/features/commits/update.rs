//! Update logic for commit-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all commit-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectCommit(index) => {
            state.selected_commit = Some(index);
            state.show_commit_detail = true;

            // Load the diff for the selected commit.
            if let (Some(path), Some(commit)) = (state.repo_path.clone(), state.commits.get(index))
            {
                let oid = commit.oid.clone();
                state.status_message = Some(format!("Loading diff for {}…", &commit.short_oid));
                commands::load_commit_diff(path, oid)
            } else {
                Task::none()
            }
        }

        Message::CommitDiffLoaded(result) => {
            match result {
                Ok(diffs) => {
                    // If there's at least one diff, auto-select the first one
                    // so the diff viewer shows something right away.
                    state.selected_diff = diffs.first().cloned();
                    state.status_message = Some(format!("Loaded {} file(s) in diff.", diffs.len()));
                }
                Err(e) => {
                    state.error_message = Some(format!("Failed to load commit diff: {e}"));
                    state.status_message = None;
                }
            }
            Task::none()
        }

        Message::CommitMessageChanged(msg) => {
            state.commit_message = msg;
            Task::none()
        }

        Message::CreateCommit => {
            let msg = state.commit_message.trim().to_string();
            if msg.is_empty() || state.staged_changes.is_empty() {
                return Task::none();
            }
            if let Some(path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some("Creating commit…".into());
                commands::create_commit(path, msg)
            } else {
                Task::none()
            }
        }

        Message::CommitCreated(result) => {
            state.is_loading = false;
            match result {
                Ok(()) => {
                    state.commit_message.clear();
                    state.status_message = Some("Commit created.".into());
                    // Trigger a full refresh to update the commit log, staging
                    // area, branches, etc.
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Commit failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
