use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActivePane, App, AppScreen, InputMode, InputPurpose};
use crate::features;

/// Top-level key dispatch — called once per key event from the event loop.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Clear error/status on any keypress so stale messages don't linger
    // (but keep them visible for at least one frame — they were set last tick).
    if app.tab().error_message.is_some() {
        app.tab_mut().error_message = None;
    }

    // Input mode takes priority on ANY screen
    if app.input_mode == InputMode::Input {
        handle_input_key(app, key);
        return;
    }

    match app.screen {
        AppScreen::Welcome => features::repo::events::handle_key(app, key),
        AppScreen::DirBrowser => {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let len = app.browser_entries.len();
                    if len > 0 {
                        let i = app.browser_list_state.selected().unwrap_or(0);
                        let new = if i == 0 { len - 1 } else { i - 1 };
                        app.browser_list_state.select(Some(new));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let len = app.browser_entries.len();
                    if len > 0 {
                        let i = app.browser_list_state.selected().unwrap_or(0);
                        let new = (i + 1) % len;
                        app.browser_list_state.select(Some(new));
                    }
                }
                KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(idx) = app.browser_list_state.selected() {
                        if let Some(path) = app.browser_entries.get(idx).cloned() {
                            if path.is_dir() {
                                // Check if it is a git repo
                                if path.join(".git").exists() {
                                    app.screen = app.browser_return_screen.clone();
                                    app.open_repo(path);
                                } else {
                                    // Navigate into directory
                                    app.browser_dir = path;
                                    app.refresh_browser();
                                }
                            }
                        }
                    }
                }
                KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                    // Go up one directory
                    if let Some(parent) = app.browser_dir.parent().map(|p| p.to_path_buf()) {
                        app.browser_dir = parent;
                        app.refresh_browser();
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.screen = app.browser_return_screen.clone();
                }
                // Allow typing 'o' to open the selected directory even if it is not a git repo
                KeyCode::Char('o') => {
                    if let Some(idx) = app.browser_list_state.selected() {
                        if let Some(path) = app.browser_entries.get(idx).cloned() {
                            app.screen = app.browser_return_screen.clone();
                            app.open_repo(path);
                        }
                    }
                }
                _ => {}
            }
        }
        AppScreen::Main => {
            if app.show_theme_panel {
                features::theme::events::handle_key(app, key);
                return;
            }

            if app.show_options_panel {
                features::options::events::handle_key(app, key);
                return;
            }

            // -- Global keys (available in Normal mode on the Main screen) --
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
                KeyCode::Char('o') => {
                    let start = app
                        .tab()
                        .repo_path
                        .clone()
                        .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
                        .or_else(dirs::home_dir)
                        .unwrap_or_else(|| std::path::PathBuf::from("/"));
                    app.open_browser(start);
                }
                KeyCode::Char('W') => {
                    app.close_tab();
                }
                // Tab management
                KeyCode::Char('N') => {
                    app.new_tab();
                }
                KeyCode::Char(']') => {
                    app.next_tab();
                }
                KeyCode::Char('[') => {
                    app.prev_tab();
                }
                KeyCode::Char('/') => {
                    app.input_mode = InputMode::Input;
                    app.input_purpose = InputPurpose::SearchQuery;
                    app.input_buffer.clear();
                    app.tab_mut().status_message = Some("Search commits:".into());
                }
                // Arrow left/right: switch panes
                KeyCode::Left => cycle_pane_backward(app),
                KeyCode::Right => cycle_pane_forward(app),

                // Arrow up/down: navigate within the active pane
                KeyCode::Up => match app.active_pane {
                    ActivePane::Branches => {
                        let tab = app.tab_mut();
                        if !tab.branches.is_empty() {
                            let len = tab.branches.len();
                            let i = tab.branch_list_state.selected().unwrap_or(0);
                            let new = if i == 0 { len - 1 } else { i - 1 };
                            tab.branch_list_state.select(Some(new));
                        }
                    }
                    ActivePane::CommitLog => {
                        features::commits::events::navigate_up(app);
                    }
                    ActivePane::DiffView => {
                        if app.tab().commit_files.len() > 1 {
                            app.prev_diff_file();
                        } else {
                            let tab = app.tab_mut();
                            tab.diff_scroll = tab.diff_scroll.saturating_sub(1);
                        }
                    }
                    ActivePane::Staging => {
                        features::staging::events::navigate_up(app);
                    }
                },
                KeyCode::Down => match app.active_pane {
                    ActivePane::Branches => {
                        let tab = app.tab_mut();
                        if !tab.branches.is_empty() {
                            let len = tab.branches.len();
                            let i = tab.branch_list_state.selected().unwrap_or(0);
                            let new = (i + 1) % len;
                            tab.branch_list_state.select(Some(new));
                        }
                    }
                    ActivePane::CommitLog => {
                        features::commits::events::navigate_down(app);
                    }
                    ActivePane::DiffView => {
                        if app.tab().commit_files.len() > 1 {
                            app.next_diff_file();
                        } else {
                            let tab = app.tab_mut();
                            tab.diff_scroll = tab.diff_scroll.saturating_add(1);
                        }
                    }
                    ActivePane::Staging => {
                        features::staging::events::navigate_down(app);
                    }
                },

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
            app.tab_mut().status_message = Some("Input cancelled".into());
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
            let query = app.input_buffer.clone();
            app.input_buffer.clear();
            app.tab_mut().search_active = true;
            app.search_commits(query);
        }
        InputPurpose::StashMessage => {
            app.tab_mut().stash_message_buffer = app.input_buffer.clone();
            app.input_buffer.clear();
            app.stash_save();
        }
        InputPurpose::None => {
            app.input_buffer.clear();
        }
    }
}

