//! Update logic for repository-level messages.

use iced::Task;

use crate::message::{Message, RepoPayload};
use crate::state::GitKraft;

use super::commands;

/// Handle all repository-related messages, returning a [`Task`] for any
/// follow-up async work.
pub(crate) fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::OpenRepo => {
            // If the active tab already has a repo open, create a new tab
            // so the user doesn't lose their current work.
            if state.active_tab().has_repo() {
                state.tabs.push(crate::state::RepoTab::new_empty());
                state.active_tab = state.tabs.len() - 1;
            }
            let tab = state.active_tab_mut();
            tab.is_loading = true;
            tab.status_message = Some("Opening folder picker…".into());
            commands::pick_folder_open()
        }

        Message::InitRepo => {
            let tab = state.active_tab_mut();
            tab.is_loading = true;
            tab.status_message = Some("Opening folder picker for init…".into());
            commands::pick_folder_init()
        }

        Message::RepoSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                // Canonicalize so the picker path matches stored repo_path
                // (git2's workdir is always canonical / symlink-resolved).
                let path = path.canonicalize().unwrap_or(path);

                // If this repo is already open in another tab, switch to it
                // instead of opening a duplicate.
                if state.switch_to_existing_tab(path.as_path()) {
                    let tab = state.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = None;
                    return Task::none();
                }
                let tab = state.active_tab_mut();
                tab.repo_path = Some(path.clone());
                tab.status_message = Some(format!("Opening {}…", path.display()));
                commands::load_repo(path)
            } else {
                // User cancelled the folder picker — clean up the empty tab
                // if we created one for this operation.
                if !state.active_tab().has_repo() && state.tabs.len() > 1 {
                    state.tabs.remove(state.active_tab);
                    if state.active_tab >= state.tabs.len() {
                        state.active_tab = state.tabs.len() - 1;
                    }
                } else {
                    let tab = state.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = None;
                }
                Task::none()
            }
        }

        Message::RepoInitSelected(maybe_path) => {
            if let Some(path) = maybe_path {
                let tab = state.active_tab_mut();
                tab.status_message = Some(format!("Initializing {}…", path.display()));
                commands::init_repo(path)
            } else {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.status_message = None;
                Task::none()
            }
        }

        Message::RepoOpened(result) => handle_repo_loaded(state, result),

        Message::RefreshRepo => with_repo!(state, loading, "Refreshing…".into(), |path| {
            let depth = state.active_tab().commits.len();
            commands::refresh_repo(path, depth)
        }),

        Message::RepoRefreshed(result) => handle_repo_loaded(state, result),

        Message::OpenRecentRepo(path) => {
            // Canonicalize for consistent comparison with stored paths.
            let path = path.canonicalize().unwrap_or(path);

            // If this repo is already open in another tab, switch to it.
            if state.switch_to_existing_tab(path.as_path()) {
                return Task::none();
            }
            // If the active tab already has a repo open, create a new tab.
            if state.active_tab().has_repo() {
                state.tabs.push(crate::state::RepoTab::new_empty());
                state.active_tab = state.tabs.len() - 1;
            }
            let tab = state.active_tab_mut();
            tab.repo_path = Some(path.clone());
            tab.is_loading = true;
            tab.status_message = Some(format!("Opening {}…", path.display()));
            commands::load_repo(path)
        }

        Message::CloseRepo => {
            // When multiple tabs are open, remove the current tab and switch
            // to the adjacent one so the user lands on their other open repo.
            // When only one tab is open, replace it with a fresh empty one to
            // show the welcome screen (we never fully remove the last tab).
            if state.tabs.len() > 1 {
                state.tabs.remove(state.active_tab);
                if state.active_tab >= state.tabs.len() {
                    state.active_tab = state.tabs.len() - 1;
                }
                // active_tab already points to the next tab; no further adjustment.
            } else {
                state.tabs[state.active_tab] = crate::state::RepoTab::new_empty();
            }

            // Reset drag state (these fields remain on GitKraft).
            state.dragging = None;
            state.dragging_h = None;
            state.drag_initialized = false;
            state.drag_initialized_h = false;

            // Persist the updated session, layout, and refresh recent repos from disk.
            let (open_tabs, active) = state.session_state();
            Task::batch([
                commands::load_recent_repos_async(),
                commands::save_session_async(open_tabs, active),
                commands::save_layout_async(state.current_layout()),
            ])
        }

        Message::RepoRestoredAt(tab_index, result) => {
            handle_repo_loaded_at(state, tab_index, result)
        }

        // ── Async persistence results ─────────────────────────────────────
        Message::RepoRecorded(result) => {
            if let Ok(recent) = result {
                state.recent_repos = recent;
            }
            // Errors are silently ignored — persistence is best-effort.
            Task::none()
        }

        Message::SettingsLoaded(result) => {
            if let Ok(recent) = result {
                state.recent_repos = recent;
            }
            Task::none()
        }

        Message::MoreCommitsLoaded(result) => handle_more_commits_loaded(state, result),

        Message::GitOperationResult(result) => handle_repo_loaded(state, result),

        _ => Task::none(),
    }
}

