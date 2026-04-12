//! Diff viewer panel — shows file diffs with colored hunks, monospace text,
//! and hunk headers highlighted.
//!
//! When a commit is selected and its diff contains multiple files, a clickable
//! file list is shown on the left side of the diff panel so the user can
//! switch between files.

use gitkraft_core::{DiffHunk, DiffInfo, DiffLine};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Font, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::theme::ThemeColors;

/// Render the diff viewer panel. If a diff is selected, render its hunks with
/// colored lines; otherwise show a placeholder message.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let c = state.colors();
    let tab = state.active_tab();

    match &tab.selected_diff {
        Some(diff) => {
            if tab.commit_diffs.len() > 1 {
                // Multiple files in this commit — show file list + divider + diff side by side.
                let file_list = commit_file_list(state, &c, state.diff_file_list_width);
                let divider = crate::widgets::divider::vertical_divider(
                    crate::state::DragTarget::DiffFileListRight,
                    &c,
                );
                let diff_panel = diff_content(diff, &c);

                let layout = row![file_list, divider, diff_panel]
                    .width(Length::Fill)
                    .height(Length::Fill);

                container(layout)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface_style)
                    .into()
            } else {
                // Single file (or staging diff) — show diff only.
                container(diff_content(diff, &c))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface_style)
                    .into()
            }
        }
        None => container(placeholder_view(&c))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into(),
    }
}

/// Clickable file list for the currently selected commit's diffs.
fn commit_file_list<'a>(state: &'a GitKraft, c: &ThemeColors, width: f32) -> Element<'a, Message> {
    let tab = state.active_tab();

    let header_icon = text('\u{F30A}') // file-diff
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(13)
        .color(c.accent);

    let header_text = text("Files").size(13).color(c.text_primary);

    let file_count = text(format!("({})", tab.commit_diffs.len()))
        .size(11)
        .color(c.muted);

    let header_row = row![
        header_icon,
        Space::with_width(4),
        header_text,
        Space::with_width(4),
        file_count,
    ]
    .align_y(Alignment::Center)
    .padding([6, 8]);

    let mut file_list_col = column![].spacing(1).width(Length::Fill);

    for diff in &tab.commit_diffs {
        let file_path_display = if diff.new_file.is_empty() {
            &diff.old_file
        } else {
            &diff.new_file
        };

        // Extract just the filename for a compact display
        let file_name = file_path_display
            .rsplit('/')
            .next()
            .unwrap_or(file_path_display);

        // Determine if this file is the currently selected one
        let is_selected = tab
            .selected_diff
            .as_ref()
            .map(|sel| sel.new_file == diff.new_file && sel.old_file == diff.old_file)
            .unwrap_or(false);

        let status_color = theme::status_color(&diff.status, c);

        let status_char = match diff.status {
            gitkraft_core::FileStatus::New | gitkraft_core::FileStatus::Untracked => "A",
            gitkraft_core::FileStatus::Modified | gitkraft_core::FileStatus::Typechange => "M",
            gitkraft_core::FileStatus::Deleted => "D",
            gitkraft_core::FileStatus::Renamed => "R",
            gitkraft_core::FileStatus::Copied => "C",
        };

        let status_badge = text(status_char)
            .size(11)
            .font(Font::MONOSPACE)
            .color(status_color);

        let name_color = if is_selected {
            c.text_primary
        } else {
            c.text_secondary
        };

        let name_label = text(file_name.to_string()).size(12).color(name_color);

        // Show parent directory as a subtle hint when names might be ambiguous
        let dir_hint: Element<'a, Message> = {
            let parent = file_path_display
                .rsplit_once('/')
                .map(|(dir, _)| dir)
                .unwrap_or("");
            if parent.is_empty() {
                Space::with_width(0).into()
            } else {
                // Show only the last directory component
                let short_dir = parent.rsplit('/').next().unwrap_or(parent);
                text(format!("{short_dir}/")).size(10).color(c.muted).into()
            }
        };

        let row_content = row![
            status_badge,
            Space::with_width(4),
            column![row![dir_hint, name_label].align_y(Alignment::Center),],
        ]
        .align_y(Alignment::Center)
        .padding([4, 8])
        .width(Length::Fill);

        let style_fn = if is_selected {
            theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
        } else {
            theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
        };

        let diff_clone = diff.clone();
        let file_btn = button(row_content)
            .padding(0)
            .width(Length::Fill)
            .style(theme::ghost_button)
            .on_press(Message::SelectDiff(diff_clone));

        let file_row = container(file_btn).width(Length::Fill).style(style_fn);

        file_list_col = file_list_col.push(file_row);
    }

    let scrollable_files = scrollable(file_list_col).height(Length::Fill);

    container(
        column![header_row, scrollable_files]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(width))
    .height(Length::Fill)
    .style(theme::sidebar_style)
    .into()
}

/// Placeholder shown when no diff is selected.
fn placeholder_view<'a>(c: &ThemeColors) -> Element<'a, Message> {
    let icon = text('\u{F30A}') // file-diff icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(32)
        .color(c.muted);

    let label = text("Select a commit or file to view diff")
        .size(14)
        .color(c.muted);

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
fn diff_content<'a>(diff: &DiffInfo, c: &ThemeColors) -> Element<'a, Message> {
    let file_path_display = if diff.new_file.is_empty() {
        diff.old_file.clone()
    } else {
        diff.new_file.clone()
    };

    let status_color = theme::status_color(&diff.status, c);

    let status_badge = text(format!(" {} ", diff.status))
        .size(12)
        .color(status_color)
        .font(Font::MONOSPACE);

    let file_label = text(file_path_display)
        .size(14)
        .color(c.text_primary)
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
            .color(c.muted)
            .font(Font::MONOSPACE);
        lines_col = lines_col.push(container(empty_msg).padding([8, 12]));
    } else {
        for hunk in &diff.hunks {
            lines_col = append_hunk_lines(lines_col, hunk, c);
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
    c: &ThemeColors,
) -> iced::widget::Column<'a, Message> {
    for line in &hunk.lines {
        let line_element = render_line(line, c);
        col = col.push(line_element);
    }
    col
}

/// Render a single [`DiffLine`] as a styled container with monospace text.
fn render_line<'a>(line: &DiffLine, c: &ThemeColors) -> Element<'a, Message> {
    match line {
        DiffLine::HunkHeader(header) => {
            let content = text(header.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(c.accent);

            container(content)
                .padding([4, 12])
                .width(Length::Fill)
                .style(theme::diff_hunk_style)
                .into()
        }

        DiffLine::Addition(content_str) => {
            let prefix = text("+").size(13).font(Font::MONOSPACE).color(c.green);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(c.green);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .style(theme::diff_add_style)
                .into()
        }

        DiffLine::Deletion(content_str) => {
            let prefix = text("-").size(13).font(Font::MONOSPACE).color(c.red);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(c.red);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .style(theme::diff_del_style)
                .into()
        }

        DiffLine::Context(content_str) => {
            let prefix = text(" ").size(13).font(Font::MONOSPACE).color(c.muted);

            let content = text(content_str.clone())
                .size(13)
                .font(Font::MONOSPACE)
                .color(c.text_secondary);

            container(row![prefix, Space::with_width(4), content])
                .padding([1, 12])
                .width(Length::Fill)
                .into()
        }
    }
}
