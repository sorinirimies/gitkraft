use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle key events when the theme selection panel is visible.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('t') | KeyCode::Down | KeyCode::Char('j') => {
            // Next theme
            app.cycle_theme_next();
        }
        KeyCode::Char('T') | KeyCode::Up | KeyCode::Char('k') => {
            // Previous theme
            app.cycle_theme_prev();
        }
        KeyCode::Esc => {
            app.show_theme_panel = false;
        }
        KeyCode::Enter => {
            // Confirm and close
            app.show_theme_panel = false;
        }
        _ => {}
    }
}
