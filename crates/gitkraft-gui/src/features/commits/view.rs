//! Commit log view — scrollable list of commits with highlighted selection.
//!
//! Commit summaries are pre-truncated with "…" based on the actual available
//! pixel width so that each row stays on exactly one line — matching
//! GitKraken's behaviour.
//!
//! All loaded commits are rendered as direct children of the scrollable’s
//! column.  Iced’s scrollable clips the viewport and only draws the visible
//! portion, keeping rendering efficient.  Pagination is triggered via the
//! `on_scroll` callback when the user nears the bottom.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Row, Space};
use iced::{Alignment, Color, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::{GitKraft, RepoTab};
use crate::theme;
use crate::theme::ThemeColors;
use crate::view_utils;
use crate::view_utils::truncate_to_fit;

// ── ref-badge helpers ─────────────────────────────────────────────────────

/// Foreground (text + border) colour for a ref badge.
fn ref_fg(kind: &gitkraft_core::RefKind, c: &ThemeColors) -> iced::Color {
    match kind {
        gitkraft_core::RefKind::Head => c.accent,
        gitkraft_core::RefKind::LocalBranch => c.green,
        gitkraft_core::RefKind::RemoteBranch => c.yellow,
        gitkraft_core::RefKind::Tag => c.muted,
    }
}

/// Build a row of coloured badge pills for `refs`, capped at `max_badges`.
/// Returns `None` when the slice is empty so callers can skip the spacing.
fn ref_badges_row<'a>(
    refs: &'a [gitkraft_core::RefLabel],
    c: &ThemeColors,
    max_badges: usize,
) -> Option<Element<'a, Message>> {
    if refs.is_empty() {
        return None;
    }
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for (i, rf) in refs.iter().take(max_badges).enumerate() {
        let fg = ref_fg(&rf.kind, c);
        let bg = c.surface;
        let name = gitkraft_core::truncate_str(&rf.name, 22);
        let badge = container(text(name).size(10).color(fg).font(iced::Font::MONOSPACE))
            .padding([1, 5])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(bg)),
                border: iced::Border {
                    color: fg,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            });
        if i > 0 {
            items.push(Space::new().width(3).into());
        }
        items.push(badge.into());
    }
    Some(Row::with_children(items).align_y(Alignment::Center).into())
}

/// Estimated height of one commit row in pixels.
const ROW_HEIGHT: f32 = 26.0;

/// Per-tab stable scroll id — Iced maintains a separate scroll position for
/// each open tab so no programmatic `scroll_to` is needed on tab switches.
pub fn commit_log_scroll_id(tab_index: usize) -> iced::widget::Id {
    iced::widget::Id::from(format!("commit_log_{tab_index}"))
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
    author_width: f32,
    selected_range: &[usize],
) -> Element<'a, Message> {
    let commit = &tab.commits[idx];
    let is_selected = tab.selected_commit == Some(idx);

    // Badge: position in the selected range (1-based), or blank space
    let selection_badge: Element<'a, Message> =
        if let Some(pos) = selected_range.iter().position(|&i| i == idx) {
            container(
                text(format!("{}", pos + 1))
                    .size(10)
                    .font(iced::Font::MONOSPACE)
                    .color(c.accent),
            )
            .width(16)
            .center_x(iced::Length::Fixed(16.0))
            .into()
        } else {
            Space::new().width(16).into()
        };

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

    // Ref badges (branch / tag / HEAD) shown between the OID and the summary.
    let badges = ref_badges_row(&commit.refs, c, 3);
    // Rough pixel budget consumed by the badges so summary truncation stays accurate.
    let badges_overhead: f32 = if commit.refs.is_empty() {
        0.0
    } else {
        commit
            .refs
            .iter()
            .take(3)
            .map(|r| r.name.len().min(22) as f32 * 6.5 + 18.0)
            .sum::<f32>()
            + 8.0
    };

    // Use pre-computed display strings; fall back gracefully if out of sync.
    // Summary is read directly from the commit to avoid duplicating it.
    let summary_str = commit.summary.as_str();
    let (time_str, author_str) = tab
        .commit_display
        .get(idx)
        .map(|(t, a)| (t.as_str(), a.as_str()))
        .unwrap_or(("", commit.author_name.as_str()));

    // Pre-truncate with "…" so the full row stays on one line.
    let row_available = (available_summary_px - badges_overhead).max(40.0);
    let display_summary = truncate_to_fit(summary_str, row_available, 7.0);
    let summary_label = container(
        text(display_summary)
            .size(12)
            .color(c.text_primary)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(Length::Fill)
    .clip(true);

    let author_label = container(
        text(author_str)
            .size(11)
            .color(c.text_secondary)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(author_width)
    .clip(true);

    let time_label = container(
        text(time_str)
            .size(11)
            .color(c.muted)
            .wrapping(iced::widget::text::Wrapping::None),
    )
    .width(72)
    .clip(true);

    // Build the row: insert badges between OID and summary when present.
    let mut row_content = row![
        selection_badge,
        Space::new().width(2),
        graph_elem,
        oid_label,
        Space::new().width(6),
    ]
    .align_y(Alignment::Center);

    if let Some(b) = badges {
        row_content = row_content.push(b).push(Space::new().width(4));
    }

    let row_content = row_content
        .push(summary_label)
        .push(Space::new().width(8))
        .push(author_label)
        .push(Space::new().width(8))
        .push(time_label)
        .padding([3, 8]);

    let is_in_range = selected_range.contains(&idx);
    let style_fn = if is_selected {
        theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
    } else if is_in_range {
        theme::highlight_row_style as fn(&iced::Theme) -> iced::widget::container::Style
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

    let multi_count = tab.selected_commits.len();
    let commit_count: iced::widget::Text<'_, iced::Theme> = if multi_count > 1 {
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

    // ── Commit rows ───────────────────────────────────────────────────────
    //
    // All loaded commits are rendered as direct children of the column.
    // Iced's scrollable handles viewport clipping and only draws the
    // visible portion, so this is efficient even for large lists.
    // Using Space-based virtual scrolling caused feedback loops between
    // the stored scroll offset and Iced's internal scroll state, resulting
    // in thumb jitter and scroll jumps.

    let total = tab.commits.len();

    // Author column scales with commit log width: ~15% of log width, clamped to [90, 180].
    let author_width = (state.commit_log_width * 0.15).clamp(90.0, 180.0);

    // Available px for the summary column:
    // commit_log_width minus graph (~30) + oid (~56) + spaces + author + time (72) + padding (16).
    let fixed_overhead = 30.0 + 56.0 + 22.0 + author_width + 72.0 + 16.0;
    let available_summary_px = (state.commit_log_width - fixed_overhead).max(40.0);

    let selected_range = tab.selected_commits.as_slice();

    let mut list_col = column![].width(Length::Fill);

    for idx in 0..total {
        list_col = list_col.push(commit_row_element(
            tab,
            idx,
            &c,
            available_summary_px,
            author_width,
            selected_range,
        ));
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

    view_utils::surface_panel(content, Length::Fill)
}
