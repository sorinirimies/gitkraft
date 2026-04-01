use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when the CommitLog pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            next_commit(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            prev_commit(app);
        }
        KeyCode::Enter => {
            app.load_commit_diff();
        }
        KeyCode::Char('g') => {
            // Jump to first commit
            if !app.commits.is_empty() {
                app.commit_list_state.select(Some(0));
            }
        }
        KeyCode::Char('G') => {
            // Jump to last commit
            if !app.commits.is_empty() {
                app.commit_list_state
                    .select(Some(app.commits.len() - 1));
            }
        }
        KeyCode::Esc => {
            app.commit_list_state.select(None);
        }
        _ => {}
    }
}

fn next_commit(app: &mut App) {
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

fn prev_commit(app: &mut App) {
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
