//! Staging area view — shows unstaged changes, staged changes, and commit
//! message input side-by-side as three columns at the bottom of the main
//! layout.

use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;

/// Render the full staging area panel (unstaged | staged | commit input).
pub(crate) fn view(state: &GitKraft) -> Element<'_, Message> {
    let c = state.colors();
    let unstaged_panel = unstaged_view(state);
    let staged_panel = staged_view(state);
    let commit_panel = commit_view(state);

    let unstaged_pct = (state.staging_unstaged_ratio * 100.0) as u16;
    let staged_pct = (state.staging_staged_ratio * 100.0) as u16;
    let commit_pct = 100u16
        .saturating_sub(unstaged_pct)
        .saturating_sub(staged_pct)
        .max(10);

    let divider1 = crate::widgets::divider::vertical_divider(
        crate::state::DragTarget::StagingUnstagedRight,
        &c,
    );
    let divider2 =
        crate::widgets::divider::vertical_divider(crate::state::DragTarget::StagingStagedRight, &c);

    let content = row![
        container(unstaged_panel)
            .width(Length::FillPortion(unstaged_pct))
            .height(Length::Fill),
        divider1,
        container(staged_panel)
            .width(Length::FillPortion(staged_pct))
            .height(Length::Fill),
        divider2,
        container(commit_panel)
            .width(Length::FillPortion(commit_pct))
            .height(Length::Fill),
    ]
    .height(Length::Fill)
    .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .style(theme::surface_style)
        .into()
}

/// Whether we're rendering the unstaged or staged file list.
enum StagingKind {
    Unstaged,
    Staged,
}

/// Render the "Unstaged Changes" file list.
fn unstaged_view(state: &GitKraft) -> Element<'_, Message> {
    staging_file_list_view(state, StagingKind::Unstaged)
}

/// Render the "Staged Changes" file list.
fn staged_view(state: &GitKraft) -> Element<'_, Message> {
    staging_file_list_view(state, StagingKind::Staged)
}

/// Shared implementation for both unstaged and staged file list panels.
fn staging_file_list_view(state: &GitKraft, kind: StagingKind) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let is_unstaged = matches!(kind, StagingKind::Unstaged);

    // ── Header ────────────────────────────────────────────────────────────
    let header_icon = if is_unstaged {
        icon!(icons::FILE_DIFF, 13, c.yellow)
    } else {
        icon!(icons::CHECK_CIRCLE_FILL, 13, c.green)
    };

    let label_text = if is_unstaged { "Unstaged" } else { "Staged" };
    let header_label = text(label_text).size(13).color(c.text_primary);

    let changes: &[gitkraft_core::DiffFileEntry] = if is_unstaged {
        &tab.unstaged_changes
    } else {
        &tab.staged_changes
    };
    let selected: &std::collections::HashSet<String> = if is_unstaged {
        &tab.selected_unstaged
    } else {
        &tab.selected_staged
    };

    let count_label = text(format!("({})", changes.len())).size(11).color(c.muted);

    let action_all_msg = if is_unstaged {
        (!changes.is_empty()).then_some(Message::StageAll)
    } else {
        (!changes.is_empty()).then_some(Message::UnstageAll)
    };
    let action_all_label = if is_unstaged {
        "Stage All"
    } else {
        "Unstage All"
    };
    let action_all_btn = view_utils::on_press_maybe(
        button(text(action_all_label).size(11))
            .padding([2, 8])
            .style(theme::toolbar_button),
        action_all_msg,
    );

    let header_row = row![
        header_icon,
        Space::new().width(4),
        header_label,
        Space::new().width(4),
        count_label,
        Space::new().width(Length::Fill),
        action_all_btn,
    ]
    .align_y(Alignment::Center)
    .padding([6, 8]);

    // ── File rows ─────────────────────────────────────────────────────────
    let file_rows: Vec<Element<'_, Message>> = changes
        .iter()
        .map(|diff| {
            let file_path_display = diff.display_path();

            let status_color = theme::status_color(&diff.status, &c);

            let status_badge = text(format!("{}", diff.status))
                .size(11)
                .font(Font::MONOSPACE)
                .color(status_color);

            let is_selected = selected.contains(file_path_display);

            let name_color = if is_selected {
                c.accent
            } else {
                c.text_primary
            };
            let file_label = text(file_path_display)
                .size(12)
                .color(name_color)
                .wrapping(iced::widget::text::Wrapping::None);

            let view_btn = button(icon!(icons::CLOUD_UPLOAD, 11, c.accent))
                .padding([2, 4])
                .style(theme::icon_button)
                .on_press(Message::LoadStagingFileDiff(
                    file_path_display.to_string(),
                    !is_unstaged,
                ));

            // Build the action buttons portion of the row.
            let mut file_row = row![
                status_badge,
                Space::new().width(6),
                container(file_label).width(Length::Fill).clip(true),
                Space::new().width(4),
                view_btn,
            ]
            .align_y(Alignment::Center);

            if is_unstaged {
                let stage_btn = button(icon!(icons::PLUS_CIRCLE, 11, c.green))
                    .padding([2, 4])
                    .style(theme::icon_button)
                    .on_press(Message::StageFile(file_path_display.to_string()));
                let discard_btn = button(icon!(icons::TRASH, 11, c.red))
                    .padding([2, 4])
                    .style(theme::icon_button)
                    .on_press(Message::DiscardFile(file_path_display.to_string()));
                file_row = file_row
                    .push(Space::new().width(2))
                    .push(stage_btn)
                    .push(Space::new().width(2))
                    .push(discard_btn);
            } else {
                let unstage_btn = button(icon!(icons::DASH_CIRCLE, 11, c.yellow))
                    .padding([2, 4])
                    .style(theme::icon_button)
                    .on_press(Message::UnstageFile(file_path_display.to_string()));
                file_row = file_row.push(Space::new().width(2)).push(unstage_btn);
            }

            let file_row = file_row.padding([2, 8]);

            let row_style = if is_selected {
                theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
            } else {
                theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
            };

            let toggle_msg = if is_unstaged {
                Message::ToggleSelectUnstaged(file_path_display.to_string())
            } else {
                Message::ToggleSelectStaged(file_path_display.to_string())
            };
            let right_click_msg = if is_unstaged {
                Message::OpenUnstagedFileContextMenu(file_path_display.to_string())
            } else {
                Message::OpenStagedFileContextMenu(file_path_display.to_string())
            };

            mouse_area(container(file_row).width(Length::Fill).style(row_style))
                .on_press(toggle_msg)
                .on_right_press(right_click_msg)
                .into()
        })
        .collect();

    // ── List column ───────────────────────────────────────────────────────
    let mut list_col = column![].spacing(1).width(Length::Fill);

    let empty_hint = if is_unstaged {
        "No unstaged changes"
    } else {
        "No staged changes"
    };

    if file_rows.is_empty() {
        list_col = list_col.push(view_utils::empty_list_hint(empty_hint, c.muted));
    } else {
        for row_el in file_rows {
            list_col = list_col.push(row_el);
        }
    }

    let content = column![
        header_row,
        scrollable(list_col)
            .height(Length::Fill)
            .direction(view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar)
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    view_utils::surface_panel(content, Length::FillPortion(3))
}

