use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle key events when the options panel is visible.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('O') => {
            app.show_options_panel = false;
        }
        KeyCode::Char('T') | KeyCode::Char('t') => {
            // Switch to theme panel
            app.show_options_panel = false;
            app.show_theme_panel = true;
        }
        // Open settings file in editor (same shortcut shown in this panel)
        KeyCode::Char(',') => {
            app.show_options_panel = false;
            app.open_settings_in_editor();
        }
        _ => {}
    }
}
