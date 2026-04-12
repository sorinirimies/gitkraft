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
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Staging '{path}'…"));
                commands::stage_file(repo_path, path)
            } else {
                Task::none()
            }
        }

        Message::UnstageFile(path) => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Unstaging '{path}'…"));
                commands::unstage_file(repo_path, path)
            } else {
                Task::none()
            }
        }

        Message::StageAll => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some("Staging all files…".into());
                commands::stage_all(repo_path)
            } else {
                Task::none()
            }
        }

        Message::UnstageAll => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some("Unstaging all files…".into());
                commands::unstage_all(repo_path)
            } else {
                Task::none()
            }
        }

        Message::DiscardFile(path) => {
            let repo_path = state.active_tab().repo_path.clone();
            if let Some(repo_path) = repo_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Discarding changes in '{path}'…"));
                commands::discard_file(repo_path, path)
            } else {
                Task::none()
            }
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
