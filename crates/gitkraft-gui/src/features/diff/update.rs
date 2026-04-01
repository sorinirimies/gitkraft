//! Update logic for diff-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all diff-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectCommit(index) => {
            state.selected_commit = Some(index);
            state.show_commit_detail = true;

            if let Some(commit) = state.commits.get(index) {
                let oid = commit.oid.clone();
                if let Some(path) = state.repo_path.clone() {
                    state.status_message = Some("Loading commit diff…".into());
                    commands::load_commit_diff(path, oid)
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        }

        Message::CommitDiffLoaded(result) => {
            match result {
                Ok(diffs) => {
                    // Auto-select the first diff if there is one.
                    state.selected_diff = diffs.first().cloned();
                    state.status_message = Some(format!("{} file(s) changed", diffs.len()));
                }
                Err(e) => {
                    state.error_message = Some(format!("Failed to load commit diff: {e}"));
                    state.status_message = None;
                }
            }
            Task::none()
        }

        Message::SelectDiff(diff_info) => {
            state.selected_diff = Some(diff_info);
            Task::none()
        }

        _ => Task::none(),
    }
}
