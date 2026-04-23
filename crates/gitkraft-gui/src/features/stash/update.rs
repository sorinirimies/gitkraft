//! Update logic for stash-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all stash-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::StashMessageChanged(msg) => {
            state.active_tab_mut().stash_message = msg;
            Task::none()
        }

        Message::StashSave => {
            // Derive the optional stash message before the macro borrows state.
            let msg = {
                let tab = state.active_tab();
                if tab.stash_message.trim().is_empty() {
                    None
                } else {
                    Some(tab.stash_message.trim().to_string())
                }
            };
            with_repo!(state, loading, "Saving stash…".into(), |repo_path| {
                state.active_tab_mut().stash_message.clear();
                commands::stash_save(repo_path, msg)
            })
        }

        Message::StashPop(index) => with_repo!(
            state,
            loading,
            format!("Popping stash@{{{index}}}…"),
            |repo_path| commands::stash_pop(repo_path, index)
        ),

        Message::StashDrop(index) => with_repo!(
            state,
            loading,
            format!("Dropping stash@{{{index}}}…"),
            |repo_path| commands::stash_drop(repo_path, index)
        ),

        Message::StashApply(index) => with_repo!(
            state,
            loading,
            format!("Applying stash@{{{index}}}…"),
            |repo_path| commands::stash_apply(repo_path, index)
        ),

        Message::ViewStashDiff(index) => {
            state.active_tab_mut().context_menu = None;
            with_repo!(state, "Loading stash diff…".into(), |repo_path| {
                commands::load_stash_diff(repo_path, index)
            })
        }

        Message::StashDiffLoaded(result) => {
            match result {
                Ok(diffs) => {
                    let tab = state.active_tab_mut();
                    tab.selected_diff = diffs.first().cloned();
                    tab.commit_files = diffs
                        .iter()
                        .map(|d| gitkraft_core::DiffFileEntry {
                            old_file: d.old_file.clone(),
                            new_file: d.new_file.clone(),
                            status: d.status.clone(),
                        })
                        .collect();
                    tab.selected_file_index = Some(0);
                    tab.status_message =
                        Some(format!("{} file(s) in stash", tab.commit_files.len()));
                }
                Err(e) => {
                    state.active_tab_mut().error_message =
                        Some(format!("Failed to load stash diff: {e}"));
                }
            }
            Task::none()
        }

        Message::StashUpdated(result) => {
            state.active_tab_mut().is_loading = false;
            match result {
                Ok(stashes) => {
                    {
                        let tab = state.active_tab_mut();
                        tab.stashes = stashes;
                        tab.status_message = Some("Stash updated.".into());
                    }
                    state.refresh_active_tab()
                }
                Err(e) => {
                    let tab = state.active_tab_mut();
                    tab.error_message = Some(format!("Stash operation failed: {e}"));
                    tab.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
