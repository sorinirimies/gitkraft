use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when the Remotes section is visible.
///
/// Remotes are displayed in the sidebar and don't have their own dedicated
/// active pane, so this handler is intentionally minimal. The global `f` key
/// already triggers `app.fetch_remote()` from the main event dispatcher.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char('f') = key.code {
        app.fetch_remote();
    }
}
