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
            state.active_tab_mut().context_menu = None;
            with_repo!(state, format!("Staging '{path}'…"), |repo_path| {
                commands::stage_file(repo_path, path)
            })
        }

        Message::UnstageFile(path) => {
            state.active_tab_mut().context_menu = None;
            with_repo!(state, format!("Unstaging '{path}'…"), |repo_path| {
                commands::unstage_file(repo_path, path)
            })
        }

        Message::StageAll => {
            state.active_tab_mut().context_menu = None;
            with_repo!(state, "Staging all files…".into(), |repo_path| {
                commands::stage_all(repo_path)
            })
        }

        Message::UnstageAll => {
            state.active_tab_mut().context_menu = None;
            with_repo!(state, "Unstaging all files…".into(), |repo_path| {
                commands::unstage_all(repo_path)
            })
        }

        Message::DiscardFile(path) => {
            state.active_tab_mut().context_menu = None;
            state.active_tab_mut().pending_discard = None;
            with_repo!(state, format!("Discarding '{path}'…"), |repo_path| {
                commands::discard_file(repo_path, path)
            })
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

        Message::ToggleSelectUnstaged(path) => {
            let shift_held = state.keyboard_modifiers.shift();
            let tab = state.active_tab_mut();

            // Find the index of the clicked file.
            let clicked_idx = tab
                .unstaged_changes
                .iter()
                .position(|d| d.display_path() == path);

            if shift_held {
                // Shift+Click: range selection from anchor to clicked index.
                if let Some(idx) = clicked_idx {
                    let anchor = tab.anchor_unstaged_index.unwrap_or(idx);
                    let range = gitkraft_core::ascending_range(anchor, idx);
                    tab.selected_unstaged.clear();
                    for i in &range {
                        if let Some(d) = tab.unstaged_changes.get(*i) {
                            tab.selected_unstaged.insert(d.display_path().to_string());
                        }
                    }
                }
            } else {
                // Regular click: toggle single file, set anchor.
                if tab.selected_unstaged.contains(&path) {
                    tab.selected_unstaged.remove(&path);
                } else {
                    tab.selected_unstaged.insert(path.clone());
                }
                if let Some(idx) = clicked_idx {
                    tab.anchor_unstaged_index = Some(idx);
                }
            }

            // Show the clicked file's diff in the diff pane.
            if let Some(diff) = tab
                .unstaged_changes
                .iter()
                .find(|d| d.display_path() == path)
                .cloned()
            {
                tab.selected_diff = Some(diff);
                tab.diff_scroll_offset = 0.0;
            }
            Task::none()
        }

        Message::ToggleSelectStaged(path) => {
            let shift_held = state.keyboard_modifiers.shift();
            let tab = state.active_tab_mut();

            let clicked_idx = tab
                .staged_changes
                .iter()
                .position(|d| d.display_path() == path);

            if shift_held {
                if let Some(idx) = clicked_idx {
                    let anchor = tab.anchor_staged_index.unwrap_or(idx);
                    let range = gitkraft_core::ascending_range(anchor, idx);
                    tab.selected_staged.clear();
                    for i in &range {
                        if let Some(d) = tab.staged_changes.get(*i) {
                            tab.selected_staged.insert(d.display_path().to_string());
                        }
                    }
                }
            } else {
                if tab.selected_staged.contains(&path) {
                    tab.selected_staged.remove(&path);
                } else {
                    tab.selected_staged.insert(path.clone());
                }
                if let Some(idx) = clicked_idx {
                    tab.anchor_staged_index = Some(idx);
                }
            }

            if let Some(diff) = tab
                .staged_changes
                .iter()
                .find(|d| d.display_path() == path)
                .cloned()
            {
                tab.selected_diff = Some(diff);
                tab.diff_scroll_offset = 0.0;
            }
            Task::none()
        }

        Message::StageSelected => {
            let paths: Vec<String> = state
                .active_tab()
                .selected_unstaged
                .iter()
                .cloned()
                .collect();
            if paths.is_empty() {
                return Task::none();
            }
            state.active_tab_mut().selected_unstaged.clear();
            state.active_tab_mut().context_menu = None;
            with_repo!(
                state,
                format!("Staging {} file(s)…", paths.len()),
                |repo_path| commands::stage_files(repo_path, paths)
            )
        }

        Message::UnstageSelected => {
            let paths: Vec<String> = state.active_tab().selected_staged.iter().cloned().collect();
            if paths.is_empty() {
                return Task::none();
            }
            state.active_tab_mut().selected_staged.clear();
            state.active_tab_mut().context_menu = None;
            with_repo!(
                state,
                format!("Unstaging {} file(s)…", paths.len()),
                |repo_path| commands::unstage_files(repo_path, paths)
            )
        }

        Message::DiscardSelected => {
            let unstaged_paths: Vec<String> = state
                .active_tab()
                .selected_unstaged
                .iter()
                .cloned()
                .collect();
            let staged_paths: Vec<String> =
                state.active_tab().selected_staged.iter().cloned().collect();

            if unstaged_paths.is_empty() && staged_paths.is_empty() {
                return Task::none();
            }

            let total = unstaged_paths.len() + staged_paths.len();
            state.active_tab_mut().selected_unstaged.clear();
            state.active_tab_mut().selected_staged.clear();
            state.active_tab_mut().context_menu = None;
            with_repo!(
                state,
                format!("Discarding {} file(s)…", total),
                |repo_path| commands::discard_all_selected(repo_path, unstaged_paths, staged_paths)
            )
        }

        Message::DiscardStagedFile(path) => {
            state.active_tab_mut().context_menu = None;
            with_repo!(state, format!("Discarding '{path}'…"), |repo_path| {
                commands::discard_staged_file(repo_path, path)
            })
        }

        _ => Task::none(),
    }
}
