//! Tab bar widget — renders a horizontal row of repository tabs at the top of
//! the window, similar to GitKraken's tab bar.
//!
//! ## Layout per tab
//!
//! ```text
//!  ┌──────────────────────────────────────────┐
//!  │  [tab_btn: icon + label]  │  [close_btn] │
//!  └──────────────────────────────────────────┘
//! ```
//!
//! **Important:** the close button is a *sibling* of the tab-switch button,
//! NOT nested inside it.  Nesting a `button` inside another `button` in Iced
//! can cause the outer button's event handling to shadow the inner one for the
//! currently-active tab (the outer `SwitchTab` message fires a synchronous
//! update+view cycle which resets the inner button's `is_pressed` state before
//! the `ButtonReleased` event arrives, so `CloseTab` never fires).  Keeping
//! them as siblings at the same row level eliminates the race entirely.

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

    let mut tabs_row = row![].spacing(2).align_y(Alignment::Center);

    for (idx, tab) in state.tabs.iter().enumerate() {
        let is_active = idx == state.active_tab;
        let name = tab.display_name();

        // ── Icon: accent when this tab is selected ────────────────────────
        let icon_color = if is_active { c.accent } else { c.muted };
        let icon = if tab.has_repo() {
            icon!(icons::FOLDER_OPEN, 12, icon_color)
        } else {
            icon!(icons::PERSON_FILL, 12, icon_color)
        };

        // ── Label: accent when selected, secondary otherwise ──────────────
        let label_color = if is_active {
            c.accent
        } else {
            c.text_secondary
        };
        let label = text(name).size(12).color(label_color);

        // ── Tab label button (icon + name only — NO close button inside) ──
        //
        // Keeping the close button *outside* this button is the key fix.
        // If `close_btn` were nested here, clicking × on the active tab
        // could have its `is_pressed` flag wiped by the intervening
        // SwitchTab update before ButtonReleased arrives.
        let tab_content = row![icon, Space::new().width(6), label].align_y(Alignment::Center);

        let tab_style = if is_active {
            theme::active_tab_button as fn(&iced::Theme, button::Status) -> button::Style
        } else {
            theme::ghost_button as fn(&iced::Theme, button::Status) -> button::Style
        };

        let tab_btn = button(tab_content)
            .padding([6, 10])
            .style(tab_style)
            .on_press(Message::SwitchTab(idx));

        // ── Close button — sibling of tab_btn, not a child ───────────────
        if state.tabs.len() > 1 {
            // Use accent colour on the active tab's × so it stands out.
            let close_color = if is_active { c.accent } else { c.muted };
            let close_btn = button(icon!(icons::X_CIRCLE, 10, close_color))
                .padding([2, 4])
                .style(theme::ghost_button)
                .on_press(Message::CloseTab(idx));

            tabs_row = tabs_row.push(row![tab_btn, close_btn].align_y(Alignment::Center));
        } else {
            tabs_row = tabs_row.push(tab_btn);
        }
    }

    // "+" button to open a new empty tab
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
