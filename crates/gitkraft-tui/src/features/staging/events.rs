use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode, InputPurpose, StagingFocus};

/// Handle keys when the Staging pane is active.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate within the currently focused sub-list (plain = move, Shift = extend selection)
        KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                select_down(app);
            } else {
                navigate_down(app);
            }
        }
        KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                select_up(app);
            } else {
                navigate_up(app);
            }
        }
        // Shift+J / Shift+K aliases for Shift+j / Shift+k (terminals send uppercase for Shift+letter)
        KeyCode::Char('J') => {
            select_down(app);
        }
        KeyCode::Char('K') => {
            select_up(app);
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

        // File history for the currently selected staging file
        KeyCode::Char('H') => {
            app.tab_mut().confirm_discard = false;
            let path = match app.tab().staging_focus {
                StagingFocus::Unstaged => app
                    .tab()
                    .unstaged_changes
                    .get(app.tab().unstaged_list_state.selected().unwrap_or(0))
                    .map(|d| d.display_path().to_string()),
                StagingFocus::Staged => app
                    .tab()
                    .staged_changes
                    .get(app.tab().staged_list_state.selected().unwrap_or(0))
                    .map(|d| d.display_path().to_string()),
            };
            if let Some(p) = path {
                app.open_file_history(p);
            }
        }

        // Blame for the currently selected staging file
        KeyCode::Char('B') => {
            app.tab_mut().confirm_discard = false;
            let path = match app.tab().staging_focus {
                StagingFocus::Unstaged => app
                    .tab()
                    .unstaged_changes
                    .get(app.tab().unstaged_list_state.selected().unwrap_or(0))
                    .map(|d| d.display_path().to_string()),
                StagingFocus::Staged => app
                    .tab()
                    .staged_changes
                    .get(app.tab().staged_list_state.selected().unwrap_or(0))
                    .map(|d| d.display_path().to_string()),
            };
            if let Some(p) = path {
                app.open_file_blame(p);
            }
        }

        // Delete the currently selected unstaged file (with confirmation)
        KeyCode::Char('D') => {
            app.tab_mut().confirm_discard = false;
            if app.tab().staging_focus == StagingFocus::Unstaged {
                if app.tab().confirm_delete_file.is_some() {
                    app.confirm_delete_file();
                } else {
                    let path = app
                        .tab()
                        .unstaged_changes
                        .get(app.tab().unstaged_list_state.selected().unwrap_or(0))
                        .map(|d| d.display_path().to_string());
                    if let Some(p) = path {
                        app.prompt_delete_file(p);
                    }
                }
            }
        }

        // Any other key cancels the discard confirmation
        _ => {
            if app.tab().confirm_discard {
                app.tab_mut().confirm_discard = false;
                app.tab_mut().status_message = Some("Discard cancelled".into());
            }
            if app.tab().confirm_delete_file.is_some() {
                app.tab_mut().confirm_delete_file = None;
                app.tab_mut().status_message = Some("Delete cancelled".into());
            }
        }
    }
}

/// Move selection down in the currently focused sub-list and update the range anchor.
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
            // Only update the anchor when no multi-selection is active so that
            // plain j/k navigation inside an existing range doesn't reset it.
            if tab.selected_unstaged.len() <= 1 {
                tab.anchor_unstaged = Some(i);
            }
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
            if tab.selected_staged.len() <= 1 {
                tab.anchor_staged = Some(i);
            }
        }
    }
}

/// Move selection up in the currently focused sub-list and update the range anchor.
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
            if tab.selected_unstaged.len() <= 1 {
                tab.anchor_unstaged = Some(i);
            }
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
            if tab.selected_staged.len() <= 1 {
                tab.anchor_staged = Some(i);
            }
        }
    }
}

// ── Shift-range selection ─────────────────────────────────────────────────────

