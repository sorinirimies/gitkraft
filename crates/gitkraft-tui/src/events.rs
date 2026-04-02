use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActivePane, App, AppScreen, InputMode, InputPurpose};
use crate::features;

/// Top-level key dispatch — called once per key event from the event loop.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Clear error/status on any keypress so stale messages don't linger
    // (but keep them visible for at least one frame — they were set last tick).
    if app.error_message.is_some() {
        app.error_message = None;
    }

    // Input mode takes priority on ANY screen
    if app.input_mode == InputMode::Input {
        handle_input_key(app, key);
        return;
    }

    match app.screen {
        AppScreen::Welcome => features::repo::events::handle_key(app, key),
        AppScreen::Main => {
            if app.show_theme_panel {
                features::theme::events::handle_key(app, key);
                return;
            }

            if app.show_options_panel {
                features::options::events::handle_key(app, key);
                return;
            }

            // ── Global keys (available in Normal mode on the Main screen) ──
            match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true;
                }
                KeyCode::Tab => cycle_pane_forward(app),
                KeyCode::BackTab => cycle_pane_backward(app),
                KeyCode::Char('r') => app.refresh(),
                KeyCode::Char('f') => app.fetch_remote(),
                KeyCode::Char('T') => {
                    app.show_theme_panel = !app.show_theme_panel;
                    app.show_options_panel = false; // close options if open
                }
                KeyCode::Char('O') => {
                    app.show_options_panel = !app.show_options_panel;
                    app.show_theme_panel = false; // close theme panel if open
                }
                _ => {
                    // Delegate to the active pane's feature handler
                    match app.active_pane {
                        ActivePane::Branches => {
                            features::branches::events::handle_key(app, key);
                        }
                        ActivePane::CommitLog => {
                            features::commits::events::handle_key(app, key);
                        }
                        ActivePane::DiffView => {
                            features::diff::events::handle_key(app, key);
                        }
                        ActivePane::Staging => {
                            features::staging::events::handle_key(app, key);
                        }
                    }
                }
            }
        }
    }
}

/// Handle keys while in Input mode (typing into the input buffer).
fn handle_input_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Enter => {
            submit_input(app);
        }
        KeyCode::Esc => {
            // Cancel input
            app.input_buffer.clear();
            app.input_mode = InputMode::Normal;
            app.input_purpose = InputPurpose::None;
            app.status_message = Some("Input cancelled".into());
        }
        _ => {}
    }
}

/// Submit the current input buffer based on the active purpose.
fn submit_input(app: &mut App) {
    let purpose = app.input_purpose;
    app.input_mode = InputMode::Normal;
    app.input_purpose = InputPurpose::None;

    match purpose {
        InputPurpose::CommitMessage => {
            app.create_commit();
        }
        InputPurpose::BranchName => {
            app.create_branch();
        }
        InputPurpose::RepoPath => {
            let path = std::path::PathBuf::from(app.input_buffer.trim());
            app.input_buffer.clear();
            app.open_repo(path);
        }
        InputPurpose::SearchQuery => {
            // Search is not fully wired yet; clear buffer for now
            app.status_message = Some(format!("Search: {}", app.input_buffer));
            app.input_buffer.clear();
        }
        InputPurpose::None => {
            app.input_buffer.clear();
        }
    }
}

/// Cycle the active pane forward: Branches → CommitLog → DiffView → Staging → Branches
fn cycle_pane_forward(app: &mut App) {
    app.confirm_discard = false;
    app.active_pane = match app.active_pane {
        ActivePane::Branches => ActivePane::CommitLog,
        ActivePane::CommitLog => ActivePane::DiffView,
        ActivePane::DiffView => ActivePane::Staging,
        ActivePane::Staging => ActivePane::Branches,
    };
}

/// Cycle the active pane backward: Branches → Staging → DiffView → CommitLog → Branches
fn cycle_pane_backward(app: &mut App) {
    app.confirm_discard = false;
    app.active_pane = match app.active_pane {
        ActivePane::Branches => ActivePane::Staging,
        ActivePane::CommitLog => ActivePane::Branches,
        ActivePane::DiffView => ActivePane::CommitLog,
        ActivePane::Staging => ActivePane::DiffView,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn q_quits_on_main() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn tab_cycles_pane() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        assert_eq!(app.active_pane, ActivePane::Branches);
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.active_pane, ActivePane::CommitLog);
    }

    #[test]
    fn o_on_welcome_enters_input() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Char('o')));
        assert_eq!(app.input_mode, InputMode::Input);
        assert_eq!(app.input_purpose, InputPurpose::RepoPath);
    }

    #[test]
    fn input_mode_captures_chars() {
        let mut app = App::new();
        app.input_mode = InputMode::Input;
        app.input_purpose = InputPurpose::RepoPath;
        app.screen = AppScreen::Welcome;
        handle_key(&mut app, key(KeyCode::Char('/')));
        handle_key(&mut app, key(KeyCode::Char('t')));
        assert_eq!(app.input_buffer, "/t");
    }

    #[test]
    fn esc_cancels_input() {
        let mut app = App::new();
        app.input_mode = InputMode::Input;
        app.input_purpose = InputPurpose::RepoPath;
        app.input_buffer = "/tmp".to_string();
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
    }
}
