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

    match app.screen {
        AppScreen::Welcome => features::repo::events::handle_key(app, key),
        AppScreen::Main => {
            if app.input_mode == InputMode::Input {
                handle_input_key(app, key);
                return;
            }

            if app.show_theme_panel {
                features::theme::events::handle_key(app, key);
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