/// Shared body for Shift+Down / Shift+Up staging range selection.
///
/// `next_idx_fn` maps `(current, len) → Option<new_idx>`, returning `None`
/// when the cursor is already at the boundary so we don't wrap around.
fn extend_staging_selection(app: &mut App, next_idx_fn: impl Fn(usize, usize) -> Option<usize>) {
    app.tab_mut().confirm_discard = false;
    let tab = app.tab_mut();
    match tab.staging_focus {
        StagingFocus::Unstaged => {
            if tab.unstaged_changes.is_empty() {
                return;
            }
            let len = tab.unstaged_changes.len();
            let current = tab.unstaged_list_state.selected().unwrap_or(0);
            let anchor = tab.anchor_unstaged.unwrap_or(current);
            // If at the boundary, stay on the current item but still build the
            // anchor-to-current range so pressing J/K always gives visible feedback.
            let new_idx = next_idx_fn(current, len).unwrap_or(current);
            for i in gitkraft_core::ascending_range(anchor, new_idx) {
                tab.selected_unstaged.insert(i);
            }
            tab.unstaged_list_state.select(Some(new_idx));
            let count = tab.selected_unstaged.len();
            tab.status_message = Some(format!("{count} file(s) selected"));
        }
        StagingFocus::Staged => {
            if tab.staged_changes.is_empty() {
                return;
            }
            let len = tab.staged_changes.len();
            let current = tab.staged_list_state.selected().unwrap_or(0);
            let anchor = tab.anchor_staged.unwrap_or(current);
            let new_idx = next_idx_fn(current, len).unwrap_or(current);
            for i in gitkraft_core::ascending_range(anchor, new_idx) {
                tab.selected_staged.insert(i);
            }
            tab.staged_list_state.select(Some(new_idx));
            let count = tab.selected_staged.len();
            tab.status_message = Some(format!("{count} file(s) selected"));
        }
    }
}

/// Extend the file range selection downward (Shift+Down / Shift+j / J).
pub fn select_down(app: &mut App) {
    extend_staging_selection(
        app,
        |cur, len| {
            if cur + 1 >= len {
                None
            } else {
                Some(cur + 1)
            }
        },
    );
}