/// Cycle the active pane forward: Branches -> CommitLog -> DiffView -> Staging -> Branches
fn cycle_pane_forward(app: &mut App) {
    app.tab_mut().confirm_discard = false;
    app.active_pane = match app.active_pane {
        ActivePane::Branches => ActivePane::CommitLog,
        ActivePane::CommitLog => ActivePane::DiffView,
        ActivePane::DiffView => ActivePane::Staging,
        ActivePane::Staging => ActivePane::Branches,
    };
}

/// Cycle the active pane backward: Branches -> Staging -> DiffView -> CommitLog -> Branches
fn cycle_pane_backward(app: &mut App) {
    app.tab_mut().confirm_discard = false;
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
    fn o_on_welcome_opens_browser() {
        let mut app = App::new();
        handle_key(&mut app, key(KeyCode::Char('o')));
        assert_eq!(app.screen, AppScreen::DirBrowser);
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

    #[test]
    fn slash_enters_search_mode() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char('/')));
        assert_eq!(app.input_mode, InputMode::Input);
        assert_eq!(app.input_purpose, InputPurpose::SearchQuery);
    }

    #[test]
    fn bracket_right_switches_tab() {
        let mut app = App::new();
        app.new_tab();
        app.active_tab_index = 0; // go back to first
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char(']')));
        assert_eq!(app.active_tab_index, 1);
    }

    #[test]
    fn bracket_left_switches_tab() {
        let mut app = App::new();
        app.new_tab();
        // active = 1
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char('[')));
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn shift_n_creates_new_tab() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char('N')));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.screen, AppScreen::Welcome);
    }

    #[test]
    fn shift_w_closes_tab() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.new_tab();
        assert_eq!(app.tabs.len(), 2);
        app.screen = AppScreen::Main; // new_tab sets Welcome
        handle_key(&mut app, key(KeyCode::Char('W')));
        assert_eq!(app.tabs.len(), 1);
    }

    #[test]
    fn arrow_right_switches_pane() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        assert_eq!(app.active_pane, ActivePane::Branches);
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.active_pane, ActivePane::CommitLog);
    }

    #[test]
    fn arrow_left_switches_pane() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        assert_eq!(app.active_pane, ActivePane::Branches);
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.active_pane, ActivePane::Staging);
    }
}
