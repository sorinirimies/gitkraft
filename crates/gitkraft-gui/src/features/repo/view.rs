//! Welcome screen shown when no repository is open.
//!
//! Renders a centered card with the GitKraft logo text, an "Open Repository"
//! button, a horizontal rule, and an "Init Repository" button.

use iced::widget::{button, column, container, horizontal_rule, text, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::theme;

/// Render the welcome / landing view (no repo open yet).
pub fn welcome_view<'a>() -> Element<'a, Message> {
    let title = text("GitKraft").size(48).color(theme::ACCENT);

    let subtitle = text("A modern Git IDE")
        .size(18)
        .color(theme::TEXT_SECONDARY);

    let open_icon = text('\u{F3D8}').font(iced_fonts::BOOTSTRAP_FONT).size(16);

    let open_btn = button(
        iced::widget::row![open_icon, text(" Open Repository").size(16)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([10, 24])
    .on_press(Message::OpenRepo);

    let init_icon = text('\u{F4DA}').font(iced_fonts::BOOTSTRAP_FONT).size(16);

    let init_btn = button(
        iced::widget::row![init_icon, text(" Init Repository").size(16)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([10, 24])
    .on_press(Message::InitRepo);

    let hint = text("Open an existing repository or initialise a new one to get started.")
        .size(14)
        .color(theme::MUTED);

    let card = container(
        column![
            title,
            subtitle,
            Space::with_height(24),
            open_btn,
            Space::with_height(8),
            horizontal_rule(1),
            Space::with_height(8),
            init_btn,
            Space::with_height(16),
            hint,
        ]
        .spacing(4)
        .align_x(Alignment::Center)
        .width(380),
    )
    .padding(40)
    .style(theme::surface_style);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(theme::BG)),
            ..Default::default()
        })
        .into()
}
