//! Stash list view — shows stash entries in the sidebar with save, pop, and
//! drop actions for each entry.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the stash panel (typically shown in the sidebar beneath branches).
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let header_icon = text('\u{F577}') // stack icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let header_label = text("Stashes").size(14).color(theme::TEXT_PRIMARY);

    let count_label = text(format!("({})", state.stashes.len()))
        .size(11)
        .color(theme::MUTED);

    let save_icon = text('\u{F4FA}') // plus-circle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::GREEN);

    let save_btn = button(save_icon)
        .padding([2, 6])
        .on_press(Message::StashSave);

    let header_row = row![
        header_icon,
        Space::with_width(6),
        header_label,
        Space::with_width(4),
        count_label,
        Space::with_width(Length::Fill),
        save_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 10]);

    // ── Stash message input ───────────────────────────────────────────────
    let stash_input = text_input("Stash message (optional)…", &state.stash_message)
        .on_input(Message::StashMessageChanged)
        .padding(4)
        .size(12);

    let input_row = container(stash_input).padding([2, 10]).width(Length::Fill);

    // ── Stash entries ─────────────────────────────────────────────────────
    let stash_entries: Vec<Element<'_, Message>> = state
        .stashes
        .iter()
        .map(|entry| {
            let index_label = text(format!("stash@{{{}}}", entry.index))
                .size(11)
                .color(theme::ACCENT)
                .font(iced::Font::MONOSPACE);

            let msg_text = if entry.message.len() > 40 {
                format!("{}…", &entry.message[..39])
            } else {
                entry.message.clone()
            };
            let msg_label = text(msg_text).size(11).color(theme::TEXT_SECONDARY);

            let pop_icon = text('\u{F117}') // box-arrow-up
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(11)
                .color(theme::GREEN);

            let pop_btn = button(pop_icon)
                .padding([2, 4])
                .on_press(Message::StashPop(entry.index));

            let drop_icon = text('\u{F5DE}') // trash
                .font(iced_fonts::BOOTSTRAP_FONT)
                .size(11)
                .color(theme::RED);

            let drop_btn = button(drop_icon)
                .padding([2, 4])
                .on_press(Message::StashDrop(entry.index));

            let entry_row = row![
                index_label,
                Space::with_width(6),
                msg_label,
                Space::with_width(Length::Fill),
                pop_btn,
                Space::with_width(2),
                drop_btn,
            ]
            .align_y(Alignment::Center)
            .padding([3, 8]);

            container(entry_row).width(Length::Fill).into()
        })
        .collect();

    let mut list_col = column![].spacing(1).width(Length::Fill);

    if stash_entries.is_empty() {
        let empty_msg = text("No stashes").size(12).color(theme::MUTED);
        list_col = list_col.push(container(empty_msg).padding([8, 10]).width(Length::Fill));
    } else {
        for entry_el in stash_entries {
            list_col = list_col.push(entry_el);
        }
    }

    let content = column![
        header_row,
        input_row,
        scrollable(list_col).height(Length::Fill),
    ]
    .width(Length::Fill);

    container(content).width(Length::Fill).into()
}
