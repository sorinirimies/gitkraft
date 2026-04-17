//! Update logic for diff-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

/// Handle diff-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectDiffByIndex(index) => {
            // Load the diff for the selected file on demand.
            let tab = state.active_tab();
            let file_entry = tab.commit_files.get(index);
            let repo_path = tab.repo_path.clone();
            let oid = tab.selected_commit_oid.clone();

            if let (Some(entry), Some(path), Some(oid)) = (file_entry, repo_path, oid) {
                let file_path = entry.display_path().to_string();
                let tab = state.active_tab_mut();
                tab.selected_file_index = Some(index);
                // Keep the previous selected_diff visible while the new one
                // loads — avoids a blink where the file list disappears.
                tab.is_loading_file_diff = true;
                tab.diff_scroll_offset = 0.0;
                crate::features::commits::commands::load_single_file_diff(path, oid, file_path)
            } else {
                Task::none()
            }
        }

        Message::SelectDiff(diff_info) => {
            let tab = state.active_tab_mut();
            tab.selected_diff = Some(diff_info);
            tab.diff_scroll_offset = 0.0;
            Task::none()
        }

        _ => Task::none(),
    }
}
