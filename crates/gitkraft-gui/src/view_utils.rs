//! Shared view utilities for the GitKraft GUI.
//!
//! Helpers in this module are used by multiple feature views and are kept here
//! to avoid duplication and make them easy to test in isolation.

// ── Text truncation ───────────────────────────────────────────────────────────

/// Truncate `s` to fit within `available_px` pixels at the given average
/// `px_per_char` rate, appending "…" when the string is shortened.
///
/// # Behaviour
/// - If `available_px` is zero or negative the string is truncated to `""`.
/// - If the string already fits it is returned unchanged (no allocation when
///   ownership is not needed — callers that already own a `String` can pass
///   `s.as_str()` and get the original value back as a new `String`; the cost
///   is one clone at most).
/// - The "…" counts as **one character** in the budget, so the returned string
///   always fits within `available_px`.
///
/// # Example
/// ```
/// # use gitkraft_gui::view_utils::truncate_to_fit;
/// assert_eq!(truncate_to_fit("hello", 100.0, 7.0), "hello");
/// assert_eq!(truncate_to_fit("hello world", 30.0, 7.0), "hel…");
/// ```
pub fn truncate_to_fit(s: &str, available_px: f32, px_per_char: f32) -> String {
    if available_px <= 0.0 || px_per_char <= 0.0 {
        return String::new();
    }

    let max_chars = (available_px / px_per_char).floor() as usize;
    let char_count = s.chars().count();

    if char_count <= max_chars {
        // Fits as-is.
        s.to_string()
    } else if max_chars <= 1 {
        // Only room for the ellipsis itself.
        "…".to_string()
    } else {
        // Take (max_chars - 1) characters then append "…".
        let mut out: String = s.chars().take(max_chars - 1).collect();
        out.push('…');
        out
    }
}

// ── Scrollbar helper ──────────────────────────────────────────────────────

/// Standard thin vertical scrollbar direction used across all sidebar panels.
///
/// Apply as: `scrollable(content).direction(thin_scrollbar()).style(overlay_scrollbar)`
pub fn thin_scrollbar() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(4),
    )
}

// ── Context menu helpers ──────────────────────────────────────────────────

/// Thin horizontal separator line for context menus.
pub fn context_menu_separator<'a, M: 'a>() -> iced::Element<'a, M> {
    iced::widget::container(iced::widget::Space::new(0, 1))
        .padding(iced::Padding {
            top: 4.0,
            right: 0.0,
            bottom: 4.0,
            left: 0.0,
        })
        .width(iced::Length::Fill)
        .into()
}

/// Header label for a context menu panel.
pub fn context_menu_header<'a, M: 'a>(label: String, muted: iced::Color) -> iced::Element<'a, M> {
    iced::widget::container(iced::widget::text(label).size(12).color(muted))
        .padding(iced::Padding {
            top: 8.0,
            right: 14.0,
            bottom: 6.0,
            left: 14.0,
        })
        .width(iced::Length::Fill)
        .into()
}

// ── Centered placeholder ──────────────────────────────────────────────────

/// A centered placeholder with an icon and a label, used for empty/loading states.
pub fn centered_placeholder<'a>(
    icon_char: char,
    icon_size: u16,
    label_text: &str,
    muted: iced::Color,
) -> iced::Element<'a, crate::message::Message> {
    use iced::widget::{column, container, text, Space};
    use iced::{Alignment, Length};

    let icon_widget = icon!(icon_char, icon_size, muted);
    let label = text(label_text.to_string()).size(14).color(muted);

    container(
        column![icon_widget, Space::new(0, 8), label]
            .spacing(4)
            .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

// ── Button helpers ────────────────────────────────────────────────────────

/// Conditionally attach an `on_press` handler to a button.
///
/// Replaces the common pattern of building the same button twice in an
/// if/else just to add or omit `.on_press(msg)`.
pub fn on_press_maybe<'a>(
    btn: iced::widget::Button<'a, crate::message::Message>,
    msg: Option<crate::message::Message>,
) -> iced::widget::Button<'a, crate::message::Message> {
    match msg {
        Some(m) => btn.on_press(m),
        None => btn,
    }
}

