//! Update logic for diff-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

/// Handle diff-related messages, returning a [`Task`] for any follow-up
/// async work.
///
/// `SelectCommit` and `CommitDiffLoaded` are handled by the commits feature
/// module — this handler only deals with `SelectDiff`.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectDiff(diff_info) => {
            state.selected_diff = Some(diff_info);
            Task::none()
        }

        _ => Task::none(),
    }
}
