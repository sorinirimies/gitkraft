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
            state.active_tab_mut().stash_message = msg;
            Task::none()
        }

        Message::StashSave => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                let msg = if tab.stash_message.trim().is_empty() {
                    None
                } else {
                    Some(tab.stash_message.trim().to_string())
                };
                tab.is_loading = true;
                tab.status_message = Some("Saving stash…".into());
                tab.stash_message.clear();
                commands::stash_save(repo_path, msg)
            } else {
                Task::none()
            }
        }

        Message::StashPop(index) => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Popping stash@{{{index}}}…"));
                commands::stash_pop(repo_path, index)
            } else {
                Task::none()
            }
        }

        Message::StashDrop(index) => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Dropping stash@{{{index}}}…"));
                commands::stash_drop(repo_path, index)
            } else {
                Task::none()
            }
        }

        Message::StashUpdated(result) => {
            state.active_tab_mut().is_loading = false;
            match result {
                Ok(stashes) => {
                    {
                        let tab = state.active_tab_mut();
                        tab.stashes = stashes;
                        tab.status_message = Some("Stash updated.".into());
                    }
                    // Also refresh the staging area since stash save/pop affects
                    // the working directory and index.
                    let path = state.active_tab().repo_path.clone();
                    if let Some(path) = path {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    let tab = state.active_tab_mut();
                    tab.error_message = Some(format!("Stash operation failed: {e}"));
                    tab.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
