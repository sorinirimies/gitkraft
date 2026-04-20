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
        KeyCode::Char('g') if !app.tab().commits.is_empty() => {
            // Jump to first commit
            app.tab_mut().commit_list_state.select(Some(0));
        }
        KeyCode::Char('G') if !app.tab().commits.is_empty() => {
            // Jump to last commit
            let last = app.tab().commits.len() - 1;
            app.tab_mut().commit_list_state.select(Some(last));
        }
        KeyCode::Esc => {
            app.tab_mut().commit_list_state.select(None);
        }
        _ => {}
    }
}

/// Move commit selection down by one.
pub fn navigate_down(app: &mut App) {
    if app.tab().commits.is_empty() {
        return;
    }
    let tab = app.tab_mut();
    let i = match tab.commit_list_state.selected() {
        Some(i) => {
            if i >= tab.commits.len() - 1 {
                i
            } else {
                i + 1
            }
        }
        None => 0,
    };
    tab.commit_list_state.select(Some(i));
}

/// Move commit selection up by one.
pub fn navigate_up(app: &mut App) {
    if app.tab().commits.is_empty() {
        return;
    }
    let tab = app.tab_mut();
    let i = match tab.commit_list_state.selected() {
        Some(i) => {
            if i == 0 {
                0
            } else {
                i - 1
            }
        }
        None => 0,
    };
    tab.commit_list_state.select(Some(i));
}
