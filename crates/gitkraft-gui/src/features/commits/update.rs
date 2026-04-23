//! Update logic for commit-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all commit-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::SelectCommit(index) => {
            // Read repo_path and commit info before taking a mutable borrow.
            let repo_path = state.active_tab().repo_path.clone();
            let commit_info = state
                .active_tab()
                .commits
                .get(index)
                .map(|c| (c.oid.clone(), c.short_oid.clone()));

            let tab = state.active_tab_mut();
            tab.selected_commit = Some(index);
            tab.show_commit_detail = true;
            // Clear previous diff state immediately for snappy feedback.
            tab.commit_files.clear();
            tab.selected_diff = None;
            tab.selected_file_index = None;
            tab.diff_scroll_offset = 0.0;

            // Load just the file list (instant — no line parsing).
            if let (Some(path), Some((oid, short_oid))) = (repo_path, commit_info) {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Loading files for {short_oid}…"));
                tab.selected_commit_oid = Some(oid.clone());
                commands::load_commit_file_list(path, oid)
            } else {
                Task::none()
            }
        }

        Message::CommitFileListLoaded(result) => {
            match result {
                Ok(files) => {
                    let file_count = files.len();
                    let tab = state.active_tab_mut();
                    tab.commit_files = files;
                    tab.status_message = Some(format!("{file_count} file(s) changed."));

                    // Auto-select the first file and load its diff.
                    if file_count > 0 {
                        let first_file = &tab.commit_files[0];
                        let file_path = first_file.display_path().to_string();
                        tab.selected_file_index = Some(0);
                        tab.is_loading_file_diff = true;

                        if let (Some(repo_path), Some(oid)) =
                            (tab.repo_path.clone(), tab.selected_commit_oid.clone())
                        {
                            return commands::load_single_file_diff(repo_path, oid, file_path);
                        }
                    }
                }
                Err(e) => {
                    let tab = state.active_tab_mut();
                    tab.commit_files.clear();
                    tab.error_message = Some(format!("Failed to load commit files: {e}"));
                    tab.status_message = None;
                }
            }
            Task::none()
        }

        Message::SingleFileDiffLoaded(result) => {
            let tab = state.active_tab_mut();
            tab.is_loading_file_diff = false;
            match result {
                Ok(diff) => {
                    tab.selected_diff = Some(diff);
                    tab.diff_scroll_offset = 0.0;
                }
                Err(e) => {
                    tab.selected_diff = None;
                    tab.error_message = Some(format!("Failed to load file diff: {e}"));
                }
            }
            Task::none()
        }

        Message::CommitMessageChanged(msg) => {
            state.active_tab_mut().commit_message = msg;
            Task::none()
        }

        Message::CreateCommit => {
            let msg;
            let staged_empty;
            {
                let tab = state.active_tab();
                msg = tab.commit_message.trim().to_string();
                staged_empty = tab.staged_changes.is_empty();
            }
            if msg.is_empty() || staged_empty {
                return Task::none();
            }
            with_repo!(state, loading, "Creating commit…".into(), |repo_path| {
                commands::create_commit(repo_path, msg)
            })
        }

        Message::CommitCreated(result) => {
            state.active_tab_mut().is_loading = false;
            match result {
                Ok(()) => {
                    {
                        let tab = state.active_tab_mut();
                        tab.commit_message.clear();
                        tab.status_message = Some("Commit created.".into());
                    }
                    state.refresh_active_tab()
                }
                Err(e) => {
                    let tab = state.active_tab_mut();
                    tab.error_message = Some(format!("Commit failed: {e}"));
                    tab.status_message = None;
                    Task::none()
                }
            }
        }

        Message::DiffFileWithWorkingTree(oid, file_path) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                state.active_tab_mut().status_message = Some(format!(
                    "Comparing {} with working tree…",
                    file_path.rsplit('/').next().unwrap_or(&file_path)
                ));
                commands::diff_file_with_working_tree(path, oid, file_path)
            } else {
                Task::none()
            }
        }

        Message::DiffWithWorkingTreeLoaded(result) => {
            match result {
                Ok(diff) => {
                    let tab = state.active_tab_mut();
                    tab.selected_diff = Some(diff);
                    tab.diff_scroll_offset = 0.0;
                    tab.status_message = Some("Showing diff against working tree".into());
                }
                Err(e) => {
                    state.active_tab_mut().status_message = Some(format!("⚠ {e}"));
                }
            }
            Task::none()
        }

        _ => Task::none(),
    }
}
