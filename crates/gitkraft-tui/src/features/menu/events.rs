//! Keys while the menu is open.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};

use super::Row;
use crate::app::App;

/// Handles a key; the menu owns the keyboard while it is open.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    let now = Instant::now();
    match key.code {
        KeyCode::Esc | KeyCode::Char('q' | 'M') => app.close_menu(now),
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => app.menu.step(1),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => app.menu.step(-1),
        KeyCode::Left | KeyCode::Char('h') => change(app, -1, now),
        KeyCode::Right | KeyCode::Char('l') => change(app, 1, now),
        KeyCode::Char(' ') | KeyCode::Enter => change(app, 1, now),
        _ => {}
    }
}

/// Changes the selected row (`dir` is the direction for cycling rows) or runs its action.
fn change(app: &mut App, dir: isize, now: Instant) {
    match app.menu.selected() {
        Row::Theme => {
            app.cycle_theme(dir);
            app.save_theme();
        }
        Row::ReducedMotion => {
            app.reduced_motion = !app.reduced_motion;
            app.save_reduced_motion();
        }
        Row::CommitStyle => {
            let Some(repo) = app.tab().repo_path.clone() else {
                app.tab_mut().status_message =
                    Some("Open a repository first: the style is per project".into());
                return;
            };
            let file = repo.join(".gitkraft").join("ai.toml");
            if !file.exists()
                && let Err(e) = crate::ai_cli::init(&repo)
            {
                app.tab_mut().error_message = Some(format!("{e:#}"));
                return;
            }
            app.close_menu(now);
            app.open_path_in_editor(file, "commit message style");
        }
        Row::SettingsFile => {
            app.close_menu(now);
            app.open_settings_in_editor();
        }
        Row::Keys => {
            app.close_menu(now);
            app.show_options_panel = true;
            app.show_theme_panel = false;
            app.show_about_panel = false;
            app.show_editor_panel = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;

    fn press(app: &mut App, code: KeyCode) {
        handle_key(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn opened() -> App {
        let mut app = App::new();
        app.reduced_motion = true; // closing is instant, no sleeping in tests
        app.open_menu();
        app
    }

    #[test]
    fn keys_move_the_cursor_and_escape_closes() {
        let mut app = opened();
        assert!(app.menu.is_open());
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Down);
        assert_eq!(app.menu.selected(), Row::CommitStyle);
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.menu.selected(), Row::ReducedMotion);
        press(&mut app, KeyCode::Esc);
        assert!(!app.menu.is_open());
        app.menu.tick(Instant::now());
        assert!(!app.menu.is_shown());
    }

    #[test]
    fn the_theme_row_cycles_both_ways_and_reduced_motion_toggles() {
        let mut app = opened();
        let before = app.current_theme_name();
        press(&mut app, KeyCode::Right);
        let after = app.current_theme_name();
        assert_ne!(before, after);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.current_theme_name(), before);

        press(&mut app, KeyCode::Char('j'));
        let was = app.reduced_motion;
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.reduced_motion, !was);
    }

    #[test]
    fn the_keys_row_hands_over_to_the_reference_panel() {
        let mut app = opened();
        app.menu.step(-1);
        press(&mut app, KeyCode::Enter);
        assert!(app.show_options_panel);
        assert!(!app.menu.is_open());
    }

    #[test]
    fn the_commit_style_row_needs_a_repository() {
        let mut app = opened();
        app.menu.step(2);
        press(&mut app, KeyCode::Enter);
        assert!(app.menu.is_open(), "stays open to show why");
        assert!(
            app.tab()
                .status_message
                .clone()
                .unwrap_or_default()
                .contains("repository")
        );
    }

    #[test]
    fn the_global_key_opens_the_menu_and_it_takes_over_the_keyboard() {
        let mut app = App::new();
        app.screen = crate::app::AppScreen::Main;
        crate::events::handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('M'), KeyModifiers::NONE),
        );
        assert!(app.menu.is_open());
        crate::events::handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        );
        assert!(!app.should_quit, "q closes the menu, not the app");
    }
}
