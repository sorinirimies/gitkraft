use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle key events when the theme selection panel is visible.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('t') | KeyCode::Down | KeyCode::Char('j') => {
            // Next theme
            app.cycle_theme_next();
            app.save_theme();
        }
        KeyCode::Char('T') | KeyCode::Up | KeyCode::Char('k') => {
            // Previous theme
            app.cycle_theme_prev();
            app.save_theme();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    #[test]
    fn navigation_moves_through_picker_and_esc_closes() {
        let mut app = App::new();
        app.show_theme_panel = true;
        app.select_picker(0);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.picker_index(), 1);
        assert!(!app.follow_system);
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert!(app.follow_system);
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(
            app.picker_index(),
            crate::features::theme::palette::PICKER_LEN - 1
        );
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(!app.show_theme_panel);
    }
}
