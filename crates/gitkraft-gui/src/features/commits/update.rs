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

            // Load the diff for the selected commit.
            // SelectCommit has a compound condition (repo_path AND commit_info)
            // that doesn't fit the with_repo! pattern, so it stays explicit.
            if let (Some(path), Some((oid, short_oid))) = (repo_path, commit_info) {
                state.active_tab_mut().status_message =
                    Some(format!("Loading diff for {short_oid}…"));
                commands::load_commit_diff(path, oid)
            } else {
                Task::none()
            }
        }

        Message::CommitDiffLoaded(result) => {
            let tab = state.active_tab_mut();
            match result {
                Ok(diffs) => {
                    // Store the full file list so the diff viewer can show a
                    // clickable file sidebar, and auto-select the first file.
                    tab.selected_diff = diffs.first().cloned();
                    tab.status_message = Some(format!("Loaded {} file(s) in diff.", diffs.len()));
                    tab.commit_diffs = diffs;
                }
                Err(e) => {
                    tab.commit_diffs.clear();
                    tab.error_message = Some(format!("Failed to load commit diff: {e}"));
                    tab.status_message = None;
                }
            }
            Task::none()
        }

        Message::CommitMessageChanged(msg) => {
            state.active_tab_mut().commit_message = msg;
            Task::none()
        }

        Message::CreateCommit => {
            // Derive the values we need from an immutable borrow before the
            // with_repo! macro takes its own borrows.
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

        _ => Task::none(),
    }
}
