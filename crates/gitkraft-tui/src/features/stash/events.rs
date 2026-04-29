use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when the Stash pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate down
        KeyCode::Char('j') | KeyCode::Down => {
            if app.tab().stashes.is_empty() {
                return;
            }
            let len = app.tab().stashes.len();
            let i = match app.tab().stash_list_state.selected() {
                Some(i) => (i + 1).min(len - 1),
                None => 0,
            };
            app.tab_mut().stash_list_state.select(Some(i));
        }
        // Navigate up
        KeyCode::Char('k') | KeyCode::Up => {
            if app.tab().stashes.is_empty() {
                return;
            }
            let i = match app.tab().stash_list_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            app.tab_mut().stash_list_state.select(Some(i));
        }
        // Pop selected stash (apply + drop)
        KeyCode::Enter | KeyCode::Char('p') => {
            app.stash_pop_selected();
        }
        // Drop selected stash (delete without applying)
        KeyCode::Char('d') => {
            app.stash_drop_selected();
        }
        // Deselect
        KeyCode::Esc => {
            app.tab_mut().stash_list_state.select(None);
        }
        _ => {}
    }
}
