//! Commit log view — GitKraken-style layout with Canvas-drawn graph.
//!
//! The graph uses the reusable `CommitGraph` widget (`widgets::commit_graph`)
//! which draws the entire branch topology as one continuous canvas — producing
//! smooth, connected subway-map style lines.
//!
//! Layout inside the scrollable:
//! ```text
//! row![
//!   ref_badges_column,   // fixed-width column of branch/tag badges
//!   CommitGraph canvas,  // single tall canvas for the entire graph
//!   messages_column,     // commit summaries + author + date
//! ]
//! ```
//! All three scroll together because they share the same scrollable parent.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Row, Space};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::icons;
use crate::message::Message;
use crate::state::{GitKraft, RepoTab};
use crate::theme;
use crate::theme::ThemeColors;
use crate::view_utils;
use crate::view_utils::truncate_to_fit;
use crate::widgets::commit_graph::CommitGraph;

// ── constants ─────────────────────────────────────────────────────────────────

/// Height of one commit row in pixels.
const ROW_HEIGHT: f32 = 26.0;

/// Fixed width of the author sub-column.
const AUTHOR_COLUMN_PX: f32 = 110.0;

/// Fixed width of the date/time sub-column.
const TIME_COLUMN_PX: f32 = 72.0;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Per-tab stable scroll id.
pub fn commit_log_scroll_id(tab_index: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("commit_log_{tab_index}"))
}

/// Foreground colour for a ref badge.
fn ref_fg(kind: &gitkraft_core::RefKind, c: &ThemeColors) -> Color {
    match kind {
        gitkraft_core::RefKind::Head => c.accent,
        gitkraft_core::RefKind::LocalBranch => c.green,
        gitkraft_core::RefKind::RemoteBranch => c.yellow,
        gitkraft_core::RefKind::Tag => c.muted,
    }
}

/// Build ref badges for one row in the BRANCH/TAG column.
fn ref_badges<'a>(refs: &'a [gitkraft_core::RefLabel], c: &ThemeColors) -> Element<'a, Message> {
    if refs.is_empty() {
        return Space::new().into();
    }
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for (i, rf) in refs.iter().take(2).enumerate() {
        let fg = ref_fg(&rf.kind, c);
        let bg = c.surface;
        let name = gitkraft_core::truncate_str(&rf.name, 14);
        let badge = container(text(name).size(9).color(fg).font(iced::Font::MONOSPACE))
            .padding([1, 4])
            .style(move |_: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border {
                    color: fg,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            });
        if i > 0 {
            items.push(Space::new().width(2).into());
        }
        items.push(badge.into());
    }
    Row::with_children(items).align_y(Alignment::Center).into()
}

/// A thin draggable divider for the column header (fixed height, won't stretch).
fn col_divider<'a>(target: crate::state::DragTarget, c: &ThemeColors) -> Element<'a, Message> {
    let border_color = c.border;
    let hit = container(iced::widget::rule::vertical(1).style(move |_: &Theme| {
        iced::widget::rule::Style {
            color: border_color,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
            snap: true,
        }
    }))
    .width(6)
    .height(Length::Fixed(20.0))
    .center_x(6)
    .center_y(Length::Fixed(20.0));

    mouse_area(hit)
        .on_press(Message::PaneDragStart(target, 0.0))
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

// ── single message row ────────────────────────────────────────────────────────

/// Build the message portion of one commit row (summary + author + date).
fn message_row<'a>(
    tab: &'a RepoTab,
    idx: usize,
    c: &ThemeColors,
    available_px: f32,
    selected_range: &[usize],
) -> Element<'a, Message> {
    let commit = &tab.commits[idx];
    let is_selected = tab.selected_commit == Some(idx);
    let is_in_range = selected_range.contains(&idx);

    let summary_str = commit.summary.as_str();
    let display_summary = truncate_to_fit(summary_str, available_px, 7.0);

    let summary_label = text(display_summary)
        .size(12)
        .color(c.text_primary)
        .wrapping(iced::widget::text::Wrapping::None);

    let (time_str, author_str) = tab
        .commit_display
        .get(idx)
        .map(|(t, a)| (t.as_str(), a.as_str()))
        .unwrap_or(("", commit.author_name.as_str()));

    let author_label = container(
        text(author_str)
            .size(11)
            .color(c.text_secondary)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fixed(AUTHOR_COLUMN_PX))
    .clip(true);

    let time_label = container(
        text(time_str)
            .size(11)
            .color(c.muted)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fixed(TIME_COLUMN_PX))
    .clip(true);

    let row_content = row![
        container(summary_label).width(Length::Fill).clip(true),
        Space::new().width(6),
        author_label,
        Space::new().width(6),
        time_label,
    ]
    .align_y(Alignment::Center)
    .padding([0, 4]);

    let style_fn = if is_selected {
        theme::selected_row_style as fn(&Theme) -> iced::widget::container::Style
    } else if is_in_range {
        theme::highlight_row_style as fn(&Theme) -> iced::widget::container::Style
    } else {
        theme::surface_style as fn(&Theme) -> iced::widget::container::Style
    };

    mouse_area(
        container(
            button(row_content)
                .padding(0)
                .width(Length::Fill)
                .on_press(Message::SelectCommit(idx))
                .style(theme::ghost_button),
        )
        .width(Length::Fill)
        .height(Length::Fixed(ROW_HEIGHT))
        .clip(true)
        .style(style_fn),
    )
    .on_right_press(Message::OpenCommitContextMenu(idx))
    .into()
}