// ── Collapsible section header ────────────────────────────────────────────

/// A collapsible section header with a chevron, label, count, and toggle message.
/// Used for "Local (N)" / "Remote (N)" in the branches sidebar.
pub fn collapsible_header<'a>(
    expanded: bool,
    label: &'a str,
    count: usize,
    on_toggle: crate::message::Message,
    muted: iced::Color,
) -> iced::Element<'a, crate::message::Message> {
    use iced::widget::{button, row, text, Space};
    use iced::Alignment;

    let chevron_char = if expanded {
        crate::icons::CHEVRON_DOWN
    } else {
        crate::icons::CHEVRON_RIGHT
    };
    let chevron = icon!(chevron_char, 11, muted);

    button(
        row![
            chevron,
            Space::new(4, 0),
            text(label).size(11).color(muted),
            Space::new(4, 0),
            text(format!("({count})")).size(10).color(muted),
        ]
        .align_y(Alignment::Center),
    )
    .padding([4, 8])
    .width(iced::Length::Fill)
    .style(crate::theme::ghost_button)
    .on_press(on_toggle)
    .into()
}

// ── Toolbar button ────────────────────────────────────────────────────────

/// A toolbar button with an icon and label text.
/// Used in the header toolbar for Refresh, Open, Close, etc.
pub fn toolbar_btn<'a>(
    icon_widget: impl Into<iced::Element<'a, crate::message::Message>>,
    label: &'a str,
    msg: crate::message::Message,
) -> iced::widget::Button<'a, crate::message::Message> {
    use iced::widget::{button, row, text, Space};
    use iced::Alignment;

    button(
        row![icon_widget.into(), Space::new(4, 0), text(label).size(12)].align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(crate::theme::toolbar_button)
    .on_press(msg)
}

// ── Panel wrapper ─────────────────────────────────────────────────────────

/// Wrap content in a full-size container with the surface background style.
pub fn surface_panel<'a>(
    content: impl Into<iced::Element<'a, crate::message::Message>>,
    width: iced::Length,
) -> iced::Element<'a, crate::message::Message> {
    iced::widget::container(content)
        .width(width)
        .height(iced::Length::Fill)
        .style(crate::theme::surface_style)
        .into()
}

// ── Empty list hint ───────────────────────────────────────────────────────

