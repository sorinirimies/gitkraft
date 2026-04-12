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
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(path) = repo_path {
                // Fetch from the first configured remote (usually "origin").
                let remote_name = state
                    .active_tab()
                    .remotes
                    .first()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "origin".to_string());

                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Fetching from '{remote_name}'…"));
                commands::fetch_remote(path, remote_name)
            } else {
                Task::none()
            }
        }

        Message::FetchCompleted(result) => {
            state.active_tab_mut().is_loading = false;
            match result {
                Ok(()) => {
                    state.active_tab_mut().status_message = Some("Fetch completed.".into());
                    // Trigger a full refresh so branches / commits reflect any
                    // new remote state.
                    let repo_path = state.active_tab().repo_path.clone();
                    if let Some(path) = repo_path {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    let tab = state.active_tab_mut();
                    tab.error_message = Some(format!("Fetch failed: {e}"));
                    tab.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
