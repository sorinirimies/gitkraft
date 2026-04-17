use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, InputMode, InputPurpose};

/// Handle keys when the Branches pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down if !app.branches.is_empty() => {
            let i = match app.branch_list_state.selected() {
                Some(i) => {
                    if i >= app.branches.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.branch_list_state.select(Some(i));
        }
        KeyCode::Char('k') | KeyCode::Up if !app.branches.is_empty() => {
            let i = match app.branch_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.branches.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.branch_list_state.select(Some(i));
        }

        // Checkout selected branch
        KeyCode::Enter => {
            app.checkout_selected_branch();
        }

        // Create new branch (enter input mode)
        KeyCode::Char('b') => {
            app.input_buffer.clear();
            app.input_mode = InputMode::Input;
            app.input_purpose = InputPurpose::BranchName;
            app.status_message = Some("Enter new branch name:".into());
        }

        // Delete selected branch
        KeyCode::Char('D') => {
            app.delete_selected_branch();
        }

        // Deselect
        KeyCode::Esc => {
            app.branch_list_state.select(None);
        }

        _ => {}
    }
}