/// Extend the file range selection upward (Shift+Up / Shift+k / K).
pub fn select_up(app: &mut App) {
    extend_staging_selection(app, |cur, _| if cur == 0 { None } else { Some(cur - 1) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    fn make_unstaged(app: &mut App, count: usize) {
        app.tab_mut().unstaged_changes = (0..count)
            .map(|i| gitkraft_core::DiffInfo {
                old_file: String::new(),
                new_file: format!("file{i}.rs"),
                status: gitkraft_core::FileStatus::Modified,
                hunks: vec![],
            })
            .collect();
        app.tab_mut().unstaged_list_state.select(Some(0));
    }

    fn make_staged(app: &mut App, count: usize) {
        app.tab_mut().staged_changes = (0..count)
            .map(|i| gitkraft_core::DiffInfo {
                old_file: String::new(),
                new_file: format!("staged{i}.rs"),
                status: gitkraft_core::FileStatus::New,
                hunks: vec![],
            })
            .collect();
        app.tab_mut().staged_list_state.select(Some(0));
        app.tab_mut().staging_focus = crate::app::StagingFocus::Staged;
    }

    // ── navigate sets anchor ──────────────────────────────────────────────────

    #[test]
    fn navigate_down_sets_anchor_unstaged() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        navigate_down(&mut app);
        assert_eq!(app.tab().anchor_unstaged, Some(1));
    }

    #[test]
    fn navigate_up_sets_anchor_staged() {
        let mut app = App::new();
        make_staged(&mut app, 3);
        app.tab_mut().staged_list_state.select(Some(2));
        navigate_up(&mut app);
        assert_eq!(app.tab().anchor_staged, Some(1));
    }

    // ── select_down (Shift+J / Shift+Down) ───────────────────────────────────

    #[test]
    fn select_down_adds_range_to_unstaged_selection() {
        let mut app = App::new();
        make_unstaged(&mut app, 5);
        app.tab_mut().unstaged_list_state.select(Some(1));
        app.tab_mut().anchor_unstaged = Some(1);

        select_down(&mut app);

        assert!(app.tab().selected_unstaged.contains(&1));
        assert!(app.tab().selected_unstaged.contains(&2));
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(2));
    }

    #[test]
    fn select_down_extends_existing_selection() {
        let mut app = App::new();
        make_unstaged(&mut app, 5);
        // Simulate two prior Shift+Down presses: anchor=1, cursor now at 2,
        // selection already contains {1, 2}.
        app.tab_mut().unstaged_list_state.select(Some(2));
        app.tab_mut().anchor_unstaged = Some(1);
        app.tab_mut().selected_unstaged.insert(1);
        app.tab_mut().selected_unstaged.insert(2);

        select_down(&mut app);

        // range from anchor(1) to new cursor(3) → {1, 2, 3}
        assert!(app.tab().selected_unstaged.contains(&1));
        assert!(app.tab().selected_unstaged.contains(&2));
        assert!(app.tab().selected_unstaged.contains(&3));
    }

    #[test]
    fn select_down_stops_at_last_file() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        app.tab_mut().unstaged_list_state.select(Some(2));
        app.tab_mut().anchor_unstaged = Some(2);

        select_down(&mut app);

        // cursor must not move past the end
        assert_eq!(app.tab().unstaged_list_state.selected(), Some(2));
        // but the anchor-to-current range (just {2}) is still added to the selection
        assert!(app.tab().selected_unstaged.contains(&2));
    }

    #[test]
    fn select_down_noop_on_empty_list() {
        let mut app = App::new();
        // no files
        select_down(&mut app);
        assert!(app.tab().selected_unstaged.is_empty());
    }

    // ── select_up (Shift+K / Shift+Up) ───────────────────────────────────────

    #[test]
    fn select_up_adds_range_to_staged_selection() {
        let mut app = App::new();
        make_staged(&mut app, 5);
        app.tab_mut().staged_list_state.select(Some(3));
        app.tab_mut().anchor_staged = Some(3);

        select_up(&mut app);

        assert!(app.tab().selected_staged.contains(&2));
        assert!(app.tab().selected_staged.contains(&3));
        assert_eq!(app.tab().staged_list_state.selected(), Some(2));
    }

    #[test]
    fn select_up_stops_at_first_file() {
        let mut app = App::new();
        make_staged(&mut app, 3);
        app.tab_mut().staged_list_state.select(Some(0));
        app.tab_mut().anchor_staged = Some(0);

        select_up(&mut app);

        assert_eq!(app.tab().staged_list_state.selected(), Some(0));
        // anchor-to-current range (just {0}) is still added to the selection
        assert!(app.tab().selected_staged.contains(&0));
    }

    // ── handle_key dispatch ───────────────────────────────────────────────────

    #[test]
    fn shift_j_calls_select_down_in_unstaged() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        app.tab_mut().unstaged_list_state.select(Some(0));
        app.tab_mut().anchor_unstaged = Some(0);

        handle_key(&mut app, key_shift(KeyCode::Char('j')));

        assert!(app.tab().selected_unstaged.contains(&0));
        assert!(app.tab().selected_unstaged.contains(&1));
    }

    #[test]
    fn uppercase_j_calls_select_down() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        app.tab_mut().unstaged_list_state.select(Some(0));
        app.tab_mut().anchor_unstaged = Some(0);

        handle_key(&mut app, key(KeyCode::Char('J')));

        assert!(app.tab().selected_unstaged.contains(&0));
        assert!(app.tab().selected_unstaged.contains(&1));
    }

    #[test]
    fn uppercase_k_calls_select_up() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        app.tab_mut().unstaged_list_state.select(Some(2));
        app.tab_mut().anchor_unstaged = Some(2);

        handle_key(&mut app, key(KeyCode::Char('K')));

        assert!(app.tab().selected_unstaged.contains(&1));
        assert!(app.tab().selected_unstaged.contains(&2));
    }

    #[test]
    fn plain_j_navigates_without_selecting() {
        let mut app = App::new();
        make_unstaged(&mut app, 3);
        app.tab_mut().unstaged_list_state.select(Some(0));

        handle_key(&mut app, key(KeyCode::Char('j')));

        assert_eq!(app.tab().unstaged_list_state.selected(), Some(1));
        // plain navigation must NOT add items to the selection set
        assert!(app.tab().selected_unstaged.is_empty());
    }

    // ── anchor defaults to current cursor when not explicitly set ─────────────

    #[test]
    fn select_down_uses_current_cursor_as_anchor_when_unset() {
        let mut app = App::new();
        make_unstaged(&mut app, 4);
        app.tab_mut().unstaged_list_state.select(Some(1));
        // anchor_unstaged is None — should default to current position (1)

        select_down(&mut app);

        // range from 1 to 2
        assert!(app.tab().selected_unstaged.contains(&1));
        assert!(app.tab().selected_unstaged.contains(&2));
    }
}