/// Centered muted text shown when a list has no items.
pub fn empty_list_hint<'a>(
    label: &'a str,
    muted: iced::Color,
) -> iced::Element<'a, crate::message::Message> {
    iced::widget::container(iced::widget::text(label.to_string()).size(12).color(muted))
        .padding([12, 8])
        .width(iced::Length::Fill)
        .center_x(iced::Length::Fill)
        .into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::truncate_to_fit;

    // ── fits without truncation ───────────────────────────────────────────

    #[test]
    fn short_string_returned_unchanged() {
        assert_eq!(truncate_to_fit("hi", 100.0, 7.0), "hi");
    }

    #[test]
    fn string_exactly_at_limit_is_not_truncated() {
        // 3 chars × 10 px/char = 30 px → max_chars = 3, char_count = 3 → fits
        assert_eq!(truncate_to_fit("abc", 30.0, 10.0), "abc");
    }

    #[test]
    fn empty_string_returned_unchanged() {
        assert_eq!(truncate_to_fit("", 100.0, 7.0), "");
    }

    // ── truncation with ellipsis ──────────────────────────────────────────

    #[test]
    fn long_string_truncated_with_ellipsis() {
        // 30px / 7px = 4 chars max; keeps 3 + "…"
        let result = truncate_to_fit("hello world", 30.0, 7.0);
        assert_eq!(result, "hel…");
        assert!(result.ends_with('…'));
    }

    #[test]
    fn result_respects_max_char_budget() {
        // 50px / 10px = 5 chars max; result must be ≤ 5 chars (counting "…" as 1)
        let result = truncate_to_fit("abcdefghij", 50.0, 10.0);
        assert_eq!(result.chars().count(), 5);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn one_char_over_limit_gives_ellipsis_only() {
        // 10px / 10px = 1 char max → only room for "…"
        let result = truncate_to_fit("ab", 10.0, 10.0);
        assert_eq!(result, "…");
    }

    #[test]
    fn branch_name_with_slash_truncated_correctly() {
        // Typical sidebar scenario: long branch name at 7.5 px/char, 120 px
        // available → max 16 chars; keeps 15 + "…"
        let name = "mario/MARIO-3924_global_design_system_library_publishing";
        let result = truncate_to_fit(name, 120.0, 7.5);
        assert_eq!(result.chars().count(), 16);
        assert!(result.ends_with('…'));
        assert!(result.starts_with("mario/MARIO-392"));
    }

    #[test]
    fn commit_summary_short_enough_shows_fully() {
        let summary = "Fix typo in README";
        // 500 px available, 7 px/char → 71 chars max — summary (18 chars) fits
        let result = truncate_to_fit(summary, 500.0, 7.0);
        assert_eq!(result, summary);
    }

    #[test]
    fn commit_summary_too_long_gets_ellipsis() {
        let summary =
            "CARTS-2149: Serialize MediaPickerOptions nav args as URI-encoded JSON strings";
        // 300 px / 7 px = 42 chars max; keeps 41 + "…"
        let result = truncate_to_fit(summary, 300.0, 7.0);
        assert_eq!(result.chars().count(), 42);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn stash_message_truncated_correctly() {
        let msg = "WIP on mario/MARIO-3869_fix_icons_svg_parsing: f51116a10d4";
        // 200 px / 6.5 px = 30 chars max; keeps 29 + "…"
        let result = truncate_to_fit(msg, 200.0, 6.5);
        assert_eq!(result.chars().count(), 30);
        assert!(result.ends_with('…'));
        assert!(result.starts_with("WIP on mario/MARIO-3869_fix_i"));
    }

    // ── edge / boundary cases ─────────────────────────────────────────────

    #[test]
    fn zero_available_px_returns_empty() {
        assert_eq!(truncate_to_fit("hello", 0.0, 7.0), "");
    }

    #[test]
    fn negative_available_px_returns_empty() {
        assert_eq!(truncate_to_fit("hello", -10.0, 7.0), "");
    }

    #[test]
    fn zero_px_per_char_returns_empty() {
        assert_eq!(truncate_to_fit("hello", 100.0, 0.0), "");
    }

    #[test]
    fn single_char_string_fits_in_one_char_budget() {
        // 10px / 10px = 1 char max, string is 1 char → fits
        assert_eq!(truncate_to_fit("a", 10.0, 10.0), "a");
    }

    #[test]
    fn unicode_multibyte_chars_counted_by_char_not_byte() {
        // "héllo" = 5 chars; 40px / 10px = 4 max → keeps 3 + "…"
        let result = truncate_to_fit("héllo", 40.0, 10.0);
        assert_eq!(result.chars().count(), 4);
        assert_eq!(result, "hél…");
    }

    #[test]
    fn unicode_ellipsis_in_source_not_duplicated() {
        // String already short enough — no extra "…" appended
        let s = "short…";
        let result = truncate_to_fit(s, 200.0, 7.0);
        assert_eq!(result, s);
    }

    #[test]
    fn very_small_px_per_char_truncates_to_many_chars() {
        // 200px / 1px = 200 chars max; 10-char string fits
        let result = truncate_to_fit("helloworld", 200.0, 1.0);
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn fractional_px_per_char_floors_correctly() {
        // 25px / 7.5px = 3.33 → floor to 3; string "abcd" (4) → truncate
        let result = truncate_to_fit("abcd", 25.0, 7.5);
        assert_eq!(result, "ab…");
    }
}
