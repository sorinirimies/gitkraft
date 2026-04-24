use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, DiffSubPane};

/// Handle keys when the Diff pane is the active pane.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.tab().diff_sub_pane {
        DiffSubPane::FileList => handle_file_list_key(app, key),
        DiffSubPane::Content => handle_content_key(app, key),
    }
}

fn handle_file_list_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Navigate files (clears selection)
        KeyCode::Char('j') => navigate_file_down(app),
        KeyCode::Char('k') => navigate_file_up(app),
        // Enter diff content sub-pane
        KeyCode::Enter | KeyCode::Char('l') => {
            if !app.tab().commit_files.is_empty() {
                app.tab_mut().diff_sub_pane = DiffSubPane::Content;
            }
        }
        _ => {}
    }
}

fn handle_content_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Scroll down
        KeyCode::Char('j') => {
            app.tab_mut().diff_scroll = app.tab().diff_scroll.saturating_add(1);
        }
        // Scroll up
        KeyCode::Char('k') => {
            app.tab_mut().diff_scroll = app.tab().diff_scroll.saturating_sub(1);
        }
        // Scroll to top
        KeyCode::Char('g') => {
            app.tab_mut().diff_scroll = 0;
        }
        // Scroll to bottom (Shift+G)
        KeyCode::Char('G') => {
            let total_lines = app
                .tab()
                .selected_diff
                .as_ref()
                .map(|d| d.hunks.iter().map(|h| h.lines.len() as u16).sum::<u16>())
                .unwrap_or(0);
            app.tab_mut().diff_scroll = total_lines.saturating_sub(1);
        }
        // Page down
        KeyCode::PageDown | KeyCode::Char('d') => {
            app.tab_mut().diff_scroll = app.tab().diff_scroll.saturating_add(20);
        }
        // Page up
        KeyCode::PageUp | KeyCode::Char('u') => {
            app.tab_mut().diff_scroll = app.tab().diff_scroll.saturating_sub(20);
        }
        // Previous file in commit diff (stay in content sub-pane)
        KeyCode::Char('h') => {
            navigate_file_up(app);
            app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        }
        // Next file in commit diff (stay in content sub-pane)
        KeyCode::Char('l') => {
            navigate_file_down(app);
            app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        }
        // Return to file list
        KeyCode::Esc => {
            app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        }
        _ => {}
    }
}

/// Navigate to the next file without multi-select (clears selection).
pub fn navigate_file_down(app: &mut App) {
    if app.tab().commit_files.is_empty() {
        return;
    }
    let len = app.tab().commit_files.len();
    let current = app.tab().commit_diff_file_index;
    let new_idx = (current + 1) % len;
    apply_single_file_navigation(app, new_idx);
}

/// Navigate to the previous file without multi-select (clears selection).
pub fn navigate_file_up(app: &mut App) {
    if app.tab().commit_files.is_empty() {
        return;
    }
    let len = app.tab().commit_files.len();
    let current = app.tab().commit_diff_file_index;
    let new_idx = if current == 0 { len - 1 } else { current - 1 };
    apply_single_file_navigation(app, new_idx);
}

/// Extend the multi-selection downward (Shift+Down in file list).
pub fn select_file_down(app: &mut App) {
    if app.tab().commit_files.is_empty() {
        return;
    }
    let current = app.tab().commit_diff_file_index;
    let len = app.tab().commit_files.len();
    if current + 1 >= len {
        return;
    }
    let new_idx = current + 1;
    // Anchor the previously focused file into the selection
    app.tab_mut().selected_file_indices.insert(current);
    app.tab_mut().selected_file_indices.insert(new_idx);
    app.tab_mut().commit_diff_file_index = new_idx;
    // Trigger background load for the new file if not already cached
    app.load_diff_for_file_index(new_idx);
}

