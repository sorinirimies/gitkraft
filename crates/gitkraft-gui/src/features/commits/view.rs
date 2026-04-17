//! Commit log view — scrollable list of commits with highlighted selection.
//!
//! Commit summaries are pre-truncated with "…" based on the actual available
//! pixel width so that each row stays on exactly one line — matching
//! GitKraken's behaviour.

//
// Renders only the rows currently visible in the viewport plus a small
// overscan buffer.  Space widgets above and below maintain the correct
// total scroll height so the scrollbar behaves naturally.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Row, Space};
use iced::{Alignment, Color, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::{GitKraft, RepoTab};
use crate::theme;
use crate::theme::ThemeColors;
use crate::view_utils;
use crate::view_utils::truncate_to_fit;

/// Estimated height of one commit row in pixels.  Used for virtual scrolling.
/// A slight over- or under-estimate only affects scrollbar thumb precision,
/// not correctness of the rendered content.
const ROW_HEIGHT: f32 = 26.0;

/// Rows rendered above and below the visible window (avoids pop-in during
/// fast scrolling).
const OVERSCAN: usize = 8;

/// Assumed visible rows (covers a 1300 px tall viewport at ROW_HEIGHT).
/// Making this generous costs almost nothing — we cap at `total` anyway.
const VISIBLE_ROWS: usize = 50;

/// Per-tab stable scroll id — Iced maintains a separate scroll position for
/// each open tab so no programmatic `scroll_to` is needed on tab switches.
pub fn commit_log_scroll_id(tab_index: usize) -> scrollable::Id {
    scrollable::Id::new(format!("commit_log_{tab_index}"))
}

// ── graph_cell ────────────────────────────────────────────────────────────────

/// Build a small `Row` of individually-coloured text elements representing one
/// row of the commit graph.
fn graph_cell<'a>(
    graph_row: &gitkraft_core::GraphRow,
    graph_colors: &[Color; 8],
) -> Row<'a, Message> {
    let width = graph_row.width;
    let len = graph_colors.len();

    if width == 0 {
        return Row::new().push(
            text("● ")
                .font(iced::Font::MONOSPACE)
                .size(12)
                .color(graph_colors[graph_row.node_color % len]),
        );
    }

    let mut column_passthrough: Vec<Option<usize>> = vec![None; width];
    let mut has_left_cross = false;
    let mut has_right_cross = false;
    let mut left_cross_color: usize = 0;
    let mut right_cross_color: usize = 0;
    let mut cross_left_col: usize = graph_row.node_column;
    let mut cross_right_col: usize = graph_row.node_column;

    for edge in &graph_row.edges {
        if edge.from_column == edge.to_column {
            column_passthrough[edge.to_column] = Some(edge.color_index);
        } else {
            let target = edge.to_column;
            if target < graph_row.node_column {
                has_left_cross = true;
                left_cross_color = edge.color_index;
                if target < cross_left_col {
                    cross_left_col = target;
                }
            } else if target > graph_row.node_column {
                has_right_cross = true;
                right_cross_color = edge.color_index;
                if target > cross_right_col {
                    cross_right_col = target;
                }
            }
        }
    }

    let mut cells: Vec<Element<'a, Message>> = Vec::with_capacity(width);

    for col in 0..width {
        if col == graph_row.node_column {
            let color = graph_colors[graph_row.node_color % len];
            cells.push(
                text("● ")
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .color(color)
                    .into(),
            );
        } else if let Some(ci) = column_passthrough.get(col).copied().flatten() {
            let in_left = has_left_cross && col >= cross_left_col && col < graph_row.node_column;
            let in_right = has_right_cross && col > graph_row.node_column && col <= cross_right_col;

            if in_left || in_right {
                let cross_ci = if in_left {
                    left_cross_color
                } else {
                    right_cross_color
                };
                cells.push(
                    text("├─")
                        .font(iced::Font::MONOSPACE)
                        .size(12)
                        .color(graph_colors[cross_ci % len])
                        .into(),
                );
            } else {
                cells.push(
                    text("│ ")
                        .font(iced::Font::MONOSPACE)
                        .size(12)
                        .color(graph_colors[ci % len])
                        .into(),
                );
            }
        } else {
            let in_left = has_left_cross && col >= cross_left_col && col < graph_row.node_column;
            let in_right = has_right_cross && col > graph_row.node_column && col <= cross_right_col;

            if in_left {
                let color = graph_colors[left_cross_color % len];
                if col == cross_left_col {
                    cells.push(
                        text("╭─")
                            .font(iced::Font::MONOSPACE)
                            .size(12)
                            .color(color)
                            .into(),
                    );
                } else {
                    cells.push(
                        text("──")
                            .font(iced::Font::MONOSPACE)
                            .size(12)
                            .color(color)
                            .into(),
                    );
                }
            } else if in_right {
                let color = graph_colors[right_cross_color % len];
                if col == cross_right_col {
                    cells.push(
                        text("─╮")
                            .font(iced::Font::MONOSPACE)
                            .size(12)
                            .color(color)
                            .into(),
                    );
                } else {
                    cells.push(
                        text("──")
                            .font(iced::Font::MONOSPACE)
                            .size(12)
                            .color(color)
                            .into(),
                    );
                }
            } else {
                cells.push(text("  ").font(iced::Font::MONOSPACE).size(12).into());
            }
        }
    }

    Row::with_children(cells).align_y(Alignment::Center)
}

