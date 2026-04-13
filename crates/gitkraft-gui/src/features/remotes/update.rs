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
            // Resolve the remote name before entering the macro so it can be
            // used in both the status string and the command argument.
            let remote_name = state
                .active_tab()
                .remotes
                .first()
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "origin".to_string());

            with_repo!(
                state,
                loading,
                format!("Fetching from '{remote_name}'…"),
                |repo_path| commands::fetch_remote(repo_path, remote_name)
            )
        }

        Message::FetchCompleted(result) => {
            state.on_ok_refresh(result, "Fetch completed.", "Fetch failed")
        }

        _ => Task::none(),
    }
}