/// Render the commit message input and "Commit" button.
fn commit_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = icon!(icons::COMMIT, 13, c.accent);

    let header_label = text("Commit").size(13).color(c.text_primary);

    let header_row = row![header_icon, Space::new().width(4), header_label,]
        .align_y(Alignment::Center)
        .padding([6, 8]);

    let input = text_input("Commit message\u{2026}", &tab.commit_message)
        .id(iced::widget::Id::new("commit_message_input"))
        .on_input(Message::CommitMessageChanged)
        .padding(8)
        .size(13);

    // First-line length hint
    let (first_line_len, severity) = gitkraft_core::check_commit_message(&tab.commit_message);
    let char_hint: Element<'_, Message> = if !tab.commit_message.is_empty() {
        let hint_color = match severity {
            gitkraft_core::CommitMsgSeverity::TooLong => c.red,
            gitkraft_core::CommitMsgSeverity::Warning => c.yellow,
            gitkraft_core::CommitMsgSeverity::Good => c.muted,
        };
        text(format!(
            "{first_line_len}/{}",
            gitkraft_core::COMMIT_SUBJECT_LIMIT
        ))
        .size(10)
        .color(hint_color)
        .into()
    } else {
        Space::new().height(0).into()
    };

    let can_commit = !tab.commit_message.trim().is_empty() && !tab.staged_changes.is_empty();

    let commit_icon = icon!(
        icons::CHECK_CIRCLE_FILL,
        14,
        if can_commit { c.green } else { c.muted }
    );

    let commit_btn_content = row![commit_icon, Space::new().width(6), text("Commit").size(13),]
        .align_y(Alignment::Center);

    let commit_msg = can_commit.then_some(Message::CreateCommit);
    let commit_btn = view_utils::on_press_maybe(
        button(commit_btn_content)
            .padding([8, 16])
            .width(Length::Fill)
            .style(theme::toolbar_button),
        commit_msg,
    );

    let staged_hint = if tab.staged_changes.is_empty() {
        text("Stage files before committing")
            .size(11)
            .color(c.muted)
    } else {
        text(format!("{} file(s) staged", tab.staged_changes.len()))
            .size(11)
            .color(c.text_secondary)
    };

    let content = column![
        header_row,
        container(
            column![
                input,
                char_hint,
                Space::new().height(4),
                commit_btn,
                Space::new().height(4),
                staged_hint,
            ]
            .spacing(2)
            .width(Length::Fill),
        )
        .padding([4, 8]),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    view_utils::surface_panel(content, Length::FillPortion(2))
}
