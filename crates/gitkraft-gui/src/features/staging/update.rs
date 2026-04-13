//! Update logic for staging-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all staging-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::StageFile(path) => {
            with_repo!(state, format!("Staging '{path}'…"), |repo_path| {
                commands::stage_file(repo_path, path)
            })
        }

        Message::UnstageFile(path) => {
            with_repo!(state, format!("Unstaging '{path}'…"), |repo_path| {
                commands::unstage_file(repo_path, path)
            })
        }

        Message::StageAll => {
            with_repo!(state, "Staging all files…".into(), |repo_path| {
                commands::stage_all(repo_path)
            })
        }

        Message::UnstageAll => {
            with_repo!(state, "Unstaging all files…".into(), |repo_path| {
                commands::unstage_all(repo_path)
            })
        }

        Message::DiscardFile(path) => {
            let tab = state.active_tab_mut();
            tab.pending_discard = Some(path);
            tab.status_message =
                Some("Click discard again to confirm, or press elsewhere to cancel.".into());
            Task::none()
        }

        Message::ConfirmDiscard(path) => {
            with_repo!(
                state,
                format!("Discarding changes in '{path}'…"),
                |repo_path| {
                    state.active_tab_mut().pending_discard = None;
                    commands::discard_file(repo_path, path)
                }
            )
        }

        Message::CancelDiscard => {
            let tab = state.active_tab_mut();
            tab.pending_discard = None;
            tab.status_message = None;
            Task::none()
        }

        Message::StagingUpdated(result) => {
            let tab = state.active_tab_mut();
            match result {
                Ok(payload) => {
                    tab.unstaged_changes = payload.unstaged;
                    tab.staged_changes = payload.staged;
                    tab.status_message = Some("Staging area updated.".into());
                }
                Err(e) => {
                    tab.error_message = Some(format!("Staging operation failed: {e}"));
                    tab.status_message = None;
                }
            }
            Task::none()
        }

        _ => Task::none(),
    }
}
