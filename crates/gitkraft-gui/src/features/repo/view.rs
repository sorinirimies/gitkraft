//! Welcome screen shown when no repository is open.
//!
//! Renders a centered card with the GitKraft logo text, an "Open Repository"
//! button, a horizontal rule, an "Init Repository" button, and (if available)
//! a list of recently opened repositories.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;

/// Render the welcome / landing view (no repo open yet).
pub fn welcome_view<'a>(state: &'a GitKraft) -> Element<'a, Message> {
    let c = state.colors();
    let tab = state.active_tab();

    // ── Loading state (e.g. auto-opening last repo) ───────────────────────
    if tab.is_loading {
        let spinner_icon = icon!(icons::ARROW_REPEAT, 32, c.accent);

        let loading_label = text(
            tab.status_message
                .as_deref()
                .unwrap_or("Loading repository..."),
        )
        .size(18)
        .color(c.text_secondary);

        let loading_col = column![spinner_icon, Space::new().height(12), loading_label]
            .align_x(Alignment::Center);

        return container(loading_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::bg_style)
            .into();
    }

    let title = text("GitKraft").size(48).color(c.accent);

    let subtitle = text("A modern Git IDE").size(18).color(c.text_secondary);

    let open_icon = icon!(icons::FOLDER_OPEN, 16);

    let open_btn = button(
        iced::widget::row![open_icon, text(" Open Repository").size(16)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([10, 24])
    .style(theme::toolbar_button)
    .on_press(Message::OpenRepo);

    let init_icon = icon!(icons::PERSON_FILL, 16);

    let init_btn = button(
        iced::widget::row![init_icon, text(" Init Repository").size(16)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([10, 24])
    .style(theme::toolbar_button)
    .on_press(Message::InitRepo);

    let hint = text("Open an existing repository or initialise a new one to get started.")
        .size(14)
        .color(c.muted);

    let mut card_col = column![
        title,
        subtitle,
        Space::new().height(24),
        open_btn,
        Space::new().height(8),
        iced::widget::rule::horizontal(1),
        Space::new().height(8),
        init_btn,
        Space::new().height(16),
        hint,
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(420);

    // ── Recent repositories ───────────────────────────────────────────────
    if !state.recent_repos.is_empty() {
        card_col = card_col
            .push(Space::new().height(20))
            .push(iced::widget::rule::horizontal(1))
            .push(Space::new().height(12));

        let recent_header_icon = icon!(icons::CLOCK_HISTORY, 14, c.accent);

        let recent_header_label = text("Recent Repositories").size(14).color(c.text_primary);

        card_col = card_col.push(
            row![
                recent_header_icon,
                Space::new().width(6),
                recent_header_label
            ]
            .align_y(Alignment::Center),
        );

        card_col = card_col.push(Space::new().height(8));

        let mut recent_list = column![].spacing(2).width(Length::Fill);

        for entry in state.recent_repos.iter().take(10) {
            let display_name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let path_str = entry.path.display().to_string();

            let folder_icon = icon!(icons::FOLDER_OPEN, 13, c.muted);

            let name_label = text(display_name.to_string())
                .size(13)
                .color(c.text_primary);

            let path_label = text(path_str).size(11).color(c.muted);

            let entry_content = row![
                folder_icon,
                Space::new().width(8),
                column![name_label, path_label].spacing(1),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill);

            let entry_btn = button(entry_content)
                .padding([6, 10])
                .width(Length::Fill)
                .style(theme::ghost_button)
                .on_press(Message::OpenRecentRepo(entry.path.clone()));

            recent_list = recent_list.push(entry_btn);
        }

        let scrollable_recent = scrollable(recent_list)
            .height(Length::Shrink)
            .direction(view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar);
        card_col = card_col.push(scrollable_recent);
    }

    let card = container(card_col).padding(40).style(theme::surface_style);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::bg_style)
        .into()
}
