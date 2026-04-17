//! Diff viewer panel — shows file diffs with colored hunks, monospace text,
//! and hunk headers highlighted.
//!
//! When a commit is selected and its diff contains multiple files, a clickable
//! file list is shown on the left side of the diff panel so the user can
//! switch between files.
//!
//! The diff content uses virtual scrolling: only the lines that fall within
//! (or near) the visible viewport are materialised as widgets, keeping the
//! widget tree small even for multi-thousand-line diffs.

use gitkraft_core::{DiffInfo, DiffLine};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Font, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::theme::ThemeColors;
use crate::view_utils;

/// Estimated height of one diff line in pixels.
const DIFF_LINE_HEIGHT: f32 = 22.0;
/// Lines rendered above and below the visible window.
const DIFF_OVERSCAN: usize = 20;
/// Assumed visible lines (covers a tall viewport).
const DIFF_VISIBLE_LINES: usize = 60;

/// Render the diff viewer panel. If a diff is selected, render its hunks with
/// colored lines; otherwise show a placeholder message.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let c = state.colors();
    let tab = state.active_tab();

    match &tab.selected_diff {
        Some(diff) => {
            if tab.commit_files.len() > 1 {
                // Multiple files in this commit — show file list + divider + diff side by side.
                let file_list = commit_file_list(state, &c, state.diff_file_list_width);
                let divider = crate::widgets::divider::vertical_divider(
                    crate::state::DragTarget::DiffFileListRight,
                    &c,
                );
                let diff_panel = diff_content(diff, &c, tab.diff_scroll_offset);

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
                container(diff_content(diff, &c, tab.diff_scroll_offset))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface_style)
                    .into()
            }
        }
        None => {
            if !tab.commit_files.is_empty() {
                // Files loaded but no diff selected yet — show file list + loading/placeholder.
                let file_list = commit_file_list(state, &c, state.diff_file_list_width);
                let divider = crate::widgets::divider::vertical_divider(
                    crate::state::DragTarget::DiffFileListRight,
                    &c,
                );
                let right_panel = if tab.is_loading_file_diff {
                    loading_diff_view(&c)
                } else {
                    placeholder_view(&c)
                };
                container(
                    row![file_list, divider, right_panel]
                        .width(Length::Fill)
                        .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::surface_style)
                .into()
            } else {
                container(placeholder_view(&c))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::surface_style)
                    .into()
            }
        }
    }
}

/// Clickable file list for the currently selected commit's diffs.
fn commit_file_list<'a>(state: &'a GitKraft, c: &ThemeColors, width: f32) -> Element<'a, Message> {
    let tab = state.active_tab();

    let header_icon = icon!(icons::FILE_DIFF, 13, c.accent);

    let header_text = text("Files").size(13).color(c.text_primary);

    let file_count = text(format!("({})", tab.commit_files.len()))
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

    for (idx, diff) in tab.commit_files.iter().enumerate() {
        // Extract just the filename for a compact display
        let file_name = diff.file_name();

        // Determine if this file is the currently selected one
        let is_selected = tab.selected_file_index == Some(idx);

        let status_color = theme::status_color(&diff.status, c);

        let status_char = format!("{}", diff.status);

        let status_badge = text(status_char)
            .size(11)
            .font(Font::MONOSPACE)
            .color(status_color);

        let name_color = if is_selected {
            c.text_primary
        } else {
            c.text_secondary
        };

        let name_label = text(file_name.to_string())
            .size(12)
            .color(name_color)
            .wrapping(iced::widget::text::Wrapping::None);

        // Show parent directory as a subtle hint when names might be ambiguous
        let dir_hint: Element<'a, Message> = {
            let short_dir = diff.short_parent_dir();
            if short_dir.is_empty() {
                Space::with_width(0).into()
            } else {
                text(format!("{short_dir}/"))
                    .size(10)
                    .color(c.muted)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .into()
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

        let file_btn = button(row_content)
            .padding(0)
            .width(Length::Fill)
            .style(theme::ghost_button)
            .on_press(Message::SelectDiffByIndex(idx));

        let file_row = container(file_btn)
            .width(Length::Fill)
            .height(Length::Fixed(26.0))
            .clip(true)
            .style(style_fn);

        file_list_col = file_list_col.push(file_row);
    }

    let scrollable_files = scrollable(file_list_col)
        .height(Length::Fill)
        .direction(view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

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
    view_utils::centered_placeholder(icons::FILE_DIFF, 32, "Select a commit or file to view diff", c.muted)
}


/// Loading indicator shown while a single file’s diff is being fetched.
fn loading_diff_view<'a>(c: &ThemeColors) -> Element<'a, Message> {
    view_utils::centered_placeholder(icons::ARROW_REPEAT, 24, "Loading diff…", c.muted)
}

/// Render the full diff content for a single [`DiffInfo`] with virtual
/// scrolling. Only lines within (or near) the visible viewport are
/// materialised as widgets.
fn diff_content<'a>(diff: &'a DiffInfo, c: &ThemeColors, scroll_offset: f32) -> Element<'a, Message> {
    let file_path_display = diff.display_path().to_string();

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
        // Flatten all lines for virtual scrolling
        let total_lines: usize = diff.hunks.iter().map(|h| h.lines.len()).sum();

        let first = ((scroll_offset / DIFF_LINE_HEIGHT) as usize).saturating_sub(DIFF_OVERSCAN);
        let last = (first + DIFF_VISIBLE_LINES + 2 * DIFF_OVERSCAN).min(total_lines);

        let top_space = first as f32 * DIFF_LINE_HEIGHT;
        let bottom_space = (total_lines - last) as f32 * DIFF_LINE_HEIGHT;

        if top_space > 0.0 {
            lines_col = lines_col.push(Space::with_height(top_space));
        }

        // Iterate through hunks to find lines in range [first..last)
        let mut global_idx = 0usize;
        for hunk in &diff.hunks {
            for line in &hunk.lines {
                if global_idx >= first && global_idx < last {
                    lines_col = lines_col.push(render_line(line, c));
                }
                global_idx += 1;
                if global_idx >= last {
                    break;
                }
            }
            if global_idx >= last {
                break;
            }
        }

        if bottom_space > 0.0 {
            lines_col = lines_col.push(Space::with_height(bottom_space));
        }
    }

    let scrollable_content = scrollable(lines_col)
        .height(Length::Fill)
        .on_scroll(|vp| Message::DiffViewScrolled(vp.absolute_offset().y))
        .direction(view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

    column![file_header, scrollable_content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Build a single diff line widget: prefix + content, monospace, colored, with optional background.
fn diff_line_widget<'a>(
    prefix: &'static str,
    content_str: &str,
    color: iced::Color,
    style: Option<fn(&iced::Theme) -> iced::widget::container::Style>,
) -> Element<'a, Message> {
    let prefix_w = text(prefix).size(13).font(Font::MONOSPACE).color(color);
    let content = text(content_str.to_string()).size(13).font(Font::MONOSPACE).color(color);
    let c = container(row![prefix_w, Space::with_width(4), content])
        .padding([1, 12])
        .width(Length::Fill);
    match style {
        Some(s) => c.style(s).into(),
        None => c.into(),
    }
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
        DiffLine::Addition(s) => diff_line_widget("+", s, c.green, Some(theme::diff_add_style)),
        DiffLine::Deletion(s) => diff_line_widget("-", s, c.red, Some(theme::diff_del_style)),
        DiffLine::Context(s) => diff_line_widget(" ", s, c.text_secondary, None),
    }
}
