use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// True when the commit-action popup is open.
pub fn popup_is_open(app: &App) -> bool {
    app.tab().pending_commit_action_oid.is_some()
}

/// Get the OID of the commit at `idx` from the active commit list (search results or all commits).
fn get_commit_oid_at(app: &App, idx: usize) -> Option<String> {
    let commits = if app.tab().search_active && !app.tab().search_results.is_empty() {
        &app.tab().search_results
    } else {
        &app.tab().commits
    };
    commits.get(idx).map(|c| c.oid.clone())
}

/// Trigger a background diff load for the commit at `idx`.
fn load_diff_at(app: &mut App, idx: usize) {
    let oid = get_commit_oid_at(app, idx);
    if let Some(oid) = oid {
        app.tab_mut().selected_commit_oid = Some(oid);
        app.load_commit_diff_by_oid();
    }
}

/// Handle keys when the CommitLog pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // If the action popup is open, route keys there instead
    if popup_is_open(app) {
        handle_popup_key(app, key);
        return;
    }
    match key.code {
        KeyCode::Char('j') => {
            navigate_down(app);
        }
        KeyCode::Char('k') => {
            navigate_up(app);
        }
        KeyCode::Enter => {
            // Load the diff for the selected commit
            if let Some(idx) = app.tab().commit_list_state.selected() {
                let commits = if app.tab().search_active && !app.tab().search_results.is_empty() {
                    &app.tab().search_results
                } else {
                    &app.tab().commits
                };
                if idx < commits.len() {
                    let oid = commits[idx].oid.clone();
                    app.tab_mut().selected_commit_oid = Some(oid);
                    app.load_commit_diff_by_oid();
                }
            }
        }
        KeyCode::Char('g') => {
            let len = active_commits_len(app);
            if len > 0 {
                // Jump to first commit
                app.tab_mut().commit_list_state.select(Some(0));
            }
        }
        KeyCode::Char('G') => {
            let len = active_commits_len(app);
            if len > 0 {
                // Jump to last commit
                app.tab_mut().commit_list_state.select(Some(len - 1));
            }
        }
        // Toggle current commit in/out of multi-selection, then advance
        KeyCode::Char(' ') => {
            if let Some(idx) = app.tab().commit_list_state.selected() {
                let commits_len = active_commits_len(app);
                let tab = app.tab_mut();
                if let Some(pos) = tab.selected_commits.iter().position(|&i| i == idx) {
                    tab.selected_commits.remove(pos);
                } else {
                    tab.selected_commits.push(idx);
                }
                let count = tab.selected_commits.len();
                tab.status_message = if count > 0 {
                    Some(format!("{count} commit(s) selected"))
                } else {
                    None
                };
                // Auto-advance to next commit (like Space does in staging)
                if idx + 1 < commits_len {
                    tab.commit_list_state.select(Some(idx + 1));
                }
            }
        }

        // Revert selected commit
        KeyCode::Char('e') => {
            app.revert_selected_commit();
        }

        // Reset soft to selected commit
        KeyCode::Char('x') => {
            app.reset_to_selected_commit("soft");
        }

        // Reset hard to selected commit
        KeyCode::Char('X') => {
            app.reset_to_selected_commit("hard");
        }

        // Force push current branch
        KeyCode::Char('F') => {
            app.force_push_branch();
        }

        KeyCode::Esc => {
            if app.tab().search_active {
                app.tab_mut().search_active = false;
                app.tab_mut().search_results.clear();
                app.tab_mut().search_query.clear();
                app.tab_mut().status_message = Some("Search cleared".into());
            } else {
                app.tab_mut().commit_list_state.select(None);
            }
        }
        _ => {}
    }
}