/// Shared handler for both `RepoOpened` and `RepoRefreshed` — they carry the
/// same payload and should update the same state fields.
///
/// When a refresh result arrives, the active tab may have changed since the
/// refresh was initiated (e.g. the user opened a new tab). To prevent writing
/// the payload into the wrong tab, we resolve the target tab by matching the
/// repo path from the payload against all open tabs, falling back to the active
/// tab only for brand-new opens (where no tab has that path yet).
///
/// Persistence (recording the repo open and refreshing the recent-repos list)
/// is dispatched as an async [`Task`] so the settings file I/O never blocks the UI.
fn handle_repo_loaded(state: &mut GitKraft, result: Result<RepoPayload, String>) -> Task<Message> {
    match result {
        Ok(payload) => {
            // Derive the workdir path (preferred) or fall back to the .git path,
            // then canonicalize for consistent comparison with stored paths.
            let path = payload
                .info
                .workdir
                .clone()
                .unwrap_or_else(|| payload.info.path.clone());
            let path = path.canonicalize().unwrap_or(path);

            // Find the tab that owns this repo path. If none match (brand-new
            // open), fall back to the active tab.
            let target_idx = state
                .tabs
                .iter()
                .position(|t| t.repo_path.as_deref() == Some(path.as_path()))
                .unwrap_or(state.active_tab);

            let tab = &mut state.tabs[target_idx];
            // Detect whether this is a refresh (tab already had a repo) or a
            // brand-new open so we can decide whether to reset the scroll.
            let is_new_open = tab.repo_info.is_none();
            tab.is_loading = false;
            tab.apply_payload(payload, path.clone());
            // Always rebuild commit_display after apply_payload.
            // The previous "only rebuild when count changes" optimisation
            // skipped the rebuild after amend/rebase/force-push where the
            // commit count stays the same but the content changes, leaving
            // stale author names and timestamps in the display.
            // compute_commit_display is O(n) string formatting — fast enough
            // to run unconditionally even for large histories.
            tab.commit_display = compute_commit_display(&tab.commits);
            tab.branch_context = compute_branch_context(&tab.commits, &tab.graph_rows);

            // Record the repo open AND persist the full session in one atomic
            // write, on a background thread so settings file I/O never blocks the UI.
            let (open_tabs, active) = state.session_state();
            let mut tasks = vec![commands::record_repo_and_save_session_async(
                path, open_tabs, active,
            )];

            // Only scroll to the top for brand-new opens; background refreshes
            // must preserve the user's current scroll position.
            if is_new_open {
                tasks.push(iced::widget::operation::scroll_to(
                    crate::features::commits::view::commit_log_scroll_id(target_idx),
                    iced::widget::operation::AbsoluteOffset { x: 0.0, y: 0.0 },
                ));
            }
            Task::batch(tasks)
        }
        Err(e) => {
            let tab = state.active_tab_mut();
            tab.is_loading = false;
            tab.error_message = Some(e);
            tab.status_message = None;
            Task::none()
        }
    }
}