// ── single row element ────────────────────────────────────────────────────────

/// Build the widget for a single commit row.
fn commit_row_element<'a>(
    tab: &'a RepoTab,
    idx: usize,
    c: &ThemeColors,
    available_summary_px: f32,
) -> Element<'a, Message> {
    let commit = &tab.commits[idx];
    let is_selected = tab.selected_commit == Some(idx);

    // Graph column
    let graph_elem: Element<'_, Message> = if let Some(grow) = tab.graph_rows.get(idx) {
        graph_cell(grow, &c.graph_colors).into()
    } else {
        text("").into()
    };

    let oid_label = text(commit.short_oid.as_str())
        .size(12)
        .color(c.accent)
        .font(iced::Font::MONOSPACE);

    // Use pre-computed display strings; fall back gracefully if out of sync.
    let (summary_str, time_str, author_str) = tab
        .commit_display
        .get(idx)
        .map(|(s, t, a)| (s.as_str(), t.as_str(), a.as_str()))
        .unwrap_or((commit.summary.as_str(), "", commit.author_name.as_str()));

    // Pre-truncate with "…" so the full row stays on one line.
    let display_summary = truncate_to_fit(summary_str, available_summary_px, 7.0);
    let summary_label = container(
        text(display_summary)
            .size(12)
            .color(c.text_primary)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fill)
    .clip(true);

    // Fixed-width columns prevent author / time from being squeezed to zero
    // and wrapping character-by-character.  Text is pre-truncated so it fits.
    let author_label = container(
        text(author_str)
            .size(11)
            .color(c.text_secondary)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(90)
    .clip(true);

    let time_label = container(
        text(time_str)
            .size(11)
            .color(c.muted)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(72)
    .clip(true);

    let row_content = row![
        graph_elem,
        oid_label,
        Space::with_width(6),
        summary_label,
        Space::with_width(8),
        author_label,
        Space::with_width(8),
        time_label,
    ]
    .align_y(Alignment::Center)
    .padding([3, 8]);

    let style_fn = if is_selected {
        theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
    } else {
        theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
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

// ── view ─────────────────────────────────────────────────────────────────────

/// Render the commit log panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = icon!(icons::CLOCK, 14, c.accent);

    let header_text = text("Commit Log").size(14).color(c.text_primary);

    let commit_count = text(format!("({})", tab.commits.len()))
        .size(12)
        .color(c.muted);

    let header_row = row![
        header_icon,
        Space::with_width(6),
        header_text,
        Space::with_width(6),
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

        return container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into();
    }

    // ── Virtual scroll window ─────────────────────────────────────────────
    //
    // Only the rows visible in the viewport (plus OVERSCAN above/below) are
    // constructed as widgets.  The remaining space is filled with two Space
    // widgets so the scrollable keeps the correct total height and the
    // scrollbar thumb stays proportional.

    let total = tab.commits.len();
    let scroll_y = tab.commit_scroll_offset;

    let first = ((scroll_y / ROW_HEIGHT) as usize).saturating_sub(OVERSCAN);
    let last = (first + VISIBLE_ROWS + 2 * OVERSCAN).min(total);

    let top_space = first as f32 * ROW_HEIGHT;
    let bottom_space = (total - last) as f32 * ROW_HEIGHT;

    let mut list_col = column![].width(Length::Fill);

    if top_space > 0.0 {
        list_col = list_col.push(Space::with_height(top_space));
    }

    // Available px for the summary column:
    // commit_log_width minus graph (~30) + oid (~56) + spaces + author (90)
    // + time (72) + row padding (16) ≈ 280 px fixed overhead.
    let available_summary_px = (state.commit_log_width - 280.0).max(40.0);

    for idx in first..last {
        list_col = list_col.push(commit_row_element(tab, idx, &c, available_summary_px));
    }

    if bottom_space > 0.0 {
        list_col = list_col.push(Space::with_height(bottom_space));
    }

    // Loading spinner shown while a background fetch is in progress.
    if tab.is_loading_more_commits {
        list_col = list_col.push(
            container(text("Loading more commits…").size(12).color(c.muted))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding([10, 0]),
        );
    }
    // End-of-history marker once all commits are loaded.
    if !tab.has_more_commits {
        list_col = list_col.push(
            container(text("— end of history —").size(11).color(c.muted))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding([10, 0]),
        );
    }

    let commit_scroll = scrollable(list_col)
        .height(Length::Fill)
        .id(commit_log_scroll_id(state.active_tab))
        .on_scroll(|vp| Message::CommitLogScrolled(vp.absolute_offset().y, vp.relative_offset().y))
        .direction(view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

    let content = column![header_row, commit_scroll]
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface_style)
        .into()
}
