use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::app::{App, AppScreen};
use crate::features;
use crate::widgets;

/// Main render entry point — called once per frame from the event loop.
pub fn render(app: &mut App, frame: &mut Frame) {
    app.rects.screen = frame.area();
    // Paint the entire terminal area with the theme's background colour.
    // Without this, light themes appear broken because ratatui defaults
    // unstyled cells to `Color::Reset` (the terminal's own background).
    let bg = app.theme().bg;
    frame.render_widget(
        Block::default().style(Style::default().bg(bg)),
        frame.area(),
    );

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
        AppScreen::Hub => {
            features::hub::view::render(app, frame, frame.area());
        }
    }
    if app.show_editor_panel && app.screen == AppScreen::Hub {
        let full = frame.area();
        let rows = gitkraft_core::EDITOR_NAMES.len() as u16 + 4;
        let w = 56.min(full.width);
        let h = rows.min(full.height);
        let area = Rect::new(
            full.x + full.width.saturating_sub(w) / 2,
            full.y + full.height.saturating_sub(h) / 2,
            w,
            h,
        );
        frame.render_widget(ratatui::widgets::Clear, area);
        app.rects.editor = area;
        features::editor::view::render(app, frame, area);
    }
    features::mouse::render_menu(app, frame);
    app.menu.tick(std::time::Instant::now());
    features::menu::view::render(app, frame, std::time::Instant::now());
}

/// Render the full Main screen layout with header, content columns, staging
/// area, and status bar.
fn render_main(app: &mut App, frame: &mut Frame) {
    // Show the shimmering skeleton while the initial repo data is loading.
    // Once commits arrive the real content takes over; background refreshes
    // (watcher-triggered) use real content + the status-bar spinner instead.
    let initial_load = app.tab().is_loading && app.tab().commits.is_empty();
    if initial_load {
        features::skeleton::render(app, frame);
        return;
    }
    let ideal_sidebar = ideal_sidebar_width(app);
    let areas = MainAreas::compute(frame.area(), ideal_sidebar);
    let outer = areas.outer;

    // Header
    widgets::header::render(app, frame, outer[0]);

    let main_cols = areas.cols;

    let mut rects = features::mouse::PaneRects {
        screen: frame.area(),
        ..Default::default()
    };
    if areas.show_sidebar {
        // Branches get the lion's share; stashes and remotes sit below.
        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),    // branches
                Constraint::Length(5), // stashes
                Constraint::Length(5), // remotes
            ])
            .split(main_cols[0]);
        rects.branches = sidebar[0];
        rects.stash = sidebar[1];
        features::branches::view::render(app, frame, sidebar[0]);
        features::stash::view::render(app, frame, sidebar[1]);
        features::remotes::view::render(app, frame, sidebar[2]);
    }

    // Commit log
    rects.commits = main_cols[1];
    features::commits::view::render(app, frame, main_cols[1]);

    // Compute a full-height overlay rect spanning main_cols[2] down through
    // the staging area (outer[2]) for theme/options panels.
    let overlay_rect = Rect {
        x: main_cols[2].x,
        y: main_cols[2].y,
        width: main_cols[2].width,
        height: main_cols[2].height + outer[2].height,
    };

    app.panel_rect = if app.show_about_panel || app.show_theme_panel || app.show_options_panel {
        overlay_rect
    } else {
        Rect::default()
    };

    // Diff view OR theme panel OR options panel (full-height overlay)
    if app.show_about_panel {
        features::about::view::render(app, frame, overlay_rect);
    } else if app.show_theme_panel {
        features::theme::view::render(app, frame, overlay_rect);
    } else if app.show_options_panel {
        features::options::view::render(app, frame, overlay_rect);
    } else if app.show_editor_panel {
        rects.editor = main_cols[2];
        features::editor::view::render(app, frame, main_cols[2]);
    } else if app.tab().file_history_path.is_some() {
        features::diff::view::render_file_history(app, frame, main_cols[2]);
    } else if app.tab().blame_path.is_some() {
        features::diff::view::render_blame(app, frame, main_cols[2]);
    } else {
        rects.diff = main_cols[2];
        features::diff::view::render(app, frame, main_cols[2]);
    }

    // Staging area (only when no overlay is active)
    if !app.show_theme_panel && !app.show_options_panel && !app.show_about_panel {
        let cols = features::staging::view::columns(app, outer[2]);
        rects.unstaged = cols[0];
        rects.staged = cols[1];
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
    app.rects = rects;

    // Status bar
    widgets::status_bar::render(app, frame, outer[3]);
}

