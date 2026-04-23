use ratatui::layout::{Constraint, Direction, Layout, Rect};
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
        AppScreen::DirBrowser => {
            features::repo::view::render_browser(app, frame, frame.area());
        }
        AppScreen::Main => {
            render_main(app, frame);
        }
    }
}

/// Render the full Main screen layout with header, content columns, staging
/// area, and status bar.
fn render_main(app: &mut App, frame: &mut Frame) {
    // -- Outer vertical split --
    //  [0] Header bar          — 3 rows
    //  [1] Main content area   — flexible
    //  [2] Staging area        — 12 rows
    //  [3] Status bar          — 1 row
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(60),
            Constraint::Percentage(40),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Header
    widgets::header::render(app, frame, outer[0]);

    // -- Main content: three columns --
    //  [0] Sidebar (branches + stashes + remotes) — dynamic width
    //  [1] Commit log                             — 40 %
    //  [2] Diff view                              — remainder

    // Compute sidebar width from the longest branch name + padding for
    // the indicator icon, highlight symbol, and borders (~6 chars overhead).
    let longest_branch = app
        .tab()
        .branches
        .iter()
        .map(|b| b.name.chars().count())
        .max()
        .unwrap_or(10);
    // +6 for: 2 (border) + 2 (highlight "▶ ") + 2 (prefix "* " or "⇄ ")
    let ideal_sidebar = (longest_branch + 6) as u16;
    let term_width = outer[1].width;
    // Sidebar gets up to 30% of terminal width, clamped to [22, 50]
    let max_sidebar = (term_width * 30 / 100).clamp(22, 50);
    let sidebar_width = ideal_sidebar.min(max_sidebar).max(22);

    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(sidebar_width),
            Constraint::Percentage(40),
            Constraint::Min(20),
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

    // Compute a full-height overlay rect spanning main_cols[2] down through
    // the staging area (outer[2]) for theme/options panels.
    let overlay_rect = Rect {
        x: main_cols[2].x,
        y: main_cols[2].y,
        width: main_cols[2].width,
        height: main_cols[2].height + outer[2].height,
    };

    // Diff view OR theme panel OR options panel (full-height overlay)
    if app.show_theme_panel {
        features::theme::view::render(app, frame, overlay_rect);
    } else if app.show_options_panel {
        features::options::view::render(app, frame, overlay_rect);
    } else if app.show_editor_panel {
        features::editor::view::render(app, frame, main_cols[2]);
    } else {
        features::diff::view::render(app, frame, main_cols[2]);
    }

    // Staging area (only when no overlay is active)
    if !app.show_theme_panel && !app.show_options_panel {
        features::staging::view::render(app, frame, outer[2]);
    } else {
        // Render staging only for the left 2/3 (unstaged + staged columns)
        let staging_partial = Rect {
            x: outer[2].x,
            y: outer[2].y,
            width: main_cols[2].x.saturating_sub(outer[2].x),
            height: outer[2].height,
        };
        features::staging::view::render(app, frame, staging_partial);
    }

    // Status bar
    widgets::status_bar::render(app, frame, outer[3]);
}