// ── main view ─────────────────────────────────────────────────────────────────

/// Render the commit log panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    // ── Header ────────────────────────────────────────────────────────────
    let header_icon = icon!(icons::CLOCK, 14, c.accent);
    let header_text = text("Commit Log").size(14).color(c.text_primary);

    let multi_count = tab.selected_commits.len();
    let commit_count: iced::widget::Text<'_, Theme> = if multi_count > 1 {
        text(format!("({} selected)", multi_count))
            .size(12)
            .color(c.accent)
    } else {
        text(format!("({})", tab.commits.len()))
            .size(12)
            .color(c.muted)
    };

    let header_row = row![
        header_icon,
        Space::new().width(6),
        header_text,
        Space::new().width(6),
        commit_count,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    if tab.commits.is_empty() {
        let empty_msg = text("No commits yet.").size(14).color(c.muted);
        let content = column![
            header_row,
            container(empty_msg)
                .width(Length::Fill)
                .padding(20)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);
        return view_utils::surface_panel(content, Length::Fill);
    }

    // ── Column widths ─────────────────────────────────────────────────────
    let ref_col_w = state.commit_ref_width;
    let graph_col_w = state.commit_graph_width;

    // Each col_divider is 6px wide; two dividers = 12px.
    const DIVIDER_W: f32 = 6.0;
    let fixed_cols =
        ref_col_w + DIVIDER_W + graph_col_w + DIVIDER_W + AUTHOR_COLUMN_PX + TIME_COLUMN_PX + 30.0;
    let msg_available_px = (state.commit_log_width - fixed_cols).max(60.0);

    // ── Column headers with draggable dividers ────────────────────────────
    let col_header = row![
        container(
            text("BRANCH / TAG")
                .size(10)
                .color(c.muted)
                .font(iced::Font::MONOSPACE)
        )
        .width(Length::Fixed(ref_col_w))
        .clip(true),
        col_divider(crate::state::DragTarget::CommitRefColumnRight, &c),
        container(
            text("GRAPH")
                .size(10)
                .color(c.muted)
                .font(iced::Font::MONOSPACE)
        )
        .width(Length::Fixed(graph_col_w))
        .clip(true),
        col_divider(crate::state::DragTarget::CommitGraphColumnRight, &c),
        text("COMMIT MESSAGE")
            .size(10)
            .color(c.muted)
            .font(iced::Font::MONOSPACE),
    ]
    .align_y(Alignment::Center)
    .padding([4, 4])
    .width(Length::Fill);

    // ── Virtual scroll window ─────────────────────────────────────────────
    //
    // Only columns 1 (ref badges) and 3 (messages) are virtualised.
    // Column 2 (graph canvas) is a single cached Canvas that Iced clips
    // to the viewport automatically — no per-frame cost.
    //
    // Space widgets above and below the visible rows maintain the correct
    // total height so the scrollbar stays proportional.

    let total = tab.commits.len();
    let selected_range = tab.selected_commits.as_slice();

    const OVERSCAN: usize = 10;
    const VISIBLE_ROWS: usize = 60;

    let scroll_y = tab.commit_scroll_offset;
    let first_row = ((scroll_y / ROW_HEIGHT) as usize).min(total.saturating_sub(1));
    let first = first_row.saturating_sub(OVERSCAN);
    let last = (first + VISIBLE_ROWS + 2 * OVERSCAN).min(total);

    let top_space = first as f32 * ROW_HEIGHT;
    let bottom_space = (total - last) as f32 * ROW_HEIGHT;

    // Column 1: ref badges (virtualised)
    let mut refs_col = column![].width(Length::Fixed(ref_col_w));
    if top_space > 0.0 {
        refs_col = refs_col.push(Space::new().width(ref_col_w).height(top_space));
    }
    for idx in first..last {
        refs_col = refs_col.push(
            container(ref_badges(&tab.commits[idx].refs, &c))
                .width(Length::Fixed(ref_col_w))
                .height(Length::Fixed(ROW_HEIGHT))
                .clip(true)
                .center_y(Length::Fixed(ROW_HEIGHT)),
        );
    }
    if bottom_space > 0.0 {
        refs_col = refs_col.push(Space::new().width(ref_col_w).height(bottom_space));
    }

    // Column 2: graph canvas (same window as other columns for alignment).
    // Compute the full graph content width from ALL rows (stable during scroll).
    let max_lanes = tab
        .graph_rows
        .iter()
        .map(|r| r.width)
        .max()
        .unwrap_or(1)
        .max(1);
    let graph_content_w = max_lanes as f32 * crate::widgets::commit_graph::LANE_W + 4.0;

    let graph_canvas = CommitGraph {
        visible_rows: tab.graph_rows[first..last].to_vec(),
        offset: first,
        colors: c.graph_colors,
        row_height: ROW_HEIGHT,
        bg_color: c.bg,
        content_width: graph_content_w,
    }
    .view(graph_col_w);

    // Wrap with the SAME Space padding as the other columns.
    let mut graph_col = column![].width(Length::Fixed(graph_col_w));
    if top_space > 0.0 {
        graph_col = graph_col.push(Space::new().width(graph_col_w).height(top_space));
    }
    graph_col = graph_col.push(graph_canvas);
    if bottom_space > 0.0 {
        graph_col = graph_col.push(Space::new().width(graph_col_w).height(bottom_space));
    }

    // Column 3: commit messages (virtualised)
    let mut msgs_col = column![].width(Length::Fill);
    if top_space > 0.0 {
        msgs_col = msgs_col.push(Space::new().height(top_space));
    }
    for idx in first..last {
        msgs_col = msgs_col.push(message_row(tab, idx, &c, msg_available_px, selected_range));
    }
    if bottom_space > 0.0 {
        msgs_col = msgs_col.push(Space::new().height(bottom_space));
    }

    // Footer (loading / end-of-history)
    let footer: Element<'_, Message> = if tab.is_loading_more_commits {
        container(text("Loading more commits…").size(12).color(c.muted))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding([10, 0])
            .into()
    } else if !tab.has_more_commits {
        container(text("— end of history —").size(11).color(c.muted))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding([10, 0])
            .into()
    } else {
        Space::new().height(0).into()
    };

    // Assemble: columns with spacers matching the header divider widths
    let scrollable_content = column![
        row![
            refs_col,
            Space::new().width(DIVIDER_W),
            graph_col,
            Space::new().width(DIVIDER_W),
            msgs_col,
        ]
        .width(Length::Fill),
        footer,
    ]
    .width(Length::Fill);

    let commit_scroll = scrollable(scrollable_content)
        .height(Length::Fill)
        .id(commit_log_scroll_id(state.active_tab))
        .on_scroll(|vp| Message::CommitLogScrolled(vp.absolute_offset().y, vp.relative_offset().y))
        .direction(view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

    let content = column![header_row, col_header, commit_scroll]
        .width(Length::Fill)
        .height(Length::Fill);

    view_utils::surface_panel(content, Length::Fill)
}
