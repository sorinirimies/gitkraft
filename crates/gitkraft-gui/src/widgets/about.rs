//! About dialog — app name, version, author and project info.

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the About dialog as a centred modal over a dimmed backdrop.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    use gitkraft_core::about;

    let c = state.colors();

    let link = |label: &'static str, url: &'static str| {
        button(text(label).size(13).color(c.accent))
            .padding([2, 0])
            .style(theme::ghost_button_flat)
            .on_press(Message::OpenUrl(url.to_string()))
    };

    let info_row = |key: &'static str, value: Element<'static, Message>| {
        row![
            container(text(key).size(13).color(c.muted)).width(90),
            value,
        ]
        .align_y(Alignment::Center)
    };

    let close_btn = button(text("Close").size(12).color(c.text_primary))
        .padding([4, 16])
        .style(theme::toolbar_button)
        .on_press(Message::ToggleAbout);

    let content = column![
        text(about::APP_NAME).size(28).color(c.accent),
        text(about::version_label())
            .size(14)
            .color(c.text_secondary),
        Space::new().height(10),
        text(about::DESCRIPTION).size(13).color(c.text_primary),
        Space::new().height(14),
        info_row(
            "Author",
            text(about::AUTHOR_NAME)
                .size(13)
                .color(c.text_primary)
                .into()
        ),
        info_row("GitHub", link("@sorinirimies", about::AUTHOR_URL).into()),
        info_row("Project", link(about::REPO_URL, about::REPO_URL).into()),
        info_row(
            "Version",
            text(about::VERSION).size(13).color(c.text_primary).into()
        ),
        info_row(
            "License",
            text(about::LICENSE).size(13).color(c.text_primary).into()
        ),
        Space::new().height(16),
        container(close_btn)
            .width(Length::Fill)
            .align_right(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill);

    let panel = container(content)
        .width(460)
        .padding(20)
        .style(theme::context_menu_style);

    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .style(theme::backdrop_style),
    )
    .on_press(Message::ToggleAbout);

    // Swallow clicks on the panel so they don't dismiss the dialog.
    let panel = mouse_area(panel).on_press(Message::Noop);

    let centered = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    iced::widget::stack![backdrop, centered].into()
}
