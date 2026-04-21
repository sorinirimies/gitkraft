//! Stash list view — shows stash entries in the sidebar with save, pop, and
//! drop actions for each entry.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;
use crate::view_utils::truncate_to_fit;

/// Render the stash panel (typically shown in the sidebar beneath branches).
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();
    let sidebar_width = state.sidebar_width;

    let header_icon = icon!(icons::STACK, 14, c.accent);

    let header_label = text("Stashes").size(14).color(c.text_primary);

    let count_label = text(format!("({})", tab.stashes.len()))
        .size(11)
        .color(c.muted);

    let save_icon = icon!(icons::PLUS_CIRCLE, 14, c.green);

    let save_btn = button(save_icon)
        .padding([2, 6])
        .style(theme::icon_button)
        .on_press(Message::StashSave);

    let header_row = row![
        header_icon,
        Space::new().width(6),
        header_label,
        Space::new().width(4),
        count_label,
        Space::new().width(Length::Fill),
        save_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    // ── Stash message input ───────────────────────────────────────────────
    let stash_input = text_input("Stash message (optional)…", &tab.stash_message)
        .on_input(Message::StashMessageChanged)
        .padding(4)
        .size(12);

    let input_row = container(stash_input).padding([2, 10]).width(Length::Fill);

    // ── Stash entries ─────────────────────────────────────────────────────
    let stash_entries: Vec<Element<'_, Message>> = tab
        .stashes
        .iter()
        .map(|entry| {
            let index_label = text(format!("stash@{{{}}}", entry.index))
                .size(11)
                .color(c.accent)
                .font(iced::Font::MONOSPACE);

            // Available px: sidebar minus stash-index label (~60px) + gap(6)
            // + pop-btn(22) + gap(2) + drop-btn(22) + padding(16) + gap(4)
            // ≈ 132 px overhead.
            let msg_available = (sidebar_width - 132.0).max(30.0);
            let display_msg = truncate_to_fit(entry.message.as_str(), msg_available, 6.5);

            let msg_label = text(display_msg)
                .size(11)
                .color(c.text_secondary)
                .wrapping(iced::widget::text::Wrapping::None);

            let pop_icon = icon!(icons::BOX_ARROW_UP, 11, c.green);

            let pop_btn = button(pop_icon)
                .padding([2, 4])
                .style(theme::icon_button)
                .on_press(Message::StashPop(entry.index));

            let drop_icon = icon!(icons::TRASH, 11, c.red);

            let drop_btn = button(drop_icon)
                .padding([2, 4])
                .style(theme::icon_button)
                .on_press(Message::StashDrop(entry.index));

            let entry_row = row![
                index_label,
                Space::new().width(6),
                msg_label,
                Space::new().width(Length::Fill),
                pop_btn,
                Space::new().width(2),
                drop_btn,
            ]
            .align_y(Alignment::Center)
            .padding([3, 8]);

            container(entry_row)
                .width(Length::Fill)
                .height(Length::Fixed(26.0))
                .clip(true)
                .into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if stash_entries.is_empty() {
        list_col = list_col.push(view_utils::empty_list_hint("No stashes", c.muted));
    } else {
        for entry_el in stash_entries {
            list_col = list_col.push(entry_el);
        }
    }

    let content = column![
        header_row,
        input_row,
        scrollable(list_col)
            .height(Length::Fill)
            .direction(view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar),
    ]
    .width(Length::Fill);

    container(content).width(Length::Fill).into()
}
