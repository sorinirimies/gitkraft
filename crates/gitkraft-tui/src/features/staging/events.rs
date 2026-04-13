use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, InputMode, InputPurpose, StagingFocus};

/// Handle keys when the Staging pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate within the currently focused sub-list
        KeyCode::Char('j') | KeyCode::Down => {
            next_file(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            prev_file(app);
        }

        // Toggle focus between unstaged and staged sub-lists
        KeyCode::Tab => {
            app.staging_focus = match app.staging_focus {
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
            if app.confirm_discard {
                app.discard_selected();
            } else {
                app.confirm_discard = true;
                app.status_message =
                    Some("Press 'd' again to confirm discard, or any other key to cancel".into());
            }
        }

        // Commit — enter input mode for commit message
        KeyCode::Char('c') => {
            app.confirm_discard = false;
            app.input_buffer.clear();
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::CommitMessage;
            app.status_message = Some("Enter commit message:".into());
        }

        // View diff of selected file
        KeyCode::Enter => {
            app.confirm_discard = false;
            app.load_staging_diff();
        }

        // Stash save
        KeyCode::Char('z') => {
            app.confirm_discard = false;
            app.stash_message_buffer.clear();
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::StashMessage;
            app.status_message = Some("Enter stash message (or leave empty):".into());
        }

        // Stash pop
        KeyCode::Char('Z') => {
            app.confirm_discard = false;
            app.stash_pop_selected();
        }

        // Any other key cancels the discard confirmation
        _ => {
            if app.confirm_discard {
                app.confirm_discard = false;
                app.status_message = Some("Discard cancelled".into());
            }
        }
    }
}

/// Move selection down in the currently focused sub-list.
fn next_file(app: &mut App) {
    app.confirm_discard = false;
    match app.staging_focus {
        StagingFocus::Unstaged => {
            if app.unstaged_changes.is_empty() {
                return;
            }
            let i = match app.unstaged_list_state.selected() {
                Some(i) => {
                    if i >= app.unstaged_changes.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.unstaged_list_state.select(Some(i));
        }
        StagingFocus::Staged => {
            if app.staged_changes.is_empty() {
                return;
            }
            let i = match app.staged_list_state.selected() {
                Some(i) => {
                    if i >= app.staged_changes.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.staged_list_state.select(Some(i));
        }
    }
}

/// Move selection up in the currently focused sub-list.
fn prev_file(app: &mut App) {
    app.confirm_discard = false;
    match app.staging_focus {
        StagingFocus::Unstaged => {
            if app.unstaged_changes.is_empty() {
                return;
            }
            let i = match app.unstaged_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.unstaged_changes.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.unstaged_list_state.select(Some(i));
        }
        StagingFocus::Staged => {
            if app.staged_changes.is_empty() {
                return;
            }
            let i = match app.staged_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.staged_changes.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.staged_list_state.select(Some(i));
        }
    }
}