/// Terminal width below which the sidebar is hidden.
pub const NARROW_COLS: u16 = 80;
/// Terminal height below which the staging area gets a smaller share.
pub const SHORT_ROWS: u16 = 24;

/// Ideal sidebar width: longest branch name + 6 chars of chrome
/// (2 border, 2 highlight "▶ ", 2 prefix "* " / "⇄ ").
fn ideal_sidebar_width(app: &App) -> u16 {
    let longest = app
        .tab()
        .branches
        .iter()
        .map(|b| b.name.chars().count())
        .max()
        .unwrap_or(10);
    (longest + 6) as u16
}

/// Pure geometry of the Main screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainAreas {
    /// header, content, staging, status bar.
    pub outer: [Rect; 4],
    /// sidebar, commit log, diff. Sidebar is zero-width when hidden.
    pub cols: [Rect; 3],
    pub show_sidebar: bool,
}

impl MainAreas {
    pub fn compute(area: Rect, ideal_sidebar: u16) -> Self {
        let short = area.height < SHORT_ROWS;
        let (content_pct, staging_pct) = if short { (70, 30) } else { (60, 40) };
        let o = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(content_pct),
                Constraint::Percentage(staging_pct),
                Constraint::Length(1),
            ])
            .split(area);

        let show_sidebar = area.width >= NARROW_COLS;
        // Up to 30% of the width, clamped to [22, 50].
        let max_sidebar = (o[1].width * 30 / 100).clamp(22, 50);
        let sidebar_w = if show_sidebar {
            ideal_sidebar.min(max_sidebar).max(22)
        } else {
            0
        };
        let commit_pct = if show_sidebar { 40 } else { 50 };
        let c = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(sidebar_w),
                Constraint::Percentage(commit_pct),
                Constraint::Min(20),
            ])
            .split(o[1]);

        Self {
            outer: [o[0], o[1], o[2], o[3]],
            cols: [c[0], c[1], c[2]],
            show_sidebar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }

    #[test]
    fn wide_terminal_shows_sidebar_within_bounds() {
        let a = MainAreas::compute(area(200, 50), 30);
        assert!(a.show_sidebar);
        assert_eq!(a.cols[0].width, 30);
    }

    #[test]
    fn sidebar_width_clamped_to_50() {
        let a = MainAreas::compute(area(300, 50), 120);
        assert_eq!(a.cols[0].width, 50);
    }

    #[test]
    fn sidebar_min_22() {
        let a = MainAreas::compute(area(120, 50), 5);
        assert_eq!(a.cols[0].width, 22);
    }

    #[test]
    fn narrow_terminal_hides_sidebar() {
        let a = MainAreas::compute(area(NARROW_COLS - 1, 40), 30);
        assert!(!a.show_sidebar);
        assert_eq!(a.cols[0].width, 0);
        assert!(MainAreas::compute(area(NARROW_COLS, 40), 30).show_sidebar);
    }

    #[test]
    fn columns_cover_content_width() {
        for w in [40u16, 79, 80, 120, 250] {
            let a = MainAreas::compute(area(w, 40), 30);
            let total: u16 = a.cols.iter().map(|r| r.width).sum();
            assert_eq!(total, w, "width {w}");
            assert!(a.cols[2].width > 0, "diff visible at {w}");
        }
    }

    #[test]
    fn short_terminal_shrinks_staging() {
        let tall = MainAreas::compute(area(120, 40), 30);
        let short = MainAreas::compute(area(120, SHORT_ROWS - 1), 30);
        assert!(
            short.outer[2].height * tall.outer[1].height
                < tall.outer[2].height * short.outer[1].height * 2
        );
        assert!(short.outer[1].height > short.outer[2].height);
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let a = MainAreas::compute(area(1, 1), 30);
        assert_eq!(a.outer.len(), 4);
    }

    #[test]
    fn drawing_the_main_screen_records_where_the_panes_are_for_the_mouse() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let mut app = App::new();
        app.screen = AppScreen::Main;
        let mut term = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        term.draw(|f| render(&mut app, f)).expect("draw");
        let r = app.rects;
        assert_eq!(r.screen, Rect::new(0, 0, 120, 40));
        for (name, rect) in [
            ("branches", r.branches),
            ("stash", r.stash),
            ("commits", r.commits),
            ("diff", r.diff),
            ("unstaged", r.unstaged),
            ("staged", r.staged),
        ] {
            assert!(rect.width > 0 && rect.height > 0, "{name} {rect:?}");
        }
        // two columns until the Actions pane is shown
        assert_eq!(r.unstaged.width, r.staged.width);
        assert!(r.unstaged.x + r.unstaged.width <= r.staged.x);
    }

    fn drawn(app: &mut App) {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let mut term = Terminal::new(TestBackend::new(120, 40)).expect("terminal");
        term.draw(|f| render(app, f)).expect("draw");
    }

    fn click(app: &mut App, x: u16, y: u16) {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        features::mouse::handle(
            app,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            },
        );
    }

    #[test]
    fn header_hints_tabs_the_theme_panel_and_the_settings_menu_react_to_clicks() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.new_tab();
        app.screen = AppScreen::Main;
        app.active_tab_index = 0;
        drawn(&mut app);
        // a tab
        let (r, _) = *app
            .header_hits
            .iter()
            .find(|(_, h)| matches!(h, features::mouse::HeaderHit::Tab(1)))
            .expect("second tab");
        click(&mut app, r.x, r.y);
        assert_eq!(app.active_tab_index, 1);
        app.screen = AppScreen::Main;
        // a hint: "[A] actions" shows the Actions pane
        drawn(&mut app);
        let (r, _) = *app
            .header_hits
            .iter()
            .find(|(_, h)| {
                *h == features::mouse::HeaderHit::Key(crossterm::event::KeyCode::Char('A'))
            })
            .expect("actions hint");
        click(&mut app, r.x + 1, r.y);
        assert!(app.show_actions_panel);
        app.show_theme_panel = true;
        // a row of the panel picks that theme; a click outside closes it
        drawn(&mut app);
        let p = app.panel_rect;
        let want = app.theme_list_state.offset() + 3; // the list may be scrolled
        click(&mut app, p.x + 3, p.y + 1 + 3);
        assert_eq!(app.picker_index(), want);
        click(&mut app, 1, 39);
        assert!(!app.show_theme_panel);
        // the settings menu: a row toggles reduced motion
        app.reduced_motion = true; // opens instantly
        app.open_menu();
        drawn(&mut app);
        let rect = features::menu::view::popup_rect(app.rects.screen, 1.0);
        let row = features::menu::Row::ALL
            .iter()
            .position(|r| *r == features::menu::Row::ReducedMotion)
            .expect("row") as u16;
        let before = app.reduced_motion;
        click(&mut app, rect.x + 3, rect.y + 2 + row);
        assert_ne!(app.reduced_motion, before);
        click(&mut app, 0, 0);
        assert!(!app.menu.is_open(), "outside closes");
    }

    /// Not a test: `cargo test -p gitkraft-tui --release -- --ignored --nocapture bench_main`
    /// opens this very repository and prints what a frame of the main screen costs.
    #[test]
    #[ignore = "a measurement, not a check"]
    fn bench_main_screen_frame() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut app = App::new();
        app.open_repo(root);
        let started = std::time::Instant::now();
        while app.tab().is_loading && started.elapsed().as_secs() < 60 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            app.poll_background();
        }
        println!(
            "loaded {} commits in {:?}",
            app.tab().commits.len(),
            started.elapsed()
        );
        let mut term = Terminal::new(TestBackend::new(200, 55)).expect("terminal");
        term.draw(|f| render(&mut app, f)).expect("warm up");
        let n = 200;
        let t = std::time::Instant::now();
        for _ in 0..n {
            term.draw(|f| render(&mut app, f)).expect("draw");
        }
        println!("main screen: {:?} per frame", t.elapsed() / n);
    }
}
