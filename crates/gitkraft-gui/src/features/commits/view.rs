//! Commit log view — scrollable list of commits with highlighted selection.
//!
//! Each commit row shows: graph │ short OID │ summary │ author │ relative time.
//! The currently selected row gets a highlighted background.

use iced::widget::{button, column, container, row, scrollable, text, Row, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Eight colours cycled by `color_index % GRAPH_COLORS.len()`.
const GRAPH_COLORS: [iced::Color; 8] = [
    iced::Color {
        r: 0.0,
        g: 0.8,
        b: 0.0,
        a: 1.0,
    }, // green
    iced::Color {
        r: 0.0,
        g: 0.8,
        b: 0.8,
        a: 1.0,
    }, // cyan
    iced::Color {
        r: 0.8,
        g: 0.0,
        b: 0.8,
        a: 1.0,
    }, // magenta
    iced::Color {
        r: 0.9,
        g: 0.9,
        b: 0.0,
        a: 1.0,
    }, // yellow
    iced::Color {
        r: 0.3,
        g: 0.5,
        b: 1.0,
        a: 1.0,
    }, // blue
    iced::Color {
        r: 1.0,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    }, // red
    iced::Color {
        r: 0.5,
        g: 1.0,
        b: 0.5,
        a: 1.0,
    }, // light green
    iced::Color {
        r: 0.5,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    }, // light cyan
];

/// Build a small `Row` of individually-coloured text elements representing one
/// row of the commit graph.
fn graph_cell(graph_row: &gitkraft_core::GraphRow) -> Row<'_, Message> {
    let width = graph_row.width;
    let mut cells: Vec<Element<'_, Message>> = Vec::with_capacity(width);

    for col in 0..width {
        if col == graph_row.node_column {
            // Commit node dot
            let color = GRAPH_COLORS[graph_row.node_color % GRAPH_COLORS.len()];
            cells.push(
                text("● ")
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .color(color)
                    .into(),
            );
        } else if let Some(edge) = graph_row
            .edges
            .iter()
            .find(|e| e.from_column == col && e.to_column == col)
        {
            // Passing-through lane
            let color = GRAPH_COLORS[edge.color_index % GRAPH_COLORS.len()];
            cells.push(
                text("│ ")
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .color(color)
                    .into(),
            );
        } else {
            // Empty column
            cells.push(text("  ").font(iced::Font::MONOSPACE).size(12).into());
        }
    }

    Row::with_children(cells).align_y(Alignment::Center)
}

/// Render the commit log panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let header_icon = text('\u{F293}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let header_text = text("Commit Log").size(14).color(theme::TEXT_PRIMARY);

    let commit_count = text(format!("({})", state.commits.len()))
        .size(12)
        .color(theme::MUTED);

    let header_row = row![
        header_icon,
        Space::with_width(6),
        header_text,
        Space::with_width(6),
        commit_count,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    if state.commits.is_empty() {
        let empty_msg = text("No commits yet.").size(14).color(theme::MUTED);

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
        let commit_rows: Vec<Element<'_, Message>> = state
            .commits
            .iter()
            .enumerate()
            .map(|(idx, commit)| {
                let is_selected = state.selected_commit == Some(idx);

                // ── Graph column ──────────────────────────────────────
                let graph_elem: Element<'_, Message> = if let Some(grow) = state.graph_rows.get(idx)
                {
                    graph_cell(grow).into()
                } else {
                    text("").into()
                };

                let oid_label = text(commit.short_oid.as_str())
                    .size(12)
                    .color(theme::ACCENT)
                    .font(iced::Font::MONOSPACE);

                let sep1 = text("│").size(12).color(theme::BORDER);
                let sep2 = text("│").size(12).color(theme::BORDER);

                let summary_text = if commit.summary.len() > 60 {
                    format!("{}…", &commit.summary[..59])
                } else {
                    commit.summary.clone()
                };
                let summary_label = text(summary_text).size(12).color(theme::TEXT_PRIMARY);

                let author_label = text(commit.author_name.as_str())
                    .size(11)
                    .color(theme::TEXT_SECONDARY);

                let time_str = gitkraft_core::utils::relative_time(commit.time);
                let time_label = text(time_str).size(11).color(theme::MUTED);

                let row_content = row![
                    graph_elem,
                    oid_label,
                    Space::with_width(6),
                    sep1,
                    Space::with_width(6),
                    summary_label,
                    Space::with_width(Length::Fill),
                    author_label,
                    Space::with_width(8),
                    sep2,
                    Space::with_width(8),
                    time_label,
                ]
                .align_y(Alignment::Center)
                .padding([4, 10]);

                let style_fn = if is_selected {
                    theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
                } else {
                    theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
                };

                let row_container = container(
                    button(row_content)
                        .padding(0)
                        .width(Length::Fill)
                        .on_press(Message::SelectCommit(idx)),
                )
                .width(Length::Fill)
                .style(style_fn);

                row_container.into()
            })
            .collect();

        let mut list_col = column![].spacing(1).width(Length::Fill);
        for row_el in commit_rows {
            list_col = list_col.push(row_el);
        }

        let content = column![header_row, scrollable(list_col).height(Length::Fill),]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into()
    }
}