/// Handle keys when the commit-action popup is open.
pub fn handle_popup_key(app: &mut App, key: KeyEvent) {
    use crate::app::{InputMode, InputPurpose};

    match key.code {
        // Navigate down in the popup
        KeyCode::Char('j') | KeyCode::Down => {
            let len = app.tab().commit_action_items.len();
            if len > 0 {
                let cur = app.tab().commit_action_cursor;
                app.tab_mut().commit_action_cursor = (cur + 1).min(len - 1);
            }
        }
        // Navigate up in the popup
        KeyCode::Char('k') | KeyCode::Up => {
            let cur = app.tab().commit_action_cursor;
            app.tab_mut().commit_action_cursor = cur.saturating_sub(1);
        }
        // Confirm selection
        KeyCode::Enter | KeyCode::Char(' ') => {
            let cursor = app.tab().commit_action_cursor;
            let kind = match app.tab().commit_action_items.get(cursor).copied() {
                Some(k) => k,
                None => return,
            };
            if kind.needs_input() {
                // Park the kind and ask for the first input
                app.tab_mut().pending_action_kind = Some(kind);
                app.tab_mut().action_input1.clear();
                app.input_buffer.clear();
                app.input_mode = InputMode::Input;
                app.input_purpose = InputPurpose::CommitActionInput1;
                let prompt = kind.input_prompt().unwrap_or("Input:");
                app.tab_mut().status_message = Some(prompt.to_string());
                // Close the popup list (keep pending_commit_action_oid so
                // execute_commit_action can find the OID)
                app.tab_mut().commit_action_items.clear();
            } else {
                // No input needed — execute directly
                let action = kind.into_action(String::new(), String::new());
                app.execute_commit_action(action);
            }
        }
        // Cancel
        KeyCode::Esc | KeyCode::Char('q') => {
            let tab = app.tab_mut();
            tab.pending_commit_action_oid = None;
            tab.commit_action_items.clear();
            tab.commit_action_cursor = 0;
            tab.status_message = Some("Action cancelled".into());
        }
        _ => {}
    }
}

/// Return the length of the currently visible commit list (search results or all commits).
fn active_commits_len(app: &App) -> usize {
    if app.tab().search_active && !app.tab().search_results.is_empty() {
        app.tab().search_results.len()
    } else {
        app.tab().commits.len()
    }
}

/// Move commit selection down by one and auto-load the diff for the new selection.
pub fn navigate_down(app: &mut App) {
    let len = active_commits_len(app);
    if len == 0 {
        return;
    }
    let i = match app.tab().commit_list_state.selected() {
        Some(i) => {
            if i >= len - 1 {
                i
            } else {
                i + 1
            }
        }
        None => 0,
    };
    app.tab_mut().commit_list_state.select(Some(i));
    app.tab_mut().anchor_commit_index = Some(i);
    app.tab_mut().selected_commits.clear();
    app.tab_mut().commit_range_diffs.clear();
    load_diff_at(app, i);
}

/// Move commit selection up by one and auto-load the diff for the new selection.
pub fn navigate_up(app: &mut App) {
    let len = active_commits_len(app);
    if len == 0 {
        return;
    }
    let i = match app.tab().commit_list_state.selected() {
        Some(i) => {
            if i == 0 {
                0
            } else {
                i - 1
            }
        }
        None => 0,
    };
    app.tab_mut().commit_list_state.select(Some(i));
    app.tab_mut().anchor_commit_index = Some(i);
    app.tab_mut().selected_commits.clear();
    app.tab_mut().commit_range_diffs.clear();
    load_diff_at(app, i);
}

/// Extend the commit selection range downward (Shift+Down).
pub fn select_commit_down(app: &mut App) {
    let len = active_commits_len(app);
    if len == 0 {
        return;
    }
    let current = app.tab().commit_list_state.selected().unwrap_or(0);
    if current + 1 >= len {
        return;
    }
    let new_idx = current + 1;

    let anchor = app
        .tab()
        .anchor_commit_index
        .or(app.tab().commit_list_state.selected())
        .unwrap_or(new_idx);

    let (start, end) = if anchor <= new_idx {
        (anchor, new_idx)
    } else {
        (new_idx, anchor)
    };
    let range: Vec<usize> = (start..=end).collect();

    app.tab_mut().commit_list_state.select(Some(new_idx));
    app.tab_mut().selected_commits = range;
    let count = app.tab().selected_commits.len();
    app.tab_mut().status_message = Some(format!("{count} commits selected"));
    // Trigger the combined diff load for the selected range
    app.load_commit_range_diff();
}

