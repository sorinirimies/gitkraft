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

        // Toggle focus between unstaged and staged sub-lists
        KeyCode::Tab => {
            let tab = app.tab_mut();
            tab.staging_focus = match tab.staging_focus {
                StagingFocus::Unstaged => StagingFocus::Staged,
                StagingFocus::Staged => StagingFocus::Unstaged,
            };
        }

        // Stage selected file
        KeyCode::Char('s') => {
            app.stage_selected();
        }

        // Unstage selected file
        KeyCode::Char('u') => {
            app.unstage_selected();
        }

        // Stage all
        KeyCode::Char('S') => {
            app.stage_all();
        }

        // Unstage all
        KeyCode::Char('U') => {
            app.unstage_all();
        }

        // Discard changes (with confirmation)
        KeyCode::Char('d') => {
            if app.tab().confirm_discard {
                app.discard_selected();
            } else {
                app.tab_mut().confirm_discard = true;
                app.tab_mut().status_message =
                    Some("Press 'd' again to confirm discard, or any other key to cancel".into());
            }
        }

        // Commit â enter input mode for commit message
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