/// Extend the multi-selection upward (Shift+Up in file list).
pub fn select_file_up(app: &mut App) {
    if app.tab().commit_files.is_empty() {
        return;
    }
    let current = app.tab().commit_diff_file_index;
    if current == 0 {
        return;
    }
    let new_idx = current - 1;
    // Anchor the previously focused file into the selection
    app.tab_mut().selected_file_indices.insert(current);
    app.tab_mut().selected_file_indices.insert(new_idx);
    app.tab_mut().commit_diff_file_index = new_idx;
    // Trigger background load for the new file if not already cached
    app.load_diff_for_file_index(new_idx);
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Move to a single file, clearing multi-selection, and trigger a diff load.
fn apply_single_file_navigation(app: &mut App, new_idx: usize) {
    app.tab_mut().commit_diff_file_index = new_idx;
    app.tab_mut().selected_file_indices.clear();
    app.tab_mut().selected_file_indices.insert(new_idx);
    app.tab_mut().diff_scroll = 0;
    if let Some(cached) = app.tab().commit_diffs.get(&new_idx).cloned() {
        app.tab_mut().selected_diff = Some(cached);
    } else {
        let file_path = app.tab().commit_files[new_idx].display_path().to_string();
        app.load_single_file_diff(new_idx, file_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, DiffSubPane};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_commit_files(count: usize) -> Vec<gitkraft_core::DiffFileEntry> {
        (0..count)
            .map(|i| gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: format!("file{i}.rs"),
                status: gitkraft_core::FileStatus::Modified,
            })
            .collect()
    }

    // ── navigate_file_down ───────────────────────────────────────────────────

    #[test]
    fn navigate_file_down_noop_on_empty_files() {
        let mut app = App::new();
        navigate_file_down(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 0);
        assert!(app.tab().selected_file_indices.is_empty());
    }

    #[test]
    fn navigate_file_down_advances_index() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 0;
        navigate_file_down(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn navigate_file_down_wraps_to_first() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        navigate_file_down(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 0);
    }

    #[test]
    fn navigate_file_down_clears_multi_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        navigate_file_down(&mut app);
        assert_eq!(app.tab().selected_file_indices.len(), 1);
        assert!(app.tab().selected_file_indices.contains(&1));
    }

    #[test]
    fn navigate_file_down_resets_scroll() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(2);
        app.tab_mut().diff_scroll = 99;
        navigate_file_down(&mut app);
        assert_eq!(app.tab().diff_scroll, 0);
    }

    // ── navigate_file_up ─────────────────────────────────────────────────────

    #[test]
    fn navigate_file_up_noop_on_empty_files() {
        let mut app = App::new();
        navigate_file_up(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 0);
        assert!(app.tab().selected_file_indices.is_empty());
    }

    #[test]
    fn navigate_file_up_decreases_index() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        navigate_file_up(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn navigate_file_up_wraps_to_last() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 0;
        navigate_file_up(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 2);
    }

    #[test]
    fn navigate_file_up_clears_multi_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        app.tab_mut().selected_file_indices.insert(1);
        app.tab_mut().selected_file_indices.insert(2);
        navigate_file_up(&mut app);
        assert_eq!(app.tab().selected_file_indices.len(), 1);
        assert!(app.tab().selected_file_indices.contains(&1));
    }

    // ── select_file_down ─────────────────────────────────────────────────────

    #[test]
    fn select_file_down_noop_on_empty_files() {
        let mut app = App::new();
        select_file_down(&mut app);
        assert!(app.tab().selected_file_indices.is_empty());
        assert_eq!(app.tab().commit_diff_file_index, 0);
    }

    #[test]
    fn select_file_down_adds_both_indices_to_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 0;
        select_file_down(&mut app);
        assert!(app.tab().selected_file_indices.contains(&0));
        assert!(app.tab().selected_file_indices.contains(&1));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn select_file_down_stops_at_last_file() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        select_file_down(&mut app);
        // index must not change past the end
        assert_eq!(app.tab().commit_diff_file_index, 2);
        assert!(app.tab().selected_file_indices.is_empty());
    }

    #[test]
    fn select_file_down_extends_existing_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(4);
        // Simulate having already shift-selected 0→1
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        app.tab_mut().commit_diff_file_index = 1;
        select_file_down(&mut app);
        assert!(app.tab().selected_file_indices.contains(&0));
        assert!(app.tab().selected_file_indices.contains(&1));
        assert!(app.tab().selected_file_indices.contains(&2));
        assert_eq!(app.tab().commit_diff_file_index, 2);
    }

    // ── select_file_up ───────────────────────────────────────────────────────

    #[test]
    fn select_file_up_noop_on_empty_files() {
        let mut app = App::new();
        select_file_up(&mut app);
        assert!(app.tab().selected_file_indices.is_empty());
        assert_eq!(app.tab().commit_diff_file_index, 0);
    }

    #[test]
    fn select_file_up_adds_both_indices_to_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        select_file_up(&mut app);
        assert!(app.tab().selected_file_indices.contains(&2));
        assert!(app.tab().selected_file_indices.contains(&1));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn select_file_up_stops_at_first_file() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 0;
        select_file_up(&mut app);
        assert_eq!(app.tab().commit_diff_file_index, 0);
        assert!(app.tab().selected_file_indices.is_empty());
    }

    #[test]
    fn select_file_up_extends_existing_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(4);
        app.tab_mut().selected_file_indices.insert(2);
        app.tab_mut().selected_file_indices.insert(3);
        app.tab_mut().commit_diff_file_index = 2;
        select_file_up(&mut app);
        assert!(app.tab().selected_file_indices.contains(&1));
        assert!(app.tab().selected_file_indices.contains(&2));
        assert!(app.tab().selected_file_indices.contains(&3));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    // ── handle_key — FileList sub-pane ───────────────────────────────────────

    #[test]
    fn j_in_file_list_navigates_down() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn k_in_file_list_navigates_up() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().commit_diff_file_index, 1);
    }

    #[test]
    fn enter_in_file_list_enters_content_sub_pane() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(2);
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::Content);
    }

    #[test]
    fn l_in_file_list_enters_content_sub_pane() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(2);
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        handle_key(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::Content);
    }

    #[test]
    fn enter_in_file_list_without_files_stays_in_file_list() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
    }

    // ── handle_key — Content sub-pane ────────────────────────────────────────

    #[test]
    fn j_in_content_scrolls_down() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 3;
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().diff_scroll, 4);
    }

    #[test]
    fn k_in_content_scrolls_up() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 5;
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().diff_scroll, 4);
    }

    #[test]
    fn k_in_content_does_not_underflow() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 0;
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().diff_scroll, 0);
    }

    #[test]
    fn g_in_content_scrolls_to_top() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 42;
        handle_key(&mut app, key(KeyCode::Char('g')));
        assert_eq!(app.tab().diff_scroll, 0);
    }

    #[test]
    fn d_in_content_pages_down() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 0;
        handle_key(&mut app, key(KeyCode::Char('d')));
        assert_eq!(app.tab().diff_scroll, 20);
    }

    #[test]
    fn u_in_content_pages_up() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.tab_mut().diff_scroll = 25;
        handle_key(&mut app, key(KeyCode::Char('u')));
        assert_eq!(app.tab().diff_scroll, 5);
    }

    #[test]
    fn esc_in_content_returns_to_file_list() {
        let mut app = App::new();
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::FileList);
    }

    #[test]
    fn h_in_content_navigates_file_up_and_stays_in_content() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 2;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Char('h')));
        assert_eq!(app.tab().commit_diff_file_index, 1);
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::Content);
    }

    #[test]
    fn l_in_content_navigates_file_down_and_stays_in_content() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(3);
        app.tab_mut().commit_diff_file_index = 0;
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        handle_key(&mut app, key(KeyCode::Char('l')));
        assert_eq!(app.tab().commit_diff_file_index, 1);
        assert_eq!(app.tab().diff_sub_pane, DiffSubPane::Content);
    }
}