/// Extend the commit selection range upward (Shift+Up).
pub fn select_commit_up(app: &mut App) {
    let len = active_commits_len(app);
    if len == 0 {
        return;
    }
    let current = app.tab().commit_list_state.selected().unwrap_or(0);
    if current == 0 {
        return;
    }
    let new_idx = current - 1;

    let anchor = app
        .tab()
        .anchor_commit_index
        .or(app.tab().commit_list_state.selected())
        .unwrap_or(new_idx);

    let (start, end) = if anchor <= new_idx {
        (anchor, new_idx)
    } else {
        (new_idx, anchor)
    };
    let range: Vec<usize> = (start..=end).collect();

    app.tab_mut().commit_list_state.select(Some(new_idx));
    app.tab_mut().selected_commits = range;
    let count = app.tab().selected_commits.len();
    app.tab_mut().status_message = Some(format!("{count} commits selected"));
    // Trigger the combined diff load for the selected range
    app.load_commit_range_diff();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_commits(count: usize) -> Vec<gitkraft_core::CommitInfo> {
        (0..count)
            .map(|i| gitkraft_core::CommitInfo {
                oid: format!("{i:040x}"),
                short_oid: format!("{i:07x}"),
                summary: format!("commit {i}"),
                message: format!("commit {i}"),
                author_name: "Test".into(),
                author_email: "test@test.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn space_toggles_commit_into_selection() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(2));

        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert!(app.tab().selected_commits.contains(&2));
    }

    #[test]
    fn space_deselects_already_selected_commit() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(2));
        app.tab_mut().selected_commits = vec![2];

        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert!(!app.tab().selected_commits.contains(&2));
    }

    #[test]
    fn space_advances_cursor() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(1));

        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert_eq!(app.tab().commit_list_state.selected(), Some(2));
    }

    #[test]
    fn space_does_not_advance_past_last_commit() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(3);
        app.tab_mut().commit_list_state.select(Some(2));

        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert_eq!(app.tab().commit_list_state.selected(), Some(2));
    }

    #[test]
    fn space_sets_status_message_with_count() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(0));

        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert_eq!(
            app.tab().status_message.as_deref(),
            Some("1 commit(s) selected")
        );

        app.tab_mut().commit_list_state.select(Some(1));
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert_eq!(
            app.tab().status_message.as_deref(),
            Some("2 commit(s) selected")
        );
    }

    #[test]
    fn space_clears_status_when_all_deselected() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(0));
        app.tab_mut().selected_commits = vec![0];

        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert!(app.tab().status_message.is_none());
    }

    #[test]
    fn space_allows_non_contiguous_selection() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);

        app.tab_mut().commit_list_state.select(Some(0));
        handle_key(&mut app, key(KeyCode::Char(' ')));
        app.tab_mut().commit_list_state.select(Some(2));
        handle_key(&mut app, key(KeyCode::Char(' ')));
        app.tab_mut().commit_list_state.select(Some(4));
        handle_key(&mut app, key(KeyCode::Char(' ')));

        assert_eq!(app.tab().selected_commits, vec![0, 2, 4]);
    }

    use crate::app::App;

    fn make_commits_simple(count: usize) -> Vec<gitkraft_core::CommitInfo> {
        (0..count)
            .map(|_| gitkraft_core::CommitInfo {
                oid: String::new(),
                short_oid: String::new(),
                summary: String::new(),
                message: String::new(),
                author_name: String::new(),
                author_email: String::new(),
                time: Default::default(),
                parent_ids: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn select_commit_down_creates_range() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits_simple(5);
        app.tab_mut().commit_list_state.select(Some(1));
        app.tab_mut().anchor_commit_index = Some(1);

        select_commit_down(&mut app);

        assert_eq!(app.tab().selected_commits, vec![1, 2]);
        assert_eq!(app.tab().commit_list_state.selected(), Some(2));
    }

    #[test]
    fn select_commit_up_creates_range() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits_simple(5);
        app.tab_mut().commit_list_state.select(Some(3));
        app.tab_mut().anchor_commit_index = Some(3);

        select_commit_up(&mut app);

        assert_eq!(app.tab().selected_commits, vec![2, 3]);
        assert_eq!(app.tab().commit_list_state.selected(), Some(2));
    }

    #[test]
    fn select_commit_down_stops_at_last() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits_simple(3);
        app.tab_mut().commit_list_state.select(Some(2));
        app.tab_mut().anchor_commit_index = Some(2);

        select_commit_down(&mut app); // already at last

        assert_eq!(app.tab().commit_list_state.selected(), Some(2));
        assert!(app.tab().selected_commits.is_empty());
    }

    #[test]
    fn navigate_down_clears_selected_commits() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(3);
        app.tab_mut().selected_commits = vec![0, 1];
        app.tab_mut().commit_list_state.select(Some(0));

        navigate_down(&mut app);

        assert!(app.tab().selected_commits.is_empty());
    }

    // ── Popup helpers ─────────────────────────────────────────────────────

    #[test]
    fn popup_is_open_false_when_no_oid() {
        let app = App::new();
        assert!(!popup_is_open(&app));
    }

    #[test]
    fn popup_is_open_true_when_oid_set() {
        let mut app = App::new();
        app.tab_mut().pending_commit_action_oid = Some("abc123".to_string());
        assert!(popup_is_open(&app));
    }

    // ── handle_popup_key ─────────────────────────────────────────────────

    fn app_with_popup() -> App {
        let mut app = App::new();
        // Give it a fake commit so open_commit_action_popup works
        app.tab_mut().commits = make_commits(3);
        app.tab_mut().commits[0].oid = "abc1234567".to_string();
        app.tab_mut().commit_list_state.select(Some(0));
        app.open_commit_action_popup();
        app
    }

    #[test]
    fn handle_popup_key_j_moves_cursor_down() {
        let mut app = app_with_popup();
        assert_eq!(app.tab().commit_action_cursor, 0);
        handle_popup_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().commit_action_cursor, 1);
    }

    #[test]
    fn handle_popup_key_down_arrow_moves_cursor_down() {
        let mut app = app_with_popup();
        handle_popup_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.tab().commit_action_cursor, 1);
    }

    #[test]
    fn handle_popup_key_k_moves_cursor_up() {
        let mut app = app_with_popup();
        app.tab_mut().commit_action_cursor = 3;
        handle_popup_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().commit_action_cursor, 2);
    }

    #[test]
    fn handle_popup_key_cursor_clamps_at_bottom() {
        let mut app = app_with_popup();
        let last = app.tab().commit_action_items.len() - 1;
        app.tab_mut().commit_action_cursor = last;
        handle_popup_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().commit_action_cursor, last);
    }

    #[test]
    fn handle_popup_key_cursor_clamps_at_top() {
        let mut app = app_with_popup();
        app.tab_mut().commit_action_cursor = 0;
        handle_popup_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().commit_action_cursor, 0);
    }

    #[test]
    fn handle_popup_key_esc_closes_popup() {
        let mut app = app_with_popup();
        assert!(popup_is_open(&app));
        handle_popup_key(&mut app, key(KeyCode::Esc));
        assert!(!popup_is_open(&app));
        assert!(app.tab().commit_action_items.is_empty());
        assert_eq!(app.tab().commit_action_cursor, 0);
    }

    #[test]
    fn handle_popup_key_q_closes_popup() {
        let mut app = app_with_popup();
        handle_popup_key(&mut app, key(KeyCode::Char('q')));
        assert!(!popup_is_open(&app));
    }

    #[test]
    fn handle_popup_key_enter_simple_action_sets_loading() {
        let mut app = app_with_popup();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        // Cursor 0 = CheckoutDetached — no input needed
        app.tab_mut().commit_action_cursor = 0;
        assert_eq!(
            app.tab().commit_action_items[0],
            gitkraft_core::CommitActionKind::CheckoutDetached
        );
        handle_popup_key(&mut app, key(KeyCode::Enter));
        // Should have dispatched to execute_commit_action → is_loading = true
        assert!(app.tab().is_loading);
        // Popup should be closed
        assert!(!popup_is_open(&app));
    }

    #[test]
    fn handle_popup_key_enter_input_action_enters_input_mode() {
        let mut app = app_with_popup();
        // Cursor 1 = CreateBranchHere — needs input
        app.tab_mut().commit_action_cursor = 1;
        assert_eq!(
            app.tab().commit_action_items[1],
            gitkraft_core::CommitActionKind::CreateBranchHere
        );
        handle_popup_key(&mut app, key(KeyCode::Enter));
        // Should enter input mode, NOT execute
        assert!(!app.tab().is_loading);
        assert_eq!(app.input_mode, crate::app::InputMode::Input);
        assert_eq!(
            app.input_purpose,
            crate::app::InputPurpose::CommitActionInput1
        );
        // pending_action_kind should be set
        assert_eq!(
            app.tab().pending_action_kind,
            Some(gitkraft_core::CommitActionKind::CreateBranchHere)
        );
        // The items list is cleared but the OID is kept for execute_commit_action
        assert!(app.tab().commit_action_items.is_empty());
        assert!(app.tab().pending_commit_action_oid.is_some());
    }

    #[test]
    fn navigate_down_clears_commit_range_diffs() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(3);
        app.tab_mut().commit_range_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];
        app.tab_mut().commit_list_state.select(Some(0));
        navigate_down(&mut app);
        assert!(app.tab().commit_range_diffs.is_empty());
    }

    #[test]
    fn select_commit_down_triggers_range_diff_load() {
        let mut app = App::new();
        app.tab_mut().commits = make_commits(5);
        app.tab_mut().commit_list_state.select(Some(1));
        app.tab_mut().anchor_commit_index = Some(1);
        // No repo_path, so load will be a no-op — just verify selected_commits is set
        select_commit_down(&mut app);
        assert_eq!(app.tab().selected_commits, vec![1, 2]);
    }
}
