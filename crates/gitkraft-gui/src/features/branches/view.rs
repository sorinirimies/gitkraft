//! Sidebar branch list — shows local and remote branches, with checkout,
//! create, and delete actions.

use gitkraft_core::BranchType;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;
use crate::view_utils::truncate_to_fit;

/// Render the branches sidebar panel.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();
    let sidebar_width = state.sidebar_width;

    let header_icon = icon!(icons::GIT_BRANCH, 14, c.accent);

    let header_text = text("Branches").size(14).color(c.text_primary);

    let toggle_icon_char = if tab.show_branch_create {
        icons::DASH_CIRCLE
    } else {
        icons::PLUS_CIRCLE
    };
    let toggle_icon = icon!(toggle_icon_char, 14, c.accent);

    let toggle_btn = button(toggle_icon)
        .padding([2, 6])
        .style(theme::icon_button)
        .on_press(Message::ToggleBranchCreate);

    let header_row = row![
        header_icon,
        Space::new().width(6),
        header_text,
        Space::new().width(Length::Fill),
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

        let create_msg = (!tab.new_branch_name.trim().is_empty()).then_some(Message::CreateBranch);
        let create_btn = view_utils::on_press_maybe(
            button(text("Create").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button),
            create_msg,
        );

        container(column![input, create_btn,].spacing(4).width(Length::Fill))
            .padding([4, 10])
            .into()
    } else {
        Space::new().into()
    };

    // ── Inline rename form ────────────────────────────────────────────────
    let rename_form: Element<'_, Message> = if let Some(ref orig) = tab.rename_branch_target {
        let input = iced::widget::text_input("new branch name", &tab.rename_branch_input)
            .on_input(Message::RenameBranchInputChanged)
            .on_submit(Message::ConfirmRenameBranch)
            .padding(6)
            .size(13);

        let rename_enabled = !tab.rename_branch_input.trim().is_empty()
            && tab.rename_branch_input.trim() != orig.as_str();
        let confirm_btn = view_utils::on_press_maybe(
            button(text("Rename").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button),
            rename_enabled.then_some(Message::ConfirmRenameBranch),
        );

        let cancel_btn = button(text("Cancel").size(13))
            .padding([4, 10])
            .style(theme::toolbar_button)
            .on_press(Message::CancelRename);

        let hint = text(format!("Renaming '{orig}'")).size(11).color(c.muted);

        container(
            column![
                hint,
                input,
                row![confirm_btn, Space::new().width(4), cancel_btn],
            ]
            .spacing(4)
            .width(Length::Fill),
        )
        .padding([4, 10])
        .into()
    } else {
        Space::new().into()
    };

    // ── Tag creation form ─────────────────────────────────────────────────
    let tag_form: Element<'_, Message> = if let Some(ref oid) = tab.create_tag_target_oid {
        let short_oid = gitkraft_core::utils::short_oid_str(oid);
        let label = if tab.create_tag_annotated {
            format!("Creating annotated tag at {short_oid}")
        } else {
            format!("Creating lightweight tag at {short_oid}")
        };
        let hint = text(label).size(11).color(c.muted);

        let name_input = iced::widget::text_input("tag-name", &tab.create_tag_name)
            .on_input(Message::TagNameChanged)
            .on_submit(Message::ConfirmCreateTag)
            .padding(6)
            .size(13);

        let tag_msg = (!tab.create_tag_name.trim().is_empty()).then_some(Message::ConfirmCreateTag);
        let confirm_btn = view_utils::on_press_maybe(
            button(text("Create tag").size(13))
                .padding([4, 10])
                .style(theme::toolbar_button),
            tag_msg,
        );

        let cancel_btn = button(text("Cancel").size(13))
            .padding([4, 10])
            .style(theme::toolbar_button)
            .on_press(Message::CancelCreateTag);

        let mut form_col = column![hint, name_input].spacing(4).width(Length::Fill);

        if tab.create_tag_annotated {
            let msg_input = iced::widget::text_input("tag message", &tab.create_tag_message)
                .on_input(Message::TagMessageChanged)
                .padding(6)
                .size(13);
            form_col = form_col.push(msg_input);
        }

        form_col = form_col.push(row![confirm_btn, Space::new().width(4), cancel_btn]);

        container(form_col).padding([4, 10]).into()
    } else {
        Space::new().into()
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
                icon!(icons::CHECK_CIRCLE_FILL, 12, c.green).into()
            } else {
                icon!(icons::GIT_BRANCH, 12, c.muted).into()
            };

            let name_color = if is_current { c.green } else { c.text_primary };

            // Available px: sidebar minus button padding(16) + indicator(14)
            // + gap(6) + delete-btn(28) + row-spacing(2) ≈ 66 px overhead.
            let name_available = (sidebar_width - 66.0).max(20.0);
            let display_name = truncate_to_fit(branch.name.as_str(), name_available, 7.5);

            let name_label = text(display_name)
                .size(13)
                .color(name_color)
                .wrapping(iced::widget::text::Wrapping::None);

            let checkout_msg =
                (!is_current).then_some(Message::CheckoutBranch(branch.name.clone()));
            let checkout_btn = view_utils::on_press_maybe(
                button(
                    row![indicator, Space::new().width(6), name_label].align_y(Alignment::Center),
                )
                .padding([4, 8])
                .width(Length::Fill)
                .style(theme::ghost_button),
                checkout_msg,
            );

            let delete_icon = icon!(icons::TRASH, 12, c.red);

            let delete_msg = (!is_current).then_some(Message::DeleteBranch(branch.name.clone()));
            let delete_btn = view_utils::on_press_maybe(
                button(delete_icon)
                    .padding([4, 6])
                    .style(theme::icon_button),
                delete_msg,
            );

            let branch_row = row![checkout_btn, delete_btn]
                .spacing(2)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            mouse_area(
                container(branch_row)
                    .width(Length::Fill)
                    .height(Length::Fixed(28.0))
                    .clip(true),
            )
            .on_right_press(Message::OpenBranchContextMenu(
                branch.name.clone(),
                local_index,
                branch.is_head,
            ))
            .into()
        })
        .collect();

    // ── Remote branches (with context menu) ───────────────────────────────
    let remote_branches: Vec<Element<'_, Message>> = tab
        .branches
        .iter()
        .filter(|b| b.branch_type == BranchType::Remote)
        .map(|branch| {
            let icon = icon!(icons::CLOUD, 12, c.muted);

            // Available px: sidebar minus item padding(16) + icon(14) + gap(6)
            // ≈ 36 px overhead.
            let label_available = (sidebar_width - 36.0).max(20.0);
            let display_remote = truncate_to_fit(branch.name.as_str(), label_available, 7.0);

            let label = text(display_remote)
                .size(12)
                .color(c.text_secondary)
                .wrapping(iced::widget::text::Wrapping::None);

            let branch_btn =
                button(row![icon, Space::new().width(6), label].align_y(Alignment::Center))
                    .padding([2, 8])
                    .width(Length::Fill)
                    .style(theme::ghost_button)
                    .on_press(Message::CheckoutRemoteBranch(branch.name.clone()));

            mouse_area(
                container(branch_btn)
                    .width(Length::Fill)
                    .height(Length::Fixed(22.0))
                    .clip(true),
            )
            .on_right_press(Message::OpenRemoteBranchContextMenu(branch.name.clone()))
            .into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if !local_branches.is_empty() || tab.local_branches_expanded {
        let local_count = tab
            .branches
            .iter()
            .filter(|b| b.branch_type == BranchType::Local)
            .count();

        let local_header_btn = view_utils::collapsible_header(
            tab.local_branches_expanded,
            "Local",
            local_count,
            Message::ToggleLocalBranches,
            c.muted,
        );
        list_col = list_col.push(local_header_btn);

        if tab.local_branches_expanded {
            for item in local_branches {
                list_col = list_col.push(item);
            }
        }
    }

    if !remote_branches.is_empty() || tab.remote_branches_expanded {
        let remote_count = tab
            .branches
            .iter()
            .filter(|b| b.branch_type == BranchType::Remote)
            .count();

        list_col = list_col.push(Space::new().height(4));

        let remote_header_btn = view_utils::collapsible_header(
            tab.remote_branches_expanded,
            "Remote",
            remote_count,
            Message::ToggleRemoteBranches,
            c.muted,
        );
        list_col = list_col.push(remote_header_btn);

        if tab.remote_branches_expanded {
            for item in remote_branches {
                list_col = list_col.push(item);
            }
        }
    }

    let content = column![
        header_row,
        create_form,
        rename_form,
        tag_form,
        scrollable(list_col)
            .height(Length::Fill)
            .direction(view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::sidebar_style)
        .into()
}
