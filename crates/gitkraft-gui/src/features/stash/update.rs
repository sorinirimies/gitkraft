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
            // Derive the optional stash message before the macro borrows state.
            let msg = {
                let tab = state.active_tab();
                if tab.stash_message.trim().is_empty() {
                    None
                } else {
                    Some(tab.stash_message.trim().to_string())
                }
            };
            with_repo!(state, loading, "Saving stash…".into(), |repo_path| {
                state.active_tab_mut().stash_message.clear();
                commands::stash_save(repo_path, msg)
            })
        }

        Message::StashPop(index) => with_repo!(
            state,
            loading,
            format!("Popping stash@{{{index}}}…"),
            |repo_path| commands::stash_pop(repo_path, index)
        ),

        Message::StashDrop(index) => with_repo!(
            state,
            loading,
            format!("Dropping stash@{{{index}}}…"),
            |repo_path| commands::stash_drop(repo_path, index)
        ),

        Message::StashUpdated(result) => {
            state.active_tab_mut().is_loading = false;
            match result {
                Ok(stashes) => {
                    {
                        let tab = state.active_tab_mut();
                        tab.stashes = stashes;
                        tab.status_message = Some("Stash updated.".into());
                    }
                    state.refresh_active_tab()
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
