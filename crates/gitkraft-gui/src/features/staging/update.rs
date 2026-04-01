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
            if let Some(repo_path) = state.repo_path.clone() {
                state.status_message = Some(format!("Staging '{path}'…"));
                commands::stage_file(repo_path, path)
            } else {
                Task::none()
            }
        }

        Message::UnstageFile(path) => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.status_message = Some(format!("Unstaging '{path}'…"));
                commands::unstage_file(repo_path, path)
            } else {
                Task::none()
            }
        }

        Message::StageAll => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.status_message = Some("Staging all files…".into());
                commands::stage_all(repo_path)
            } else {
                Task::none()
            }
        }

        Message::UnstageAll => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.status_message = Some("Unstaging all files…".into());
                commands::unstage_all(repo_path)
            } else {
                Task::none()
            }
        }

        Message::DiscardFile(path) => {
            if let Some(repo_path) = state.repo_path.clone() {
                state.status_message = Some(format!("Discarding changes in '{path}'…"));
                commands::discard_file(repo_path, path)
            } else {
                Task::none()
            }
        }

        Message::StagingUpdated(result) => {
            match result {
                Ok(payload) => {
                    state.unstaged_changes = payload.unstaged;
                    state.staged_changes = payload.staged;
                    state.status_message = Some("Staging area updated.".into());
                }
                Err(e) => {
                    state.error_message = Some(format!("Staging operation failed: {e}"));
                    state.status_message = None;
                }
            }
            Task::none()
        }

        _ => Task::none(),
    }
}
