use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle keys when the Diff pane is the active pane.
pub fn handle_key(app: &mut App, key: KeyEvent) {
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
            // Compute total line count from the selected diff
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
        // Previous file in commit diff
        KeyCode::Char('h') => {
            app.prev_diff_file();
        }
        // Next file in commit diff
        KeyCode::Char('l') => {
            app.next_diff_file();
        }
        _ => {}
    }
}
