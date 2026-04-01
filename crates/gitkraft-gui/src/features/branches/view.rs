//! Sidebar branch list — shows local and remote branches, with checkout,
//! create, and delete actions.

use gitkraft_core::BranchType;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the branches sidebar panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let header_icon = text('\u{F404}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let header_text = text("Branches").size(14).color(theme::TEXT_PRIMARY);

    let toggle_icon_char = if state.show_branch_create {
        '\u{F2EA}' // dash-circle
    } else {
        '\u{F4FA}' // plus-circle
    };
    let toggle_icon = text(toggle_icon_char)
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let toggle_btn = button(toggle_icon)
        .padding([2, 6])
        .on_press(Message::ToggleBranchCreate);

    let header_row = row![
        header_icon,
        Space::with_width(6),
        header_text,
        Space::with_width(Length::Fill),
        toggle_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    // ── New branch form ───────────────────────────────────────────────────
    let create_form: Element<'_, Message> = if state.show_branch_create {
        let input = text_input("new-branch-name", &state.new_branch_name)
            .on_input(Message::NewBranchNameChanged)
            .padding(6)
            .size(13);

        let create_btn = if state.new_branch_name.trim().is_empty() {
            button(text("Create").size(13)).padding([4, 10])
        } else {
            button(text("Create").size(13))
                .padding([4, 10])
                .on_press(Message::CreateBranch)
        };

        container(
            column![
                input,
                create_btn,
            ]
            .spacing(4)
            .width(Length::Fill),
        )
        .padding([4, 10])
        .into()
    } else {
        Space::with_height(0).into()
    };

    // ── Branch list ───────────────────────────────────────────────────────
    let local_branches: Vec<Element<'_, Message>> = state
        .branches
        .iter()
        .filter(|b| b.branch_type == BranchType::Local)
        .map(|branch| {
            let is_current = branch.is_head;

            let indicator: Element<'_, Message> = if is_current {
                text('\u{F287}') // check-circle-fill
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(12)
                    .color(theme::GREEN)
                    .into()
            } else {
                text('\u{F404}') // git-branch icon
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(12)
                    .color(theme::MUTED)
                    .into()
            };

            let name_color = if is_current {
                theme::GREEN
            } else {
                theme::TEXT_PRIMARY
            };

            let name_label = text(branch.name.as_str())
                .size(13)
                .color(name_color);

            let checkout_btn = if is_current {
                // Already on this branch — no checkout action.
                button(
                    row![indicator, Space::with_width(6), name_label]
                        .align_y(Alignment::Center),
                )
                .padding([4, 8])
                .width(Length::Fill)
            } else {
                button(
                    row![indicator, Space::with_width(6), name_label]
                        .align_y(Alignment::Center),
                )
                .padding([4, 8])
                .width(Length::Fill)
                .on_press(Message::CheckoutBranch(branch.name.clone()))
            };

            let delete_icon = text('\u{F5DE}')
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(12)
                .color(theme::RED);

            let delete_btn = if is_current {
                // Can't delete the current branch.
                button(delete_icon).padding([4, 6])
            } else {
                button(delete_icon)
                    .padding([4, 6])
                    .on_press(Message::DeleteBranch(branch.name.clone()))
            };

            let branch_row = row![checkout_btn, delete_btn]
                .spacing(2)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            container(branch_row)
                .width(Length::Fill)
                .into()
        })
        .collect();

    // ── Remote branches (read-only list) ──────────────────────────────────
    let remote_branches: Vec<Element<'_, Message>> = state
        .branches
        .iter()
        .filter(|b| b.branch_type == BranchType::Remote)
        .map(|branch| {
            let icon = text('\u{F469}') // cloud
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(12)
                .color(theme::MUTED);

            let label = text(branch.name.as_str())
                .size(12)
                .color(theme::TEXT_SECONDARY);

            container(
                row![icon, Space::with_width(6), label]
                    .align_y(Alignment::Center),
            )
            .padding([2, 8])
            .width(Length::Fill)
            .into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if !local_branches.is_empty() {
        let local_header = text("Local")
            .size(11)
            .color(theme::MUTED);
        list_col = list_col.push(container(local_header).padding([6, 10]));
        for item in local_branches {
            list_col = list_col.push(item);
        }
    }

    if !remote_branches.is_empty() {
        list_col = list_col.push(Space::with_height(8));
        let remote_header = text("Remote")
            .size(11)
            .color(theme::MUTED);
        list_col = list_col.push(container(remote_header).padding([6, 10]));
        for item in remote_branches {
            list_col = list_col.push(item);
        }
    }

    let content = column![
        header_row,
        create_form,
        scrollable(list_col).height(Length::Fill),
    ]
    .width(220)
    .height(Length::Fill);

    container(content)
        .width(220)
        .height(Length::Fill)
        .style(theme::sidebar_style)
        .into()
}
