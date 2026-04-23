use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, InputMode, InputPurpose, StagingFocus};

/// Handle keys when the Staging pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate within the currently focused sub-list
        KeyCode::Char('j') => {
            navigate_down(app);
        }
        KeyCode::Char('k') => {
            navigate_up(app);
        }

        // Toggle selection on current file
        KeyCode::Char(' ') => {
            let tab = app.tab_mut();
            match tab.staging_focus {
                StagingFocus::Unstaged => {
                    if let Some(idx) = tab.unstaged_list_state.selected() {
                        if tab.selected_unstaged.contains(&idx) {
                            tab.selected_unstaged.remove(&idx);
                        } else {
                            tab.selected_unstaged.insert(idx);
                        }
                        // Auto-advance to next item
                        if idx + 1 < tab.unstaged_changes.len() {
                            tab.unstaged_list_state.select(Some(idx + 1));
                        }
                    }
                }
                StagingFocus::Staged => {
                    if let Some(idx) = tab.staged_list_state.selected() {
                        if tab.selected_staged.contains(&idx) {
                            tab.selected_staged.remove(&idx);
                        } else {
                            tab.selected_staged.insert(idx);
                        }
                        if idx + 1 < tab.staged_changes.len() {
                            tab.staged_list_state.select(Some(idx + 1));
                        }
                    }
                }
            }
        }

        // Toggle focus between unstaged and staged sub-lists
        KeyCode::Tab => {
            let tab = app.tab_mut();
            tab.staging_focus = match tab.staging_focus {
                StagingFocus::Unstaged => StagingFocus::Staged,
                StagingFocus::Staged => StagingFocus::Unstaged,
            };
        }

        // Stage selected file(s)
        KeyCode::Char('s') => {
            let selected = app.tab().selected_unstaged.clone();
            if !selected.is_empty() {
                // Stage all selected files
                let paths: Vec<String> = selected
                    .iter()
                    .filter_map(|&idx| app.tab().unstaged_changes.get(idx))
                    .map(|d| d.display_path().to_string())
                    .collect();
                app.tab_mut().selected_unstaged.clear();
                app.stage_files(paths);
            } else {
                app.stage_selected();
            }
        }

        // Unstage selected file(s)
        KeyCode::Char('u') => {
            let selected = app.tab().selected_staged.clone();
            if !selected.is_empty() {
                let paths: Vec<String> = selected
                    .iter()
                    .filter_map(|&idx| app.tab().staged_changes.get(idx))
                    .map(|d| d.display_path().to_string())
                    .collect();
                app.tab_mut().selected_staged.clear();
                app.unstage_files(paths);
            } else {
                app.unstage_selected();
            }
        }

        // Stage all
        KeyCode::Char('S') => {
            app.stage_all();
        }

        // Unstage all
        KeyCode::Char('U') => {
            app.unstage_all();
        }

        // Discard changes (with confirmation, or batch for multi-select)
        KeyCode::Char('d') => {
            let selected = app.tab().selected_unstaged.clone();
            if !selected.is_empty() {
                // Discard all selected files directly (no confirmation for batch)
                let paths: Vec<String> = selected
                    .iter()
                    .filter_map(|&idx| app.tab().unstaged_changes.get(idx))
                    .map(|d| d.display_path().to_string())
                    .collect();
                app.tab_mut().selected_unstaged.clear();
                app.discard_files(paths);
            } else if app.tab().confirm_discard {
                app.discard_selected();
            } else {
                app.tab_mut().confirm_discard = true;
                app.tab_mut().status_message =
                    Some("Press 'd' again to confirm discard, or any other key to cancel".into());
            }
        }

        // Commit – enter input mode for commit message
        KeyCode::Char('c') => {
            app.tab_mut().confirm_discard = false;
            app.input_buffer.clear();
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::CommitMessage;
            app.tab_mut().status_message = Some("Enter commit message:".into());
        }

        // View diff of selected file
        KeyCode::Enter => {
            app.tab_mut().confirm_discard = false;
            app.load_staging_diff();
            // Switch to diff pane so the user can see the loaded diff
            if app.tab().selected_diff.is_some() {
                app.active_pane = crate::app::ActivePane::DiffView;
            }
        }

        // Open selected file in the configured editor
        KeyCode::Char('e') => {
            app.tab_mut().confirm_discard = false;
            app.open_selected_in_editor();
        }

        // Stash save
        KeyCode::Char('z') => {
            app.tab_mut().confirm_discard = false;
            app.tab_mut().stash_message_buffer.clear();
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::StashMessage;
            app.tab_mut().status_message = Some("Enter stash message (or leave empty):".into());
        }

        // Stash pop
        KeyCode::Char('Z') => {
            app.tab_mut().confirm_discard = false;
            app.stash_pop_selected();
        }

        // Any other key cancels the discard confirmation
        _ => {
            if app.tab().confirm_discard {
                app.tab_mut().confirm_discard = false;
                app.tab_mut().status_message = Some("Discard cancelled".into());
            }
        }
    }
}

/// Move selection down in the currently focused sub-list.
pub fn navigate_down(app: &mut App) {
    app.tab_mut().confirm_discard = false;
    let tab = app.tab_mut();
    match tab.staging_focus {
        StagingFocus::Unstaged => {
            if tab.unstaged_changes.is_empty() {
                return;
            }
            let i = match tab.unstaged_list_state.selected() {
                Some(i) => {
                    if i >= tab.unstaged_changes.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            tab.unstaged_list_state.select(Some(i));
        }
        StagingFocus::Staged => {
            if tab.staged_changes.is_empty() {
                return;
            }
            let i = match tab.staged_list_state.selected() {
                Some(i) => {
                    if i >= tab.staged_changes.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            tab.staged_list_state.select(Some(i));
        }
    }
}

/// Move selection up in the currently focused sub-list.
pub fn navigate_up(app: &mut App) {
    app.tab_mut().confirm_discard = false;
    let tab = app.tab_mut();
    match tab.staging_focus {
        StagingFocus::Unstaged => {
            if tab.unstaged_changes.is_empty() {
                return;
            }
            let i = match tab.unstaged_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        tab.unstaged_changes.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            tab.unstaged_list_state.select(Some(i));
        }
        StagingFocus::Staged => {
            if tab.staged_changes.is_empty() {
                return;
            }
            let i = match tab.staged_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        tab.staged_changes.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            tab.staged_list_state.select(Some(i));
        }
    }
}
