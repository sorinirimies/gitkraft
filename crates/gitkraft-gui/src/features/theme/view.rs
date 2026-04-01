//! Theme selector widget for the GitKraft GUI.
//!
//! Provides a [`pick_list`] drop-down that lets the user switch between
//! all built-in Iced themes at runtime.

use iced::widget::pick_list;
use iced::{Element, Length, Theme};

use crate::message::Message;

/// All available built-in Iced themes.
pub fn all_themes() -> Vec<Theme> {
    vec![
        Theme::Dark,
        Theme::Light,
        Theme::Dracula,
        Theme::Nord,
        Theme::SolarizedLight,
        Theme::SolarizedDark,
        Theme::GruvboxLight,
        Theme::GruvboxDark,
        Theme::CatppuccinLatte,
        Theme::CatppuccinFrappe,
        Theme::CatppuccinMacchiato,
        Theme::CatppuccinMocha,
        Theme::TokyoNight,
        Theme::TokyoNightStorm,
        Theme::TokyoNightLight,
        Theme::KanagawaWave,
        Theme::KanagawaDragon,
        Theme::KanagawaLotus,
        Theme::Moonfly,
        Theme::Nightfly,
        Theme::Oxocarbon,
    ]
}

/// Create a theme selector [`pick_list`] widget.
///
/// The widget displays the name of `current_theme` and, when opened, lists
/// every theme returned by [`all_themes`]. Selecting a new entry emits
/// [`Message::ThemeChanged`].
pub fn theme_selector(current_theme: &Theme) -> Element<'static, Message> {
    pick_list(
        all_themes(),
        Some(current_theme.clone()),
        Message::ThemeChanged,
    )
    .placeholder("Select theme")
    .text_size(13.0)
    .width(Length::Fixed(180.0))
    .into()
}
