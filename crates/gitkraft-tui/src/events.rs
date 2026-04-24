use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActivePane, App, AppScreen, DiffSubPane, InputMode, InputPurpose};
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
                                    // If current tab already has a repo, open in a new tab
                                    if app.tab().repo_path.is_some() {
                                        app.new_tab();
                                    }
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
                            // If current tab already has a repo, open in a new tab
                            if app.tab().repo_path.is_some() {
                                app.new_tab();
                            }
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

            if app.show_editor_panel {
                features::editor::events::handle_key(app, key);
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
                KeyCode::Char('p') => app.pull_rebase(),
                KeyCode::Char('P') => app.push_branch(),
                KeyCode::Char('T') => {
                    app.show_theme_panel = !app.show_theme_panel;
                    app.show_options_panel = false; // close options if open
                    app.show_editor_panel = false;
                }
                KeyCode::Char('O') => {
                    app.show_options_panel = !app.show_options_panel;
                    app.show_theme_panel = false; // close theme panel if open
                    app.show_editor_panel = false;
                }
                KeyCode::Char('E') => {
                    app.show_editor_panel = !app.show_editor_panel;
                    app.show_theme_panel = false;
                    app.show_options_panel = false;
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
                // Arrow left/right: switch panes (or navigate diff sub-panes)
                KeyCode::Left => {
                    if app.active_pane == ActivePane::DiffView
                        && app.tab().diff_sub_pane == DiffSubPane::Content
                    {
                        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
                    } else {
                        cycle_pane_backward(app);
                    }
                }
                KeyCode::Right => {
                    if app.active_pane == ActivePane::DiffView
                        && app.tab().diff_sub_pane == DiffSubPane::FileList
                        && !app.tab().commit_files.is_empty()
                    {
                        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
                    } else {
                        cycle_pane_forward(app);
                    }
                }

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
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            features::commits::events::select_commit_up(app);
                        } else {
                            features::commits::events::navigate_up(app);
                        }
                    }
                    ActivePane::DiffView => match app.tab().diff_sub_pane {
                        DiffSubPane::FileList => {
                            if !app.tab().commit_files.is_empty() {
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    features::diff::events::select_file_up(app);
                                } else {
                                    features::diff::events::navigate_file_up(app);
                                }
                            } else {
                                let tab = app.tab_mut();
                                tab.diff_scroll = tab.diff_scroll.saturating_sub(1);
                            }
                        }
                        DiffSubPane::Content => {
                            let tab = app.tab_mut();
                            tab.diff_scroll = tab.diff_scroll.saturating_sub(1);
                        }
                    },
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
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            features::commits::events::select_commit_down(app);
                        } else {
                            features::commits::events::navigate_down(app);
                        }
                    }
                    ActivePane::DiffView => match app.tab().diff_sub_pane {
                        DiffSubPane::FileList => {
                            if !app.tab().commit_files.is_empty() {
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    features::diff::events::select_file_down(app);
                                } else {
                                    features::diff::events::navigate_file_down(app);
                                }
                            } else {
                                let tab = app.tab_mut();
                                tab.diff_scroll = tab.diff_scroll.saturating_add(1);
                            }
                        }
                        DiffSubPane::Content => {
                            let tab = app.tab_mut();
                            tab.diff_scroll = tab.diff_scroll.saturating_add(1);
                        }
                    },
                    ActivePane::Staging => {
                        features::staging::events::navigate_down(app);
                    }
                },

                KeyCode::Char('a') if app.active_pane == ActivePane::CommitLog => {
                    app.open_commit_action_popup();
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
        InputPurpose::CommitActionInput1 => {
            let value = app.input_buffer.trim().to_string();
            app.input_buffer.clear();
            if value.is_empty() {
                app.tab_mut().status_message = Some("Cancelled".into());
                app.tab_mut().pending_action_kind = None;
                app.tab_mut().pending_commit_action_oid = None;
                return;
            }
            let kind = match app.tab().pending_action_kind {
                Some(k) => k,
                None => return,
            };
            if kind.needs_second_input() {
                // Store first input, ask for second
                app.tab_mut().action_input1 = value;
                app.input_mode = crate::app::InputMode::Input;
                app.input_purpose = InputPurpose::CommitActionInput2;
                let prompt = kind.second_input_prompt().unwrap_or("Message:");
                app.tab_mut().status_message = Some(prompt.to_string());
            } else {
                // Build and execute the action
                let action = kind.into_action(value, String::new());
                app.execute_commit_action(action);
            }
        }
        InputPurpose::CommitActionInput2 => {
            let value2 = app.input_buffer.trim().to_string();
            app.input_buffer.clear();
            let kind = match app.tab().pending_action_kind {
                Some(k) => k,
                None => return,
            };
            let input1 = app.tab().action_input1.clone();
            let action = kind.into_action(input1, value2);
            app.execute_commit_action(action);
        }
        InputPurpose::None => {
            app.input_buffer.clear();
        }
    }
}

