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
        // Extend range selection downward.
        // J (Shift+j) works in every terminal because terminals uppercase the
        // letter; Shift+j with explicit SHIFT modifier is also handled for
        // terminals that support keyboard enhancement.
        KeyCode::Char('J') => select_file_down(app),
        // Extend range selection upward.
        KeyCode::Char('K') => select_file_up(app),
        // Enter diff content sub-pane
        KeyCode::Enter | KeyCode::Char('l') if !app.tab().commit_files.is_empty() => {
            app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        }
        // File history for the currently highlighted commit file
        KeyCode::Char('H') => {
            let path = app
                .tab()
                .commit_files
                .get(app.tab().commit_diff_file_index)
                .map(|f| f.display_path().to_string());
            if let Some(p) = path {
                app.open_file_history(p);
            }
        }
        // Blame for the currently highlighted commit file
        KeyCode::Char('B') => {
            let path = app
                .tab()
                .commit_files
                .get(app.tab().commit_diff_file_index)
                .map(|f| f.display_path().to_string());
            if let Some(p) = path {
                app.open_file_blame(p);
            }
        }
        // Open the focused file (or all selected files) in the configured editor.
        KeyCode::Char('e') => {
            app.open_commit_files_in_editor();
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
        // Open the focused file (or all selected files) in the configured editor.
        // Works from Content sub-pane too so the user doesn't need to switch back.
        KeyCode::Char('e') => {
            app.open_commit_files_in_editor();
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

/// Shared body for Shift+Up/Down file-range selection.
///
/// Uses `anchor_file_index` as the fixed end of the range and moves the
/// cursor to the result of `next_idx_fn`.  If already at a boundary the
/// cursor stays but the anchor-to-current range is still applied so the
/// user always gets visible feedback on the first key press.
///
/// The selection is **replaced** (not accumulated) with
/// `ascending_range(anchor, new_cursor)` on every call, matching standard
/// range-selection behaviour (Shift+Up shrinks what Shift+Down expanded).
fn extend_file_selection(app: &mut App, next_idx_fn: impl Fn(usize, usize) -> Option<usize>) {
    if app.tab().commit_files.is_empty() {
        return;
    }
    let len = app.tab().commit_files.len();
    let current = app.tab().commit_diff_file_index;
    let anchor = app.tab().anchor_file_index.unwrap_or(current);
    // At boundary: stay on current item but still build the range so the
    // first press always produces visible feedback.
    let new_idx = next_idx_fn(current, len).unwrap_or(current);

    // Replace the selection with the full anchor-to-cursor range.
    let range: std::collections::HashSet<usize> = gitkraft_core::ascending_range(anchor, new_idx)
        .into_iter()
        .collect();
    app.tab_mut().selected_file_indices = range;
    app.tab_mut().commit_diff_file_index = new_idx;

    let count = app.tab().selected_file_indices.len();
    app.tab_mut().status_message = Some(format!("{count} file(s) selected"));

    // Load diffs for ALL files in the new selection range, not just the
    // focused one.  `load_diff_for_file_index` is a no-op for files already
    // in `commit_diffs`, so iterating the whole set is safe and cheap.
    // This ensures the multi-file concatenated view renders immediately
    // instead of showing "Loading…" for every non-focused file.
    let all_selected: Vec<usize> = app.tab().selected_file_indices.iter().copied().collect();
    for idx in all_selected {
        app.load_diff_for_file_index(idx);
    }

    // Also reset the scroll so the concatenated view starts at the top.
    app.tab_mut().diff_scroll = 0;
}

/// Extend the multi-selection downward (Shift+Down in file list).
pub fn select_file_down(app: &mut App) {
    extend_file_selection(
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

/// Extend the multi-selection upward (Shift+Up in file list).
pub fn select_file_up(app: &mut App) {
    extend_file_selection(app, |cur, _| if cur == 0 { None } else { Some(cur - 1) });
}

/// Handle keys when the file-history overlay is active.
pub fn handle_file_history_key(app: &mut App, key: KeyEvent) {
    let len = app.tab().file_history_commits.len();
    match key.code {
        KeyCode::Char('j') | KeyCode::Down if len > 0 => {
            let cur = app.tab().file_history_cursor;
            app.tab_mut().file_history_cursor = (cur + 1).min(len - 1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let cur = app.tab().file_history_cursor;
            app.tab_mut().file_history_cursor = cur.saturating_sub(1);
        }
        KeyCode::Char('g') => {
            app.tab_mut().file_history_cursor = 0;
        }
        KeyCode::Char('G') if len > 0 => {
            app.tab_mut().file_history_cursor = len - 1;
        }
        KeyCode::Enter => {
            // Jump to the selected commit and close the overlay
            let cursor = app.tab().file_history_cursor;
            if let Some(commit) = app.tab().file_history_commits.get(cursor).cloned() {
                let oid = commit.oid.clone();
                let tab = app.tab_mut();
                tab.file_history_path = None;
                tab.file_history_commits.clear();
                tab.selected_commit_oid = Some(oid);
            }
            app.load_commit_diff_by_oid();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            let tab = app.tab_mut();
            tab.file_history_path = None;
            tab.file_history_commits.clear();
            tab.file_history_cursor = 0;
            tab.status_message = Some("File history closed".into());
        }
        _ => {}
    }
}

/// Handle keys when the blame overlay is active.
pub fn handle_blame_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.tab_mut().blame_scroll = app.tab().blame_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.tab_mut().blame_scroll = app.tab().blame_scroll.saturating_sub(1);
        }
        KeyCode::Char('d') => {
            app.tab_mut().blame_scroll = app.tab().blame_scroll.saturating_add(10);
        }
        KeyCode::Char('u') => {
            app.tab_mut().blame_scroll = app.tab().blame_scroll.saturating_sub(10);
        }
        KeyCode::Char('g') => {
            app.tab_mut().blame_scroll = 0;
        }
        KeyCode::Char('G') => {
            let len = app.tab().blame_lines.len() as u16;
            app.tab_mut().blame_scroll = len.saturating_sub(1);
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            let tab = app.tab_mut();
            tab.blame_path = None;
            tab.blame_lines.clear();
            tab.blame_scroll = 0;
            tab.status_message = Some("Blame closed".into());
        }
        _ => {}
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Move to a single file, clearing multi-selection, and trigger a diff load.
fn apply_single_file_navigation(app: &mut App, new_idx: usize) {
    // Plain navigation fixes the anchor for any subsequent Shift range selection.
    app.tab_mut().anchor_file_index = Some(new_idx);
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

    // ── handle_file_history_key ───────────────────────────────────────────

    fn make_commit_info(summary: &str) -> gitkraft_core::CommitInfo {
        gitkraft_core::CommitInfo {
            oid: "abc1234567890".to_string(),
            short_oid: "abc1234".to_string(),
            summary: summary.to_string(),
            message: summary.to_string(),
            author_name: "author".to_string(),
            author_email: "a@b.com".to_string(),
            time: Default::default(),
            parent_ids: vec![],
        }
    }

    fn app_with_history() -> App {
        let mut app = App::new();
        app.tab_mut().file_history_path = Some("src/main.rs".to_string());
        app.tab_mut().file_history_commits = vec![
            make_commit_info("commit 0"),
            make_commit_info("commit 1"),
            make_commit_info("commit 2"),
        ];
        app.tab_mut().file_history_cursor = 0;
        app
    }

    #[test]
    fn file_history_j_moves_cursor_down() {
        let mut app = app_with_history();
        handle_file_history_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().file_history_cursor, 1);
    }

    #[test]
    fn file_history_k_moves_cursor_up() {
        let mut app = app_with_history();
        app.tab_mut().file_history_cursor = 2;
        handle_file_history_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().file_history_cursor, 1);
    }

    #[test]
    fn file_history_cursor_clamps_at_bottom() {
        let mut app = app_with_history();
        app.tab_mut().file_history_cursor = 2;
        handle_file_history_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().file_history_cursor, 2);
    }

    #[test]
    fn file_history_cursor_clamps_at_top() {
        let mut app = app_with_history();
        handle_file_history_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().file_history_cursor, 0);
    }

    #[test]
    fn file_history_esc_closes_overlay() {
        let mut app = app_with_history();
        handle_file_history_key(&mut app, key(KeyCode::Esc));
        assert!(app.tab().file_history_path.is_none());
        assert!(app.tab().file_history_commits.is_empty());
    }

    // ── handle_blame_key ──────────────────────────────────────────────────

    fn app_with_blame() -> App {
        let mut app = App::new();
        app.tab_mut().blame_path = Some("src/main.rs".to_string());
        app.tab_mut().blame_scroll = 5;
        app
    }

    #[test]
    fn blame_j_scrolls_down() {
        let mut app = app_with_blame();
        handle_blame_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.tab().blame_scroll, 6);
    }

    #[test]
    fn blame_k_scrolls_up() {
        let mut app = app_with_blame();
        handle_blame_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.tab().blame_scroll, 4);
    }

    #[test]
    fn blame_esc_closes_overlay() {
        let mut app = app_with_blame();
        handle_blame_key(&mut app, key(KeyCode::Esc));
        assert!(app.tab().blame_path.is_none());
        assert!(app.tab().blame_lines.is_empty());
        assert_eq!(app.tab().blame_scroll, 0);
    }

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
        app.tab_mut().anchor_file_index = Some(2);
        select_file_down(&mut app);
        // cursor must not move past the end
        assert_eq!(app.tab().commit_diff_file_index, 2);
        // but the anchor-to-current range ({2}) is still applied
        assert!(app.tab().selected_file_indices.contains(&2));
    }

    #[test]
    fn select_file_down_extends_existing_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(4);
        // Anchor at 0; cursor at 1 after a prior Shift+Down.
        app.tab_mut().anchor_file_index = Some(0);
        app.tab_mut().commit_diff_file_index = 1;
        select_file_down(&mut app);
        // Range is replaced with ascending_range(anchor=0, new_cursor=2) = {0,1,2}
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
        app.tab_mut().anchor_file_index = Some(0);
        select_file_up(&mut app);
        // cursor must not move before the start
        assert_eq!(app.tab().commit_diff_file_index, 0);
        // but the anchor-to-current range ({0}) is still applied
        assert!(app.tab().selected_file_indices.contains(&0));
    }

    #[test]
    fn select_file_up_extends_existing_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = make_commit_files(4);
        // Anchor at 3; cursor at 2 after a prior Shift+Up.
        app.tab_mut().anchor_file_index = Some(3);
        app.tab_mut().commit_diff_file_index = 2;
        select_file_up(&mut app);
        // Range is replaced with ascending_range(anchor=3, new_cursor=1) = {1,2,3}
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

    #[test]
    fn e_in_content_sub_pane_also_opens_file() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/repo"));
        app.tab_mut().commit_files = vec![gitkraft_core::DiffFileEntry {
            old_file: String::new(),
            new_file: "src/lib.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
        }];
        // User has pressed → to enter the Content sub-pane
        app.tab_mut().diff_sub_pane = DiffSubPane::Content;
        app.editor = gitkraft_core::Editor::Helix;

        handle_key(&mut app, key(KeyCode::Char('e')));

        assert!(
            app.pending_editor_open.is_some(),
            "e in Content sub-pane must also queue a terminal editor open"
        );
    }

    #[test]
    fn e_in_file_list_queues_editor_open() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/repo"));
        app.tab_mut().commit_files = vec![gitkraft_core::DiffFileEntry {
            old_file: String::new(),
            new_file: "src/lib.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
        }];
        app.tab_mut().diff_sub_pane = DiffSubPane::FileList;
        app.editor = gitkraft_core::Editor::Helix; // terminal editor → queued

        handle_key(&mut app, key(KeyCode::Char('e')));

        assert!(
            app.pending_editor_open.is_some(),
            "e in file list must queue a terminal editor open"
        );
    }
}
