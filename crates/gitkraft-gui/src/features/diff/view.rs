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
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
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

    // ── Priority 1: commit range diff (multiple commits selected) ─────────
    if !tab.commit_range_diffs.is_empty() {
        let multi_panel = multi_diff_content(&tab.commit_range_diffs, &c, tab.diff_scroll_offset);
        return container(multi_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into();
    }

    // ── Priority 2: loading indicator for range diff ───────────────────────
    if tab.selected_commits.len() > 1 && tab.is_loading_file_diff {
        return container(loading_diff_view(&c))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::surface_style)
            .into();
    }

    // ── Priority 3: multi-file diff (existing code unchanged below) ────────
    // Multi-file mode: show concatenated diffs for all selected files.
    if !tab.multi_file_diffs.is_empty() {
        let file_list = commit_file_list(state, &c, state.diff_file_list_width);
        let divider = crate::widgets::divider::vertical_divider(
            crate::state::DragTarget::DiffFileListRight,
            &c,
        );
        let multi_panel = multi_diff_content(&tab.multi_file_diffs, &c, tab.diff_scroll_offset);
        return container(
            row![file_list, divider, multi_panel]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface_style)
        .into();
    }

    match &tab.selected_diff {
        Some(diff) => {
            if !tab.commit_files.is_empty() {
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

    let multi_count = tab.selected_commit_file_indices.len();
    let file_count = if multi_count > 1 {
        text(format!("({} selected)", multi_count))
            .size(11)
            .color(c.accent)
    } else {
        text(format!("({})", tab.commit_files.len()))
            .size(11)
            .color(c.muted)
    };

    let header_row = row![
        header_icon,
        Space::new().width(4),
        header_text,
        Space::new().width(4),
        file_count,
    ]
    .align_y(Alignment::Center)
    .padding([6, 8]);

    let mut file_list_col = column![].spacing(1).width(Length::Fill);
    let oid_for_menu = tab.selected_commit_oid.clone().unwrap_or_default();

    for (idx, diff) in tab.commit_files.iter().enumerate() {
        // Extract just the filename for a compact display
        let file_name = diff.file_name();

        // Determine if this file is the primary selected one or part of a multi-selection
        let is_selected = tab.selected_file_index == Some(idx);
        let is_multi_selected = tab.selected_commit_file_indices.contains(&idx);

        let status_color = theme::status_color(&diff.status, c);

        let status_char = format!("{}", diff.status);

        let status_badge = text(status_char)
            .size(11)
            .font(Font::MONOSPACE)
            .color(status_color);

        let name_color = if is_selected || is_multi_selected {
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
                Space::new().into()
            } else {
                text(format!("{short_dir}/"))
                    .size(10)
                    .color(c.muted)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .into()
            }
        };

        // Position of this file in the selection order (0-based → display as 1-based)
        let selection_badge: Element<'a, Message> = if let Some(pos) = tab
            .selected_commit_file_indices
            .iter()
            .position(|&i| i == idx)
        {
            container(
                text(format!("{}", pos + 1))
                    .size(10)
                    .font(Font::MONOSPACE)
                    .color(c.accent),
            )
            .width(16)
            .center_x(Length::Fixed(16.0))
            .into()
        } else {
            Space::new().width(16).into()
        };

        let row_content = row![
            selection_badge,
            Space::new().width(2),
            status_badge,
            Space::new().width(4),
            column![row![dir_hint, name_label].align_y(Alignment::Center),],
        ]
        .align_y(Alignment::Center)
        .padding([4, 8])
        .width(Length::Fill);

        // Primary selected row gets the strongest highlight; multi-selected-but-not-primary
        // gets a distinct secondary highlight; unselected rows use the plain surface style.
        let style_fn = if is_selected {
            theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
        } else if is_multi_selected {
            theme::highlight_row_style as fn(&iced::Theme) -> iced::widget::container::Style
        } else {
            theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
        };

        let file_btn = button(row_content)
            .padding(0)
            .width(Length::Fill)
            .style(theme::ghost_button)
            .on_press(Message::SelectDiffByIndex(idx));

        let file_row = mouse_area(
            container(file_btn)
                .width(Length::Fill)
                .height(Length::Fixed(26.0))
                .clip(true)
                .style(style_fn),
        )
        .on_right_press(Message::OpenCommitFileContextMenu(
            oid_for_menu.clone(),
            diff.display_path().to_string(),
        ));

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

/// Render concatenated diffs for multiple selected files, separated by per-file
/// headers showing the file path and status.
fn multi_diff_content<'a>(
    diffs: &'a [DiffInfo],
    c: &ThemeColors,
    _scroll_offset: f32,
) -> Element<'a, Message> {
    let mut col = column![].width(Length::Fill);

    for diff in diffs {
        // Per-file header bar
        let status_color = theme::status_color(&diff.status, c);
        let header = container(
            row![
                text(format!(" {} ", diff.status))
                    .size(12)
                    .color(status_color)
                    .font(Font::MONOSPACE),
                Space::new().width(8),
                text(diff.display_path().to_string())
                    .size(13)
                    .color(c.text_primary)
                    .font(Font::MONOSPACE),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .width(Length::Fill)
        .style(theme::header_style);

        col = col.push(header);

        if diff.hunks.is_empty() {
            col = col.push(
                container(
                    text("No diff content.")
                        .size(13)
                        .color(c.muted)
                        .font(Font::MONOSPACE),
                )
                .padding([4, 12]),
            );
        } else {
            for hunk in &diff.hunks {
                for line in &hunk.lines {
                    col = col.push(render_line(line, c));
                }
            }
        }

        // Small gap between files
        col = col.push(Space::new().height(8));
    }

    scrollable(col)
        .height(Length::Fill)
        .on_scroll(|vp| Message::DiffViewScrolled(vp.absolute_offset().y))
        .direction(view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar)
        .into()
}

/// Placeholder shown when no diff is selected.
fn placeholder_view<'a>(c: &ThemeColors) -> Element<'a, Message> {
    view_utils::centered_placeholder(
        icons::FILE_DIFF,
        32,
        "Select a commit or file to view diff",
        c.muted,
    )
}

/// Loading indicator shown while a single file's diff is being fetched.
fn loading_diff_view<'a>(c: &ThemeColors) -> Element<'a, Message> {
    view_utils::centered_placeholder(icons::ARROW_REPEAT, 24, "Loading diff…", c.muted)
}

/// Render the full diff content for a single [`DiffInfo`] with virtual
/// scrolling. Only lines within (or near) the visible viewport are
/// materialised as widgets.
fn diff_content<'a>(
    diff: &'a DiffInfo,
    c: &ThemeColors,
    scroll_offset: f32,
) -> Element<'a, Message> {
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
        row![status_badge, Space::new().width(8), file_label].align_y(iced::Alignment::Center),
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
            lines_col = lines_col.push(Space::new().height(top_space));
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
            lines_col = lines_col.push(Space::new().height(bottom_space));
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
    let content = text(content_str.to_string())
        .size(13)
        .font(Font::MONOSPACE)
        .color(color);
    let c = container(row![prefix_w, Space::new().width(4), content])
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

// ── File history overlay ──────────────────────────────────────────────────────

/// Render the file-history overlay in the diff panel area.
pub fn file_history_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();
    let path = tab.file_history_path.as_deref().unwrap_or("");
    let file_name = path.rsplit('/').next().unwrap_or(path);

    let close_btn = button(text("✕").size(13).color(c.muted))
        .padding([2, 8])
        .style(theme::ghost_button)
        .on_press(Message::CloseFileHistory);

    let header = row![
        icon!(icons::CLOCK, 14, c.accent),
        Space::new().width(6),
        text(format!("File History: {file_name}"))
            .size(14)
            .color(c.text_primary),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    let body: Element<'_, Message> = if tab.file_history_commits.is_empty() {
        let msg = if tab.is_loading {
            "Loading…"
        } else {
            "No commits touch this file."
        };
        container(text(msg).size(13).color(c.muted))
            .width(Length::Fill)
            .padding(20)
            .center_x(Length::Fill)
            .into()
    } else {
        let mut list = column![].width(Length::Fill);
        for commit in &tab.file_history_commits {
            let short = commit.short_oid.as_str();
            let summary = view_utils::truncate_to_fit(&commit.summary, 280.0, 7.0);
            let rel_time = commit.relative_time();

            let row_content = row![
                text(short).size(11).color(c.accent).font(Font::MONOSPACE),
                Space::new().width(8),
                container(
                    text(summary)
                        .size(12)
                        .color(c.text_primary)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .width(Length::Fill)
                .clip(true),
                Space::new().width(8),
                text(rel_time).size(11).color(c.muted),
            ]
            .align_y(Alignment::Center)
            .padding([3, 8]);

            let oid = commit.oid.clone();
            list = list.push(
                button(row_content)
                    .padding(0)
                    .width(Length::Fill)
                    .style(theme::ghost_button)
                    .on_press(Message::SelectFileHistoryCommit(oid)),
            );
        }

        scrollable(list)
            .height(Length::Fill)
            .on_scroll(|vp| Message::FileHistoryScrolled(vp.absolute_offset().y))
            .direction(view_utils::thin_scrollbar())
            .style(theme::overlay_scrollbar)
            .into()
    };

    let content = column![header, body]
        .width(Length::Fill)
        .height(Length::Fill);

    view_utils::surface_panel(content, Length::Fill)
}

// ── Blame overlay ─────────────────────────────────────────────────────────────

/// Render the blame overlay in the diff panel area.
pub fn blame_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();
    let path = tab.blame_path.as_deref().unwrap_or("");
    let file_name = path.rsplit('/').next().unwrap_or(path);

    let close_btn = button(
        row![
            text("✕").size(12).color(c.text_primary),
            Space::new().width(4),
            text("Close").size(12).color(c.text_primary),
            Space::new().width(4),
            text("[Esc]").size(11).color(c.muted),
        ]
        .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(theme::toolbar_button)
    .on_press(Message::CloseFileBlame);

    let header = row![
        icon!(icons::CLOCK, 14, c.accent),
        Space::new().width(6),
        text(format!("Blame: {file_name}"))
            .size(14)
            .color(c.text_primary),
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    let body: Element<'_, Message> = if tab.blame_lines.is_empty() {
        let msg = if tab.is_loading {
            "Loading…"
        } else {
            "No blame data."
        };
        container(text(msg).size(13).color(c.muted))
            .width(Length::Fill)
            .padding(20)
            .center_x(Length::Fill)
            .into()
    } else {
        let mut list = column![].width(Length::Fill);
        for line in &tab.blame_lines {
            let rel_time = line.relative_time();
            let author = view_utils::truncate_to_fit(&line.author_name, 80.0, 7.0);
            let line_content = container(
                text(line.content.as_str())
                    .size(11)
                    .color(c.text_primary)
                    .font(Font::MONOSPACE)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true);

            let blame_row = row![
                text(line.short_oid.as_str())
                    .size(10)
                    .color(c.accent)
                    .font(Font::MONOSPACE),
                Space::new().width(6),
                container(
                    text(author)
                        .size(10)
                        .color(c.text_secondary)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .width(80)
                .clip(true),
                Space::new().width(4),
                container(
                    text(rel_time)
                        .size(10)
                        .color(c.muted)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .width(54)
                .clip(true),
                Space::new().width(4),
                container(
                    text(format!("{:4}", line.line_number))
                        .size(10)
                        .color(c.muted)
                        .font(Font::MONOSPACE),
                )
                .width(32),
                Space::new().width(6),
                line_content,
            ]
            .align_y(Alignment::Center)
            .padding([1, 8]);

            list = list.push(blame_row);
        }

        scrollable(list)
            .height(Length::Fill)
            .on_scroll(|vp| Message::BlameScrolled(vp.absolute_offset().y))
            .direction(view_utils::thin_scrollbar())
            .style(theme::overlay_scrollbar)
            .into()
    };

    let content = column![header, body]
        .width(Length::Fill)
        .height(Length::Fill);

    view_utils::surface_panel(content, Length::Fill)
}
