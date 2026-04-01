//! Diff viewer panel — shows file diffs with colored hunks, monospace text,
//! and hunk headers highlighted.

use gitkraft_core::{DiffHunk, DiffInfo, DiffLine};
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Font, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the diff viewer panel. If a diff is selected, render its hunks with
/// colored lines; otherwise show a placeholder message.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let content: Element<'_, Message> = match &state.selected_diff {
        Some(diff) => diff_content(diff),
        None => placeholder_view(),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface_style)
        .into()
}

/// Placeholder shown when no diff is selected.
fn placeholder_view<'a>() -> Element<'a, Message> {
    let icon = text('\u{F30A}') // file-diff icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(32)
        .color(theme::MUTED);

    let label = text("Select a commit or file to view diff")
        .size(14)
        .color(theme::MUTED);

    container(
        column![icon, Space::with_height(8), label]
            .spacing(4)
            .align_x(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Render the full diff content for a single [`DiffInfo`].
fn diff_content<'a>(diff: &DiffInfo) -> Element<'a, Message> {
    let file_path_display = if diff.new_file.is_empty() {
        diff.old_file.clone()
    } else {
        diff.new_file.clone()
    };

    let status_color = theme::status_color(&diff.status);

    let status_badge = text(format!(" {} ", diff.status))
        .size(12)
        .color(status_color)
        .font(Font::MONOSPACE);

    let file_label = text(file_path_display)
        .size(14)
        .color(theme::TEXT_PRIMARY)
        .font(Font::MONOSPACE);

    let file_header = container(
        row![status_badge, Space::with_width(8), file_label].align_y(iced::Alignment::Center),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .style(theme::header_style);

    let mut lines_col = column![].width(Length::Fill);

    if diff.hunks.is_empty() {
        let empty_msg = text("No diff content available.")
            .size(13)
            .color(theme::MUTED)
            .font(Font::MONOSPACE);
        lines_col = lines_col.push(container(empty_msg).padding([8, 12]));
    } else {
        for hunk in &diff.hunks {
            lines_col = append_hunk_lines(lines_col, hunk);
        }
    }

    let scrollable_content = scrollable(lines_col).height(Length::Fill);

    column![file_header, scrollable_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Append all lines from a single [`DiffHunk`] into the column.
fn append_hunk_lines<'a>(
    mut col: iced::widget::Column<'a, Message>,
    hunk: &DiffHunk,
) -> iced::widget::Column<'a, Message> {
    for line in &hunk.lines {
        let line_element = render_line(line);
        col = col.push(line_element);
    }
    col
}

/// Render a single [`DiffLine`] as a styled container with monospace text.
fn render_line<'a>(line: &DiffLine) -> Element<'a, Message> {
    match line {
        DiffLine::HunkHeader(header) => {
            let content = text(header.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(theme::ACCENT);

            container(content)
                .padding([4, 12])
                .width(Length::Fill)
                .style(theme::diff_hunk_style)
                .into()
        }

        DiffLine::Addition(content_str) => {
            let prefix = text("+").size(13).font(Font::MONOSPACE).color(theme::GREEN);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(theme::GREEN);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .style(theme::diff_add_style)
                .into()
        }

        DiffLine::Deletion(content_str) => {
            let prefix = text("-").size(13).font(Font::MONOSPACE).color(theme::RED);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(theme::RED);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .style(theme::diff_del_style)
                .into()
        }

        DiffLine::Context(content_str) => {
            let prefix = text(" ").size(13).font(Font::MONOSPACE).color(theme::MUTED);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(theme::TEXT_SECONDARY);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .into()
        }
    }
}
