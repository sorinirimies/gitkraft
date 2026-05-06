//! Update logic for staging-related messages.

use std::collections::HashSet;

use gitkraft_core::DiffInfo;
use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Shared logic for shift-click / regular-click file selection in the staging area.
/// Updates `selection` and `anchor` based on click type, then returns the diff
/// for the clicked file (if found) so the caller can set `selected_diff`.
fn toggle_staging_file(
    changes: &[DiffInfo],
    selection: &mut HashSet<String>,
    anchor: &mut Option<usize>,
    path: &str,
    shift_held: bool,
) -> Option<DiffInfo> {
    let clicked_idx = changes.iter().position(|d| d.display_path() == path);

    if shift_held {
        if let Some(idx) = clicked_idx {
            let range = crate::view_utils::shift_click_range(*anchor, idx);
            selection.clear();
            for i in &range {
                if let Some(d) = changes.get(*i) {
                    selection.insert(d.display_path().to_string());
                }
            }
        }
    } else {
        if selection.contains(path) {
            selection.remove(path);
        } else {
            selection.insert(path.to_string());
        }
        if let Some(idx) = clicked_idx {
            *anchor = Some(idx);
        }
    }

    changes.iter().find(|d| d.display_path() == path).cloned()
}

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
            let diff = toggle_staging_file(
                &tab.unstaged_changes,
                &mut tab.selected_unstaged,
                &mut tab.anchor_unstaged_index,
                &path,
                shift_held,
            );
            if let Some(diff) = diff {
                tab.selected_diff = Some(diff);
                tab.diff_scroll_offset = 0.0;
            }
            Task::none()
        }

        Message::ToggleSelectStaged(path) => {
            let shift_held = state.keyboard_modifiers.shift();
            let tab = state.active_tab_mut();
            let diff = toggle_staging_file(
                &tab.staged_changes,
                &mut tab.selected_staged,
                &mut tab.anchor_staged_index,
                &path,
                shift_held,
            );
            if let Some(diff) = diff {
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
