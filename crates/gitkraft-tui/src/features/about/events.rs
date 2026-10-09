use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle key events when the About panel is visible.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('A') | KeyCode::Char('q') => {
            app.show_about_panel = false;
        }
        KeyCode::Char('g') => open_url(app, gitkraft_core::about::AUTHOR_URL),
        KeyCode::Char('p') => open_url(app, gitkraft_core::about::REPO_URL),
        _ => {}
    }
}

fn open_url(app: &mut App, url: &str) {
    if let Err(e) = gitkraft_core::open_file_default(std::path::Path::new(url)) {
        app.tab_mut().error_message = Some(format!("Could not open {url}: {e}"));
    }
}
