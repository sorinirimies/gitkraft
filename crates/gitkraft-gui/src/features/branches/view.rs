//! Sidebar branch list — shows local and remote branches, with checkout,
//! create, and delete actions.

use gitkraft_core::BranchType;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the branches sidebar panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F404}')
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let header_text = text("Branches").size(14).color(c.text_primary);

    let toggle_icon_char = if tab.show_branch_create {
        '\u{F2EA}' // dash-circle
    } else {
        '\u{F4FA}' // plus-circle
    };
    let toggle_icon = text(toggle_icon_char)
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let toggle_btn = button(toggle_icon)
        .padding([2, 6])
        .style(theme::icon_button)
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
    let create_form: Element<'_, Message> = if tab.show_branch_create {
        let input = text_input("new-branch-name", &tab.new_branch_name)
            .on_input(Message::NewBranchNameChanged)
            .padding(6)
            .size(13);

        let create_btn = if tab.new_branch_name.trim().is_empty() {
            button(text("Create").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button)
        } else {
            button(text("Create").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button)
                .on_press(Message::CreateBranch)
        };

        container(column![input, create_btn,].spacing(4).width(Length::Fill))
            .padding([4, 10])
            .into()
    } else {
        Space::with_height(0).into()
    };

    // ── Inline rename form ────────────────────────────────────────────────
    let rename_form: Element<'_, Message> = if let Some(ref orig) = tab.rename_branch_target {
        let input = iced::widget::text_input("new branch name", &tab.rename_branch_input)
            .on_input(Message::RenameBranchInputChanged)
            .on_submit(Message::ConfirmRenameBranch)
            .padding(6)
            .size(13);

        let confirm_btn = if tab.rename_branch_input.trim().is_empty()
            || tab.rename_branch_input.trim() == orig.as_str()
        {
            button(text("Rename").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button)
        } else {
            button(text("Rename").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button)
                .on_press(Message::ConfirmRenameBranch)
        };

        let cancel_btn = button(text("Cancel").size(13))
            .padding([4, 10])
            .style(theme::toolbar_button)
            .on_press(Message::CancelRename);

        let hint = text(format!("Renaming '{orig}'")).size(11).color(c.muted);

        container(
            column![
                hint,
                input,
                row![confirm_btn, Space::with_width(4), cancel_btn],
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
    let local_branches: Vec<Element<'_, Message>> = tab
        .branches
        .iter()
        .filter(|b| b.branch_type == BranchType::Local)
        .enumerate()
        .map(|(local_index, branch)| {
            let is_current = branch.is_head;

            let indicator: Element<'_, Message> = if is_current {
                text('\u{F287}') // check-circle-fill
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(12)
                    .color(c.green)
                    .into()
            } else {
                text('\u{F404}') // git-branch icon
                    .font(iced_fonts::BOOTSTRAP_FONT)
                    .size(12)
                    .color(c.muted)
                    .into()
            };

            let name_color = if is_current { c.green } else { c.text_primary };

            let name_label = text(branch.name.as_str()).size(13).color(name_color);

            let checkout_btn = if is_current {
                // Already on this branch — no checkout action.
                button(row![indicator, Space::with_width(6), name_label].align_y(Alignment::Center))
                    .padding([4, 8])
                    .width(Length::Fill)
                    .style(theme::ghost_button)
            } else {
                button(row![indicator, Space::with_width(6), name_label].align_y(Alignment::Center))
                    .padding([4, 8])
                    .width(Length::Fill)
                    .style(theme::ghost_button)
                    .on_press(Message::CheckoutBranch(branch.name.clone()))
            };

            let delete_icon = text('\u{F5DE}')
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(12)
                .color(c.red);

            let delete_btn = if is_current {
                // Can't delete the current branch.
                button(delete_icon)
                    .padding([4, 6])
                    .style(theme::icon_button)
            } else {
                button(delete_icon)
                    .padding([4, 6])
                    .style(theme::icon_button)
                    .on_press(Message::DeleteBranch(branch.name.clone()))
            };

            let branch_row = row![checkout_btn, delete_btn]
                .spacing(2)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            mouse_area(container(branch_row).width(Length::Fill))
                .on_right_press(Message::OpenBranchContextMenu(
                    branch.name.clone(),
                    local_index,
                    branch.is_head,
                ))
                .into()
        })
        .collect();

    // ── Remote branches (read-only list) ──────────────────────────────────
    let remote_branches: Vec<Element<'_, Message>> = tab
        .branches
        .iter()
        .filter(|b| b.branch_type == BranchType::Remote)
        .map(|branch| {
            let icon = text('\u{F469}') // cloud
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(12)
                .color(c.muted);

            let label = text(branch.name.as_str()).size(12).color(c.text_secondary);

            container(row![icon, Space::with_width(6), label].align_y(Alignment::Center))
                .padding([2, 8])
                .width(Length::Fill)
                .into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if !local_branches.is_empty() {
        let local_header = text("Local").size(11).color(c.muted);
        list_col = list_col.push(container(local_header).padding([6, 10]));
        for item in local_branches {
            list_col = list_col.push(item);
        }
    }

    if !remote_branches.is_empty() {
        list_col = list_col.push(Space::with_height(8));
        let remote_header = text("Remote").size(11).color(c.muted);
        list_col = list_col.push(container(remote_header).padding([6, 10]));
        for item in remote_branches {
            list_col = list_col.push(item);
        }
    }

    let content = column![
        header_row,
        create_form,
        rename_form,
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
