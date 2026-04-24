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
            let shift_held = state.keyboard_modifiers.shift();

            if shift_held {
                // ── Shift+Click: range selection from anchor to clicked index ──
                let anchor = state
                    .active_tab()
                    .anchor_commit_index
                    .or(state.active_tab().selected_commit)
                    .unwrap_or(index);

                let (start, end) = if anchor <= index {
                    (anchor, index)
                } else {
                    (index, anchor)
                };
                let range: Vec<usize> = (start..=end).collect();

                // Determine the oldest and newest commits in the selection.
                // Commits are stored newest-first, so the highest index is the oldest.
                let oldest_idx = *range.last().unwrap();
                let newest_idx = range[0];
                let oldest_oid = state
                    .active_tab()
                    .commits
                    .get(oldest_idx)
                    .map(|c| c.oid.clone());
                let newest_oid = state
                    .active_tab()
                    .commits
                    .get(newest_idx)
                    .map(|c| c.oid.clone());
                let repo_path = state.active_tab().repo_path.clone();

                let tab = state.active_tab_mut();
                tab.selected_commits = range;
                tab.selected_commit = Some(index);
                tab.commit_range_diffs.clear();
                tab.is_loading_file_diff = true;

                if let (Some(oldest), Some(newest), Some(path)) =
                    (oldest_oid, newest_oid, repo_path)
                {
                    return commands::load_commit_range_diff(path, oldest, newest);
                }
                return Task::none();
            }

            // ── Regular click: single commit, set anchor ──────────────────────
            let repo_path = state.active_tab().repo_path.clone();
            let commit_info = state
                .active_tab()
                .commits
                .get(index)
                .map(|c| (c.oid.clone(), c.short_oid.clone()));

            let tab = state.active_tab_mut();
            tab.anchor_commit_index = Some(index);
            tab.selected_commits.clear();
            tab.selected_commit = Some(index);
            tab.show_commit_detail = true;
            // Clear previous diff state immediately for snappy feedback.
            tab.commit_files.clear();
            tab.selected_diff = None;
            tab.selected_file_index = None;
            tab.diff_scroll_offset = 0.0;
            tab.selected_commit_file_indices.clear();
            tab.multi_file_diffs.clear();
            tab.commit_range_diffs.clear();

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

        Message::DiffMultiWithWorkingTree(oid, file_paths) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                let count = file_paths.len();
                state.active_tab_mut().is_loading_file_diff = true;
                state.active_tab_mut().diff_scroll_offset = 0.0;
                state.active_tab_mut().status_message =
                    Some(format!("Comparing {count} files with working tree…"));
                commands::load_multi_file_commit_vs_workdir(path, oid.clone(), file_paths.clone())
            } else {
                Task::none()
            }
        }

        Message::CheckoutFileAtCommit(oid, file_path) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                state.active_tab_mut().status_message = Some(format!(
                    "Restoring '{}'…",
                    file_path.rsplit('/').next().unwrap_or(&file_path)
                ));
                commands::checkout_file_at_commit(path, oid.clone(), file_path.clone())
            } else {
                Task::none()
            }
        }

        Message::CheckoutMultiFilesAtCommit(oid, file_paths) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                let count = file_paths.len();
                state.active_tab_mut().status_message = Some(format!("Restoring {count} files…"));
                commands::checkout_multi_files_at_commit(path, oid.clone(), file_paths.clone())
            } else {
                Task::none()
            }
        }

        Message::CherryPickCommits(oids) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                let count = oids.len();
                state.active_tab_mut().status_message =
                    Some(format!("Cherry-picking {count} commit(s)…"));
                commands::cherry_pick_commits(path, oids)
            } else {
                Task::none()
            }
        }

        Message::RevertCommits(oids) => {
            state.active_tab_mut().context_menu = None;
            if let Some(path) = state.active_tab().repo_path.clone() {
                let count = oids.len();
                state.active_tab_mut().status_message =
                    Some(format!("Reverting {count} commit(s)…"));
                commands::revert_commits(path, oids)
            } else {
                Task::none()
            }
        }

        Message::CommitRangeDiffLoaded(result) => {
            let tab = state.active_tab_mut();
            tab.is_loading_file_diff = false;
            match result {
                Ok(diffs) => {
                    tab.commit_range_diffs = diffs;
                    tab.diff_scroll_offset = 0.0;
                    let count = tab.selected_commits.len();
                    tab.status_message = Some(format!("Showing combined diff for {count} commits"));
                }
                Err(e) => {
                    tab.commit_range_diffs.clear();
                    tab.error_message = Some(format!("Range diff failed: {e}"));
                }
            }
            Task::none()
        }

        _ => Task::none(),
    }
}
