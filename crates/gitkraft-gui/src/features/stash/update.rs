//! Update logic for stash-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all stash-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::StashMessageChanged(msg) => {
            state.stash_message = msg;
            Task::none()
        }

        Message::StashSave => {
            if let Some(repo_path) = state.repo_path.clone() {
                let msg = if state.stash_message.trim().is_empty() {
                    None
                } else {
                    Some(state.stash_message.trim().to_string())
                };
                state.is_loading = true;
                state.status_message = Some("Saving stash…".into());
                state.stash_message.clear();
                commands::stash_save(repo_path, msg)
            } else {
                Task::none()
            }
        }

        Message::StashPop(index) => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some(format!("Popping stash@{{{index}}}…"));
                commands::stash_pop(repo_path, index)
            } else {
                Task::none()
            }
        }

        Message::StashDrop(index) => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some(format!("Dropping stash@{{{index}}}…"));
                commands::stash_drop(repo_path, index)
            } else {
                Task::none()
            }
        }

        Message::StashUpdated(result) => {
            state.is_loading = false;
            match result {
                Ok(stashes) => {
                    state.stashes = stashes;
                    state.status_message = Some("Stash updated.".into());
                    // Also refresh the staging area since stash save/pop affects
                    // the working directory and index.
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Stash operation failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
