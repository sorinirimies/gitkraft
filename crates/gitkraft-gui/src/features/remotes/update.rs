//! Update logic for remote-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all remote-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::Fetch => {
            if let Some(path) = state.repo_path.clone() {
                // Fetch from the first configured remote (usually "origin").
                let remote_name = state
                    .remotes
                    .first()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "origin".to_string());

                state.is_loading = true;
                state.status_message = Some(format!("Fetching from '{remote_name}'…"));
                commands::fetch_remote(path, remote_name)
            } else {
                Task::none()
            }
        }

        Message::FetchCompleted(result) => {
            state.is_loading = false;
            match result {
                Ok(()) => {
                    state.status_message = Some("Fetch completed.".into());
                    // Trigger a full refresh so branches / commits reflect any
                    // new remote state.
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Fetch failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
