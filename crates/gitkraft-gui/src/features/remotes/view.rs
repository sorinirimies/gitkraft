//! Remotes list — rendered in the sidebar below branches.
//!
//! Shows each configured remote with its URL, and a "Fetch" button per remote
//! (currently we only support fetching the first remote via `Message::Fetch`).

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the remotes section for the sidebar.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let header_icon = text('\u{F469}') // cloud icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let header_text = text("Remotes").size(14).color(c.text_primary);

    let fetch_icon = text('\u{F116}') // arrow-down-circle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let fetch_btn = if tab.remotes.is_empty() {
        button(fetch_icon).padding([2, 6]).style(theme::icon_button)
    } else {
        button(fetch_icon)
            .padding([2, 6])
            .style(theme::icon_button)
            .on_press(Message::Fetch)
    };

    let header_row = row![
        header_icon,
        Space::with_width(6),
        header_text,
        Space::with_width(Length::Fill),
        fetch_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    let mut list_col = column![].spacing(2).width(Length::Fill);

    if tab.remotes.is_empty() {
        let empty_msg = text("No remotes configured").size(12).color(c.muted);
        list_col = list_col.push(container(empty_msg).padding([4, 10]).width(Length::Fill));
    } else {
        for remote in &tab.remotes {
            let name_label = text(remote.name.as_str()).size(13).color(c.text_primary);

            let url_str = remote.url.as_deref().unwrap_or("<no url>");

            let url_label = text(url_str).size(11).color(c.muted);

            let remote_item = column![name_label, url_label]
                .spacing(2)
                .width(Length::Fill);

            list_col = list_col.push(container(remote_item).padding([4, 10]).width(Length::Fill));
        }
    }

    column![header_row, list_col]
        .spacing(2)
        .width(Length::Fill)
        .into()
}
