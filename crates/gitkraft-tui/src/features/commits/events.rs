use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

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
    load_diff_at(app, i);
}
