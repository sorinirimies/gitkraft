//! Tab bar widget — renders a horizontal row of repository tabs at the top of
//! the window, similar to GitKraken's tab bar.

use iced::widget::{button, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::theme::ThemeColors;

/// Render the tab bar above the header toolbar.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let c = state.colors();

    let mut tabs_row = row![].spacing(0).align_y(Alignment::Center);

    for (idx, tab) in state.tabs.iter().enumerate() {
        let is_active = idx == state.active_tab;
        let name = tab.display_name().to_string();

        // Repo icon
        let icon = if tab.has_repo() {
            icon!(
                icons::FOLDER_OPEN,
                12,
                if is_active { c.accent } else { c.muted }
            )
        } else {
            icon!(
                icons::PERSON_FILL,
                12,
                if is_active { c.accent } else { c.muted }
            )
        };

        // Tab label
        let label = text(name).size(12).color(if is_active {
            c.text_primary
        } else {
            c.text_secondary
        });

        // Close button (only show if there's more than 1 tab)
        let close_btn: Element<'_, Message> = if state.tabs.len() > 1 {
            button(icon!(icons::X_CIRCLE, 10, c.muted))
                .padding([0, 4])
                .style(theme::ghost_button)
                .on_press(Message::CloseTab(idx))
                .into()
        } else {
            Space::with_width(0).into()
        };

        let tab_content = row![
            icon,
            Space::with_width(6),
            label,
            Space::with_width(4),
            close_btn
        ]
        .align_y(Alignment::Center);

        let tab_btn = button(tab_content)
            .padding([6, 12])
            .style(if is_active {
                theme::active_tab_button
            } else {
                theme::ghost_button
            })
            .on_press(Message::SwitchTab(idx));

        tabs_row = tabs_row.push(tab_btn);
    }

    // "+" button to add a new tab
    let new_tab_btn = button(icon!(icons::PLUS_CIRCLE, 14, c.text_secondary))
        .padding([6, 10])
        .style(theme::ghost_button)
        .on_press(Message::NewTab);

    tabs_row = tabs_row.push(new_tab_btn);

    let scrollable_tabs = scrollable(tabs_row)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .style(crate::theme::overlay_scrollbar)
        .width(Length::Fill);

    container(scrollable_tabs)
        .width(Length::Fill)
        .style(tab_bar_style)
        .into()
}

/// Dark background style for the tab bar — slightly darker than the header.
fn tab_bar_style(theme: &iced::Theme) -> container::Style {
    let c = ThemeColors::from_theme(theme);
    container::Style {
        background: Some(iced::Background::Color(iced::Color {
            r: (c.bg.r - 0.02).max(0.0),
            g: (c.bg.g - 0.02).max(0.0),
            b: (c.bg.b - 0.02).max(0.0),
            a: 1.0,
        })),
        border: iced::Border {
            color: c.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