/// Like `handle_repo_loaded` but writes into a specific tab index.
/// Used for parallel startup restore; does NOT record a repo open.
fn handle_repo_loaded_at(
    state: &mut GitKraft,
    tab_index: usize,
    result: Result<RepoPayload, String>,
) -> Task<Message> {
    if tab_index >= state.tabs.len() {
        return Task::none(); // tab closed before restore completed
    }
    state.tabs[tab_index].is_loading = false;
    match result {
        Ok(payload) => {
            let path = payload
                .info
                .workdir
                .clone()
                .unwrap_or_else(|| payload.info.path.clone());
            let tab = &mut state.tabs[tab_index];
            tab.apply_payload(payload, path);
            tab.commit_display = compute_commit_display(&tab.commits);
            tab.branch_context = compute_branch_context(&tab.commits, &tab.graph_rows);
            // Already in recent_repos — no need to re-record.
            Task::none()
        }
        Err(e) => {
            let tab = &mut state.tabs[tab_index];
            tab.error_message = Some(e);
            tab.status_message = None;
            Task::none()
        }
    }
}

/// Append a newly loaded commit page to the active tab's commit log.
fn handle_more_commits_loaded(
    state: &mut GitKraft,
    result: Result<crate::message::CommitPage, String>,
) -> Task<Message> {
    let tab = state.active_tab_mut();
    tab.is_loading_more_commits = false;
    match result {
        Ok(page) => {
            let prev_count = tab.commits.len();
            let new_total = page.commits.len();
            // If the server returned no new commits, we've hit the end.
            tab.has_more_commits = new_total > prev_count;
            if new_total > prev_count {
                // Only compute display strings for the newly added commits.
                let new_display = compute_commit_display(&page.commits[prev_count..]);
                tab.commit_display.extend(new_display);
            }
            tab.commits = page.commits;
            tab.graph_rows = page.graph_rows;
            // Rebuild full branch context since lane assignments may have changed.
            tab.branch_context = compute_branch_context(&tab.commits, &tab.graph_rows);
        }
        Err(e) => {
            tab.status_message = Some(format!("Failed to load more commits: {e}"));
        }
    }
    Task::none()
}

/// Pre-compute display strings for the commit log so the view function
/// never allocates strings on the hot rendering path.
fn compute_commit_display(commits: &[gitkraft_core::CommitInfo]) -> Vec<(String, String)> {
    commits
        .iter()
        .map(|c| {
            let time = gitkraft_core::utils::relative_time(c.time);
            // Truncate author to fit in the fixed-width author column (~90 px).
            let author = gitkraft_core::truncate_str(&c.author_name, 20);
            (time, author)
        })
        .collect()
}

