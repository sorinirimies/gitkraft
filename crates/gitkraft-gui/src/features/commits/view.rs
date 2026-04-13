//! Commit log view — scrollable list of commits with highlighted selection.
//!
//! Each commit row shows: graph │ short OID │ summary │ author │ relative time.
//! The currently selected row gets a highlighted background.
//!
//! Uses `keyed_column` so that Iced can diff the widget tree efficiently when
//! the list hasn't changed — this avoids rebuilding hundreds of row widgets
//! every single frame.

use iced::widget::{button, column, container, keyed_column, row, scrollable, text, Row, Space};
use iced::{Alignment, Color, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Build a small `Row` of individually-coloured text elements representing one
/// row of the commit graph.
///
/// `graph_colors` is the per-theme palette of 8 lane colours obtained from
/// [`ThemeColors::graph_colors`].
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

    // Collect pass-through edges per column and crossing edge info.
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

/// Render the commit log panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F293}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

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

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into()
    } else {
        // Use keyed_column so Iced can diff the tree by a stable key
        // instead of rebuilding all rows from scratch every frame.
        // We use the enumeration index as the key (Copy + PartialEq).
        let list = keyed_column(tab.commits.iter().enumerate().map(|(idx, commit)| {
            let key = idx;

            let is_selected = tab.selected_commit == Some(idx);

            // ── Graph column ──────────────────────────────────
            let graph_elem: Element<'_, Message> = if let Some(grow) = tab.graph_rows.get(idx) {
                graph_cell(grow, &c.graph_colors).into()
            } else {
                text("").into()
            };

            let oid_label = text(commit.short_oid.as_str())
                .size(12)
                .color(c.accent)
                .font(iced::Font::MONOSPACE);

            let summary_text = if commit.summary.chars().count() > 60 {
                let truncated: String = commit.summary.chars().take(59).collect();
                format!("{truncated}…")
            } else {
                commit.summary.clone()
            };
            let summary_label = text(summary_text).size(12).color(c.text_primary);

            let author_label = text(commit.author_name.as_str())
                .size(11)
                .color(c.text_secondary);

            let time_str = gitkraft_core::utils::relative_time(commit.time);
            let time_label = text(time_str).size(11).color(c.muted);

            let row_content = row![
                graph_elem,
                oid_label,
                Space::with_width(6),
                summary_label,
                Space::with_width(Length::Fill),
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

            let row_container = container(
                button(row_content)
                    .padding(0)
                    .width(Length::Fill)
                    .on_press(Message::SelectCommit(idx))
                    .style(theme::ghost_button),
            )
            .width(Length::Fill)
            .style(style_fn);

            let element: Element<'_, Message> = row_container.into();
            (key, element)
        }))
        .width(Length::Fill);

        let content = column![header_row, scrollable(list).height(Length::Fill),]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into()
    }
}