/// Cycle the active pane forward: Branches -> CommitLog -> DiffView -> Staging -> Branches
fn cycle_pane_forward(app: &mut App) {
    app.tab_mut().confirm_discard = false;
    if app.active_pane == ActivePane::DiffView {
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        app.tab_mut().selected_file_indices.clear();
    }
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
    if app.active_pane == ActivePane::DiffView {
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        app.tab_mut().selected_file_indices.clear();
    }
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

    #[test]
    fn o_on_main_opens_browser() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        handle_key(&mut app, key(KeyCode::Char('o')));
        assert_eq!(app.screen, AppScreen::DirBrowser);
    }

    #[test]
    fn space_in_staging_toggles_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::Staging;
        app.tab_mut()
            .unstaged_changes
            .push(gitkraft_core::DiffInfo {
                old_file: String::new(),
                new_file: "test.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
                hunks: Vec::new(),
            });
        app.tab_mut().unstaged_list_state.select(Some(0));
        app.tab_mut().staging_focus = crate::app::StagingFocus::Unstaged;

        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert!(app.tab().selected_unstaged.contains(&0));
    }

    #[test]
    fn p_triggers_pull() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.tabs[0].repo_path = Some(std::path::PathBuf::from("/tmp/fake"));
        handle_key(&mut app, key(KeyCode::Char('p')));
        // pull_rebase sets loading
        assert!(app.tab().is_loading);
    }

    #[test]
    fn shift_p_triggers_push() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.tabs[0].repo_path = Some(std::path::PathBuf::from("/tmp/fake"));
        // No head_branch → should set error
        handle_key(&mut app, key(KeyCode::Char('P')));
        assert!(app.tab().error_message.is_some());
    }

    // ── DiffView sub-pane helpers ─────────────────────────────────────────

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    fn populate_commit_files(app: &mut App, count: usize) {
        app.tab_mut().commit_files = (0..count)
            .map(|i| gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: format!("file{i}.rs"),
                status: gitkraft_core::FileStatus::Modified,
            })
            .collect();
    }

    // ── DiffView sub-pane: Right / Left arrow ────────────────────────────

    #[test]
    fn right_in_diffview_file_list_with_files_enters_content() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        populate_commit_files(&mut app, 2);
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::Content);
        assert_eq!(app.active_pane, ActivePane::DiffView);
    }

    #[test]
    fn right_in_diffview_file_list_without_files_cycles_pane_forward() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        // no commit_files
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.active_pane, ActivePane::Staging);
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
    }

    #[test]
    fn right_in_diffview_content_cycles_pane_forward() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.active_pane, ActivePane::Staging);
    }

    #[test]
    fn left_in_diffview_content_returns_to_file_list() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
        assert_eq!(app.active_pane, ActivePane::DiffView);
    }

    #[test]
    fn left_in_diffview_file_list_cycles_pane_backward() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.active_pane, ActivePane::CommitLog);
    }

    // ── DiffView sub-pane: Tab resets state ──────────────────────────────

    #[test]
    fn tab_from_diffview_resets_sub_pane_to_file_list() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Tab));
        assert_eq!(app.active_pane, ActivePane::Staging);
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
    }

    #[test]
    fn tab_from_diffview_clears_multi_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        handle_key(&mut app, key(KeyCode::Tab));
        assert!(app.tab().selected_file_indices.is_empty());
    }

    // ── DiffView sub-pane: Shift+Up/Down multi-selection ─────────────────

    #[test]
    fn shift_down_in_diffview_file_list_extends_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        populate_commit_files(&mut app, 3);
        app.tab_mut().commit_diff_file_index = 0;
        handle_key(&mut app, key_shift(KeyCode::Down));
        assert!(app.tab().selected_file_indices.contains(&0));
        assert!(app.tab().selected_file_indices.contains(&1));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn shift_up_in_diffview_file_list_extends_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        populate_commit_files(&mut app, 3);
        app.tab_mut().commit_diff_file_index = 2;
        handle_key(&mut app, key_shift(KeyCode::Up));
        assert!(app.tab().selected_file_indices.contains(&2));
        assert!(app.tab().selected_file_indices.contains(&1));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn unshifted_down_in_diffview_file_list_clears_multi_selection() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        populate_commit_files(&mut app, 3);
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        handle_key(&mut app, key(KeyCode::Down));
        // single-select clears multi: only one entry should remain
        assert_eq!(app.tab().selected_file_indices.len(), 1);
    }

    // ── DiffView Content sub-pane: Up/Down scrolls ───────────────────────

    #[test]
    fn down_in_diffview_content_scrolls() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 2;
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.tab().diff_scroll, 3);
    }

    #[test]
    fn up_in_diffview_content_scrolls() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.active_pane = ActivePane::DiffView;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 5;
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.tab().diff_scroll, 4);
    }
}
