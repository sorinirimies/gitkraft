//! Shimmering skeleton loading screen shown while the initial repo data loads.
//!
//! Uses [`tui_skeleton`] widgets to render animated placeholders that match
//! the exact structure of the main layout (sidebar | commit log | diff,
//! staging at the bottom) so the transition to real content feels seamless.
//!
//! The skeleton is only shown on the **initial** load (when commits are still
//! empty).  Background watcher refreshes use the real content with the
//! status-bar spinner instead.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use tui_skeleton::{AnimationMode, SkeletonBlock, SkeletonList, SkeletonTable};

use crate::app::App;

/// Render the full shimmering skeleton in place of the main view.
///
/// Called from [`crate::layout::render_main`] when `is_loading && commits.is_empty()`.
pub fn render(app: &mut App, frame: &mut Frame) {
    let theme = app.theme();

    // `elapsed_ms` drives all animations — tick_count ≈ frame × 33 ms at 30 fps.
    let elapsed_ms = app.tick_count.saturating_mul(33);

    // Use theme colours so the skeleton looks at home in any colour scheme.
    let base = theme.border_inactive; // dim background fill
    let hi = theme.sel_bg; // brighter shimmer highlight

    let area = frame.area();

    // ── Outer layout: header | content | staging | status ─────────────────
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(60),
            Constraint::Percentage(40),
            Constraint::Length(1),
        ])
        .split(area);

    // Reuse the real header (it is static and always available).
    crate::widgets::header::render(app, frame, outer[0]);

    // ── Main content: sidebar | commit log | diff ────────────────────────
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Percentage(40),
            Constraint::Min(20),
        ])
        .split(outer[1]);

    // Sidebar is split into branches / stashes / remotes.
    let sidebar = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(5),
            Constraint::Length(5),
        ])
        .split(main_cols[0]);

    let inner = pane(frame, sidebar[0], " Branches ", theme.border_inactive);
    frame.render_widget(
        SkeletonList::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .items(10)
            .base(base)
            .highlight(hi),
        inner,
    );

    let inner = pane(frame, sidebar[1], " Stashes ", theme.border_inactive);
    frame.render_widget(
        SkeletonList::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .items(3)
            .base(base)
            .highlight(hi),
        inner,
    );

    let inner = pane(frame, sidebar[2], " Remotes ", theme.border_inactive);
    frame.render_widget(
        SkeletonList::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .items(2)
            .base(base)
            .highlight(hi),
        inner,
    );

    // Commit log — table with graph | OID | summary | author | time columns.
    let inner = pane(frame, main_cols[1], " Commit Log ", theme.border_inactive);
    frame.render_widget(
        SkeletonTable::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .rows(30)
            .columns(&[
                Constraint::Length(2),  // graph
                Constraint::Length(8),  // short OID
                Constraint::Fill(1),    // commit summary
                Constraint::Length(14), // author
                Constraint::Length(9),  // relative time
            ])
            .zebra(true)
            .base(base)
            .highlight(hi),
        inner,
    );

    // Diff pane — solid shimmer block.
    let inner = pane(frame, main_cols[2], " Diff ", theme.border_inactive);
    frame.render_widget(
        SkeletonBlock::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .base(base)
            .highlight(hi),
        inner,
    );

    // ── Staging area: unstaged | staged ─────────────────────────────
    let staging_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(outer[2]);

    let inner = pane(frame, staging_cols[0], " Unstaged ", theme.border_inactive);
    frame.render_widget(
        SkeletonList::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .items(5)
            .base(base)
            .highlight(hi),
        inner,
    );

    let inner = pane(frame, staging_cols[1], " Staged ", theme.border_inactive);
    frame.render_widget(
        SkeletonList::new(elapsed_ms)
            .mode(AnimationMode::Sweep)
            .items(5)
            .base(base)
            .highlight(hi),
        inner,
    );

    // Status bar — show the real one (spinner + any status message).
    crate::widgets::status_bar::render(app, frame, outer[3]);
}

/// Draw a titled bordered pane and return the inner [`Rect`] for the caller to fill.
fn pane(frame: &mut Frame, area: Rect, title: &str, border_color: ratatui::style::Color) -> Rect {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}