/// Pre-compute the inherited branch name for each commit row.
///
/// For each graph lane, we track the last-seen branch/tag ref name.
/// When a commit has an explicit ref, it uses that (the real badge is shown).
/// When it doesn't, it inherits the branch name from the nearest ancestor
/// in the same lane.  This allows the UI to show a ghost/semi-transparent
/// branch label on hover even for commits without explicit refs.
fn compute_branch_context(
    commits: &[gitkraft_core::CommitInfo],
    graph_rows: &[gitkraft_core::GraphRow],
) -> Vec<Option<String>> {
    if commits.len() != graph_rows.len() {
        return vec![None; commits.len()];
    }

    // Track the current branch name per lane (column).
    let max_lanes = graph_rows.iter().map(|r| r.width).max().unwrap_or(1);
    let mut lane_labels: Vec<Option<String>> = vec![None; max_lanes];
    let mut result = Vec::with_capacity(commits.len());

    for (i, commit) in commits.iter().enumerate() {
        let col = graph_rows[i].node_column;

        // If this commit has branch/tag refs, update the lane label.
        if let Some(first_ref) = commit.refs.first() {
            if col < lane_labels.len() {
                lane_labels[col] = Some(first_ref.name.clone());
            }
            // Commits with explicit refs show real badges, so no ghost needed.
            result.push(None);
        } else {
            // Inherit from the lane.
            let label = if col < lane_labels.len() {
                lane_labels[col].clone()
            } else {
                None
            };
            result.push(label);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_commit(oid: &str, refs: Vec<gitkraft_core::RefLabel>) -> gitkraft_core::CommitInfo {
        gitkraft_core::CommitInfo {
            oid: oid.to_string(),
            summary: String::new(),
            message: String::new(),
            author_name: String::new(),
            author_email: String::new(),
            time: Default::default(),
            parent_ids: Vec::new(),
            refs,
        }
    }

    fn make_graph_row(node_column: usize) -> gitkraft_core::GraphRow {
        gitkraft_core::GraphRow {
            width: node_column + 1,
            node_column,
            node_color: 0,
            edges: Vec::new(),
        }
    }

    #[test]
    fn branch_context_empty_when_no_commits() {
        let result = compute_branch_context(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn branch_context_none_for_commits_with_refs() {
        let refs = vec![gitkraft_core::RefLabel {
            name: "main".to_string(),
            kind: gitkraft_core::RefKind::Head,
        }];
        let commits = vec![make_commit("aaa", refs)];
        let graph = vec![make_graph_row(0)];
        let result = compute_branch_context(&commits, &graph);
        // Commits with explicit refs get None (real badges shown)
        assert_eq!(result, vec![None]);
    }

    #[test]
    fn branch_context_inherits_from_ancestor_in_same_lane() {
        let refs = vec![gitkraft_core::RefLabel {
            name: "main".to_string(),
            kind: gitkraft_core::RefKind::Head,
        }];
        let commits = vec![
            make_commit("tip", refs),       // has ref "main"
            make_commit("mid", Vec::new()), // no refs, same lane
            make_commit("old", Vec::new()), // no refs, same lane
        ];
        let graph = vec![make_graph_row(0), make_graph_row(0), make_graph_row(0)];
        let result = compute_branch_context(&commits, &graph);
        assert_eq!(result[0], None); // has explicit ref
        assert_eq!(result[1], Some("main".to_string())); // inherited
        assert_eq!(result[2], Some("main".to_string())); // inherited
    }

    #[test]
    fn branch_context_separate_lanes_get_separate_labels() {
        let main_ref = vec![gitkraft_core::RefLabel {
            name: "main".to_string(),
            kind: gitkraft_core::RefKind::Head,
        }];
        let feat_ref = vec![gitkraft_core::RefLabel {
            name: "feature".to_string(),
            kind: gitkraft_core::RefKind::LocalBranch,
        }];
        let commits = vec![
            make_commit("a", main_ref),   // lane 0, "main"
            make_commit("b", feat_ref),   // lane 1, "feature"
            make_commit("c", Vec::new()), // lane 0 -> inherits "main"
            make_commit("d", Vec::new()), // lane 1 -> inherits "feature"
        ];
        let graph = vec![
            make_graph_row(0),
            gitkraft_core::GraphRow {
                width: 2,
                node_column: 1,
                node_color: 1,
                edges: Vec::new(),
            },
            make_graph_row(0),
            gitkraft_core::GraphRow {
                width: 2,
                node_column: 1,
                node_color: 1,
                edges: Vec::new(),
            },
        ];
        let result = compute_branch_context(&commits, &graph);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some("main".to_string()));
        assert_eq!(result[3], Some("feature".to_string()));
    }

    #[test]
    fn branch_context_mismatched_lengths_returns_all_none() {
        let commits = vec![make_commit("a", Vec::new())];
        let graph = vec![]; // mismatched
        let result = compute_branch_context(&commits, &graph);
        assert_eq!(result, vec![None]);
    }
}
