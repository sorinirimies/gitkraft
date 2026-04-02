use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::{App, AppScreen};
use crate::features;
use crate::widgets;

/// Main render entry point — called once per frame from the event loop.
pub fn render(app: &mut App, frame: &mut Frame) {
    match app.screen {
        AppScreen::Welcome => {
            features::repo::view::render(&*app, frame, frame.area());
        }
        AppScreen::Main => {
            render_main(app, frame);
        }
    }
}

/// Render the full Main screen layout with header, content columns, staging
/// area, and status bar.
fn render_main(app: &mut App, frame: &mut Frame) {
    // ── Outer vertical split ──────────────────────────────────────────────
    //  [0] Header bar          — 3 rows
    //  [1] Main content area   — flexible
    //  [2] Staging area        — 12 rows
    //  [3] Status bar          — 1 row
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(12),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Header
    widgets::header::render(app, frame, outer[0]);

    // ── Main content: three columns ───────────────────────────────────────
    //  [0] Sidebar (branches + stashes + remotes) — 25 cols
    //  [1] Commit log                             — 40 %
    //  [2] Diff view                              — remainder
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(25),
            Constraint::Percentage(40),
            Constraint::Min(30),
        ])
        .split(outer[1]);

    // The sidebar is itself split vertically: branches get the lion's share,
    // with stashes and remotes at the bottom.
    let sidebar = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),    // branches
            Constraint::Length(5), // stashes
            Constraint::Length(5), // remotes
        ])
        .split(main_cols[0]);

    features::branches::view::render(app, frame, sidebar[0]);
    features::stash::view::render(app, frame, sidebar[1]);
    features::remotes::view::render(app, frame, sidebar[2]);

    // Commit log
    features::commits::view::render(app, frame, main_cols[1]);

    // Diff view OR theme panel OR options panel
    if app.show_theme_panel {
        features::theme::view::render(app, frame, main_cols[2]);
    } else if app.show_options_panel {
        features::options::view::render(app, frame, main_cols[2]);
    } else {
        features::diff::view::render(app, frame, main_cols[2]);
    }

    // Staging area
    features::staging::view::render(app, frame, outer[2]);

    // Status bar
    widgets::status_bar::render(app, frame, outer[3]);
}
