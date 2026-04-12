//! Staging area view — shows unstaged changes, staged changes, and commit
//! message input side-by-side as three columns at the bottom of the main
//! layout.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Font, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the full staging area panel (unstaged | staged | commit input).
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let unstaged_panel = unstaged_view(state);
    let staged_panel = staged_view(state);
    let commit_panel = commit_view(state);

    let content = row![unstaged_panel, staged_panel, commit_panel]
        .spacing(1)
        .height(Length::Fill)
        .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .style(theme::surface_style)
        .into()
}

/// Render the "Unstaged Changes" file list.
fn unstaged_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F30A}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(13)
        .color(c.yellow);

    let header_label = text("Unstaged").size(13).color(c.text_primary);

    let count_label = text(format!("({})", tab.unstaged_changes.len()))
        .size(11)
        .color(c.muted);

    let stage_all_btn = if tab.unstaged_changes.is_empty() {
        button(text("Stage All").size(11))
            .padding([2, 8])
            .style(theme::toolbar_button)
    } else {
        button(text("Stage All").size(11))
            .padding([2, 8])
            .style(theme::toolbar_button)
            .on_press(Message::StageAll)
    };

    let header_row = row![
        header_icon,
        Space::with_width(4),
        header_label,
        Space::with_width(4),
        count_label,
        Space::with_width(Length::Fill),
        stage_all_btn,
    ]
    .align_y(Alignment::Center)
    .padding([6, 8]);

    let file_rows: Vec<Element<'_, Message>> = tab
        .unstaged_changes
        .iter()
        .map(|diff| {
            let file_path_display = if diff.new_file.is_empty() {
                &diff.old_file
            } else {
                &diff.new_file
            };

            let status_color = theme::status_color(&diff.status, &c);

            let status_badge = text(format!("{}", diff.status))
                .size(11)
                .font(Font::MONOSPACE)
                .color(status_color);

            let file_label = text(file_path_display.as_str())
                .size(12)
                .color(c.text_primary);

            let view_btn = button(
                text('\u{F2C0}')
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(11)
                    .color(c.accent),
            )
            .padding([2, 4])
            .style(theme::icon_button)
            .on_press(Message::SelectDiff(diff.clone()));

            let stage_btn = button(
                text('\u{F4FA}')
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(11)
                    .color(c.green),
            )
            .padding([2, 4])
            .style(theme::icon_button)
            .on_press(Message::StageFile(file_path_display.to_string()));

            let discard_btn = button(
                text('\u{F5DE}')
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(11)
                    .color(c.red),
            )
            .padding([2, 4])
            .style(theme::icon_button)
            .on_press(Message::DiscardFile(file_path_display.to_string()));

            let file_row = row![
                status_badge,
                Space::with_width(6),
                file_label,
                Space::with_width(Length::Fill),
                view_btn,
                Space::with_width(2),
                stage_btn,
                Space::with_width(2),
                discard_btn,
            ]
            .align_y(Alignment::Center)
            .padding([2, 8]);

            container(file_row).width(Length::Fill).into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if file_rows.is_empty() {
        let empty_msg = text("No unstaged changes").size(12).color(c.muted);
        list_col = list_col.push(
            container(empty_msg)
                .padding([12, 8])
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else {
        for row_el in file_rows {
            list_col = list_col.push(row_el);
        }
    }

    let content = column![header_row, scrollable(list_col).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .style(theme::surface_style)
        .into()
}

/// Render the "Staged Changes" file list.
fn staged_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F287}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(13)
        .color(c.green);

    let header_label = text("Staged").size(13).color(c.text_primary);

    let count_label = text(format!("({})", tab.staged_changes.len()))
        .size(11)
        .color(c.muted);

    let unstage_all_btn = if tab.staged_changes.is_empty() {
        button(text("Unstage All").size(11))
            .padding([2, 8])
            .style(theme::toolbar_button)
    } else {
        button(text("Unstage All").size(11))
            .padding([2, 8])
            .style(theme::toolbar_button)
            .on_press(Message::UnstageAll)
    };

    let header_row = row![
        header_icon,
        Space::with_width(4),
        header_label,
        Space::with_width(4),
        count_label,
        Space::with_width(Length::Fill),
        unstage_all_btn,
    ]
    .align_y(Alignment::Center)
    .padding([6, 8]);

    let file_rows: Vec<Element<'_, Message>> = tab
        .staged_changes
        .iter()
        .map(|diff| {
            let file_path_display = if diff.new_file.is_empty() {
                &diff.old_file
            } else {
                &diff.new_file
            };

            let status_color = theme::status_color(&diff.status, &c);

            let status_badge = text(format!("{}", diff.status))
                .size(11)
                .font(Font::MONOSPACE)
                .color(status_color);

            let file_label = text(file_path_display.as_str())
                .size(12)
                .color(c.text_primary);

            let view_btn = button(
                text('\u{F2C0}')
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(11)
                    .color(c.accent),
            )
            .padding([2, 4])
            .style(theme::icon_button)
            .on_press(Message::SelectDiff(diff.clone()));

            let unstage_btn = button(
                text('\u{F2EA}')
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(11)
                    .color(c.yellow),
            )
            .padding([2, 4])
            .style(theme::icon_button)
            .on_press(Message::UnstageFile(file_path_display.to_string()));

            let file_row = row![
                status_badge,
                Space::with_width(6),
                file_label,
                Space::with_width(Length::Fill),
                view_btn,
                Space::with_width(2),
                unstage_btn,
            ]
            .align_y(Alignment::Center)
            .padding([2, 8]);

            container(file_row).width(Length::Fill).into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if file_rows.is_empty() {
        let empty_msg = text("No staged changes").size(12).color(c.muted);
        list_col = list_col.push(
            container(empty_msg)
                .padding([12, 8])
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    } else {
        for row_el in file_rows {
            list_col = list_col.push(row_el);
        }
    }

    let content = column![header_row, scrollable(list_col).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .style(theme::surface_style)
        .into()
}

/// Render the commit message input and "Commit" button.
fn commit_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F468}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(13)
        .color(c.accent);

    let header_label = text("Commit").size(13).color(c.text_primary);

    let header_row = row![header_icon, Space::with_width(4), header_label,]
        .align_y(Alignment::Center)
        .padding([6, 8]);

    let input = text_input("Commit message…", &tab.commit_message)
        .on_input(Message::CommitMessageChanged)
        .padding(8)
        .size(13);

    let can_commit = !tab.commit_message.trim().is_empty() && !tab.staged_changes.is_empty();

    let commit_icon = text('\u{F287}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(if can_commit { c.green } else { c.muted });

    let commit_btn_content = row![commit_icon, Space::with_width(6), text("Commit").size(13),]
        .align_y(Alignment::Center);

    let commit_btn = if can_commit {
        button(commit_btn_content)
            .padding([8, 16])
            .width(Length::Fill)
            .style(theme::toolbar_button)
            .on_press(Message::CreateCommit)
    } else {
        button(commit_btn_content)
            .padding([8, 16])
            .width(Length::Fill)
            .style(theme::toolbar_button)
    };

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
                Space::with_height(6),
                commit_btn,
                Space::with_height(4),
                staged_hint,
            ]
            .spacing(2)
            .width(Length::Fill),
        )
        .padding([4, 8]),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::FillPortion(2))
        .height(Length::Fill)
        .style(theme::surface_style)
        .into()
}
