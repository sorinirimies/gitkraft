use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when stash-related actions are triggered.
///
/// Stash operations are primarily accessed from the staging pane via `z`/`Z`
/// shortcuts, but this handler covers any stash-specific interactions when the
/// stash section of the sidebar might be focused in the future.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Save current working directory to stash
        KeyCode::Char('s') | KeyCode::Char('z') => {
            app.stash_save();
        }
        // Pop the most recent stash
        KeyCode::Char('p') | KeyCode::Char('Z') => {
            app.stash_pop_selected();
        }
        // Drop the most recent stash
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.stash_drop_selected();
        }
        _ => {}
    }
}
