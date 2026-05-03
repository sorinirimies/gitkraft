//! Update logic for diff-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

/// Handle diff-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectDiffByIndex(index) => {
            let shift_held = state.keyboard_modifiers.shift();
            let tab = state.active_tab();
            let repo_path = tab.repo_path.clone();
            let oid = tab.selected_commit_oid.clone();

            if shift_held {
                // ── Shift+Click: range selection from anchor to clicked index ──
                //
                // The anchor is the file that was last clicked WITHOUT Shift.
                // Every Shift+Click replaces the selection with everything
                // between the anchor and the clicked index (inclusive), exactly
                // like standard file-manager range selection.
                let anchor = state
                    .active_tab()
                    .anchor_file_index
                    .or(state.active_tab().selected_file_index)
                    .unwrap_or(index);

                let (start, end) = if anchor <= index {
                    (anchor, index)
                } else {
                    (index, anchor)
                };

                // Build the range in ascending order so badges are always
                // numbered top-to-bottom.
                let range: Vec<usize> = (start..=end).collect();
                let count = range.len();

                let tab = state.active_tab_mut();
                tab.selected_commit_file_indices = range;
                tab.selected_file_index = Some(index);

                if count == 1 {
                    // Range collapsed to a single file — behave like a normal
                    // single-file selection (no multi-diff panel).
                    tab.multi_file_diffs.clear();
                    let file_entry = tab.commit_files.get(start).cloned();
                    if let (Some(entry), Some(path), Some(oid)) = (file_entry, repo_path, oid) {
                        let file_path = entry.display_path().to_string();
                        let tab = state.active_tab_mut();
                        tab.is_loading_file_diff = true;
                        tab.diff_scroll_offset = 0.0;
                        return crate::features::commits::commands::load_single_file_diff(
                            path, oid, file_path,
                        );
                    }
                } else {
                    // Multiple files in range — load and display them all.
                    tab.selected_diff = None;
                    tab.is_loading_file_diff = true;
                    tab.diff_scroll_offset = 0.0;
                    let file_paths: Vec<String> = tab
                        .selected_commit_file_indices
                        .iter()
                        .filter_map(|&i| {
                            tab.commit_files
                                .get(i)
                                .map(|f| f.display_path().to_string())
                        })
                        .collect();
                    if let (Some(path), Some(oid)) = (repo_path, oid) {
                        return crate::features::commits::commands::load_commit_multi_diffs(
                            path, oid, file_paths,
                        );
                    }
                }
                Task::none()
            } else {
                // ── Regular click: single-file selection, set range anchor ─────
                let tab = state.active_tab_mut();
                tab.anchor_file_index = Some(index); // fix the anchor for future Shift+Clicks
                tab.selected_commit_file_indices.clear();
                // NOTE: multi_file_diffs, commit_range_diffs, and
                // diff_scroll_offset are intentionally NOT cleared here.
                // Clearing them immediately would change the widget-tree
                // structure before the replacement diff is ready, causing the
                // file list panel to visually flash/blink.  Instead we defer
                // those resets to SingleFileDiffLoaded where the new diff
                // content is set atomically.

                let file_entry = tab.commit_files.get(index).cloned();
                if let (Some(entry), Some(path), Some(oid)) = (file_entry, repo_path, oid) {
                    let file_path = entry.display_path().to_string();
                    let tab = state.active_tab_mut();
                    tab.selected_file_index = Some(index);
                    tab.is_loading_file_diff = true;
                    crate::features::commits::commands::load_single_file_diff(path, oid, file_path)
                } else {
                    Task::none()
                }
            }
        }

        Message::SelectDiff(diff_info) => {
            state.active_tab_mut().context_menu = None;
            let tab = state.active_tab_mut();
            tab.selected_diff = Some(diff_info);
            tab.diff_scroll_offset = 0.0;
            Task::none()
        }

        Message::CommitMultiDiffLoaded(result) => {
            let tab = state.active_tab_mut();
            tab.is_loading_file_diff = false;
            match result {
                Ok(diffs) => {
                    tab.multi_file_diffs = diffs;
                    tab.selected_diff = None;
                    tab.diff_scroll_offset = 0.0;
                }
                Err(e) => {
                    tab.multi_file_diffs.clear();
                    tab.error_message = Some(format!("Failed to load multi-file diff: {e}"));
                }
            }
            Task::none()
        }

        _ => Task::none(),
    }
}
