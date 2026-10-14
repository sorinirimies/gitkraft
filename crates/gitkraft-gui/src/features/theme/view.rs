//! Theme selector widget for the GitKraft GUI.
//!
//! Provides a [`pick_list()`] drop-down that lets the user switch between
//! "System" plus all unified themes defined in `gitkraft_core` at runtime.

use iced::widget::pick_list;
use iced::{Element, Length};

use crate::message::Message;

/// A wrapper around a theme index that implements `Display` for the pick-list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeChoice {
    pub index: usize,
    pub name: &'static str,
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

/// All core presets as [`ThemeChoice`] values, derived from the canonical
/// core definitions.
pub(crate) fn all_themes() -> Vec<ThemeChoice> {
    gitkraft_core::THEME_NAMES
        .iter()
        .enumerate()
        .map(|(i, name)| ThemeChoice { index: i, name })
        .collect()
}

/// [`all_themes`] preceded by a "System" entry (index `THEME_COUNT`).
pub(crate) fn theme_choices() -> Vec<ThemeChoice> {
    let mut v = vec![ThemeChoice {
        index: gitkraft_core::THEME_COUNT,
        name: gitkraft_core::SYSTEM_THEME_NAME,
    }];
    v.extend(all_themes());
    v
}

/// Create a theme selector [`pick_list()`] widget.
///
/// Lists "System" first, then every preset. Selecting an entry emits
/// [`Message::ThemeChanged`] (`THEME_COUNT` for System, else the preset index).
pub(crate) fn theme_selector(
    current_theme_index: usize,
    follow_system: bool,
    width: f32,
) -> Element<'static, Message> {
    let choices = theme_choices();
    let selected = if follow_system {
        choices.first().cloned()
    } else {
        // +1: the "System" entry is listed first.
        choices.get(current_theme_index + 1).cloned()
    };

    pick_list(choices, selected, |choice| {
        Message::ThemeChanged(choice.index)
    })
    .placeholder("Select theme")
    .text_size(13.0)
    .width(Length::Fixed(width))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_lead_with_system() {
        let c = theme_choices();
        assert_eq!(c.len(), gitkraft_core::THEME_COUNT + 1);
        assert_eq!(c[0].index, gitkraft_core::THEME_COUNT);
        assert_eq!(c[0].name, "System");
        assert_eq!(c[1].index, 0);
    }

    #[test]
    fn all_themes_match_core() {
        let t = all_themes();
        assert_eq!(t.len(), gitkraft_core::THEME_COUNT);
        assert!(t.iter().enumerate().all(|(i, c)| c.index == i));
    }
}
