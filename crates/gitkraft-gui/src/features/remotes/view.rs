//! Remotes list — rendered in the sidebar below branches.
//!
//! Shows each configured remote with its URL, and a "Fetch" button per remote
//! (currently we only support fetching the first remote via `Message::Fetch`).

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;

/// Render the remotes section for the sidebar.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = icon!(icons::CLOUD, 14, c.accent);

    let header_text = text("Remotes").size(14).color(c.text_primary);

    let fetch_icon = icon!(icons::CLOUD_ARROW_DOWN, 14, c.accent);

    let fetch_msg = (!tab.remotes.is_empty()).then_some(Message::Fetch);
    let fetch_btn = crate::view_utils::on_press_maybe(
        button(fetch_icon).padding([2, 6]).style(theme::icon_button),
        fetch_msg,
    );

    let header_row = row![
        header_icon,
        Space::new(6, 0),
        header_text,
        Space::new(Length::Fill, 0),
        fetch_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    let mut list_col = column![].spacing(2).width(Length::Fill);

    if tab.remotes.is_empty() {
        list_col = list_col.push(view_utils::empty_list_hint(
            "No remotes configured",
            c.muted,
        ));
    } else {
        for remote in &tab.remotes {
            let name_label = container(
                text(remote.name.as_str())
                    .size(13)
                    .color(c.text_primary)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .width(Length::Fill)
            .height(Length::Fixed(18.0))
            .clip(true);

            let url_str = remote.url.as_deref().unwrap_or("<no url>");

            let url_label = container(
                text(url_str)
                    .size(11)
                    .color(c.muted)
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .width(Length::Fill)
            .height(Length::Fixed(16.0))
            .clip(true);

            let remote_item = container(
                column![name_label, url_label]
                    .spacing(2)
                    .width(Length::Fill),
            )
            .padding([4, 10])
            .width(Length::Fill)
            .height(Length::Fixed(42.0))
            .clip(true);

            list_col = list_col.push(remote_item);
        }
    }

    column![header_row, list_col]
        .spacing(2)
        .width(Length::Fill)
        .into()
}
