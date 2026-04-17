use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

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
            app.load_commit_diff();
        }
        KeyCode::Char('g') if !app.commits.is_empty() => {
            // Jump to first commit
            app.commit_list_state.select(Some(0));
        }
        KeyCode::Char('G') if !app.commits.is_empty() => {
            // Jump to last commit
            app.commit_list_state.select(Some(app.commits.len() - 1));
        }
        KeyCode::Esc => {
            app.commit_list_state.select(None);
        }
        _ => {}
    }
}

/// Move commit selection down by one.
pub fn navigate_down(app: &mut App) {
    if app.commits.is_empty() {
        return;
    }
    let i = match app.commit_list_state.selected() {
        Some(i) => {
            if i >= app.commits.len() - 1 {
                i
            } else {
                i + 1
            }
        }
        None => 0,
    };
    app.commit_list_state.select(Some(i));
}

/// Move commit selection up by one.
pub fn navigate_up(app: &mut App) {
    if app.commits.is_empty() {
        return;
    }
    let i = match app.commit_list_state.selected() {
        Some(i) => {
            if i == 0 {
                0
            } else {
                i - 1
            }
        }
        None => 0,
    };
    app.commit_list_state.select(Some(i));
}
