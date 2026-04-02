use ratatui::style::Color;

use gitkraft_core::{AppTheme, Rgb};

/// All the colour slots the UI needs.  Every view file should obtain a
/// `UiTheme` via `app.theme()` and use these fields instead of hard-coding
/// `Color::*` values.
pub struct UiTheme {
    pub border_active: Color,
    pub border_inactive: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub diff_add: Color,
    pub diff_del: Color,
    pub diff_context: Color,
    pub diff_hunk: Color,
    pub sel_bg: Color,
    pub bg: Color,
}

impl UiTheme {
    /// Convert from the core's platform-agnostic theme.
    pub fn from_core(t: &AppTheme) -> Self {
        Self {
            border_active: rgb_to_color(t.accent),
            border_inactive: rgb_to_color(t.border),
            text_primary: rgb_to_color(t.text_primary),
            text_secondary: rgb_to_color(t.text_secondary),
            text_muted: rgb_to_color(t.text_muted),
            accent: rgb_to_color(t.accent),
            success: rgb_to_color(t.success),
            warning: rgb_to_color(t.warning),
            error: rgb_to_color(t.error),
            diff_add: rgb_to_color(t.diff_add),
            diff_del: rgb_to_color(t.diff_del),
            diff_context: rgb_to_color(t.diff_context),
            diff_hunk: rgb_to_color(t.diff_hunk),
            sel_bg: rgb_to_color(t.selection),
            bg: rgb_to_color(t.background),
        }
    }
}

/// Convert a core [`Rgb`] to a ratatui [`Color`].
fn rgb_to_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

/// Map a theme index (0–26) to a concrete `UiTheme` by delegating to the
/// canonical definitions in `gitkraft_core`.
pub fn theme_for_index(index: usize) -> UiTheme {
    UiTheme::from_core(&gitkraft_core::theme_by_index(index))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_core::THEME_COUNT;

    #[test]
    fn all_27_themes_resolve() {
        for i in 0..THEME_COUNT {
            let theme = theme_for_index(i);
            // Just verify it doesn't panic and returns distinct active/inactive borders
            assert_ne!(
                format!("{:?}", theme.border_active),
                format!("{:?}", theme.border_inactive),
                "theme index {i} has identical active and inactive border colors"
            );
        }
    }

    #[test]
    fn out_of_range_returns_default() {
        let d = theme_for_index(0);
        let oob = theme_for_index(999);
        assert_eq!(
            format!("{:?}", d.border_active),
            format!("{:?}", oob.border_active)
        );
    }

    #[test]
    fn from_core_round_trips_rgb() {
        let core_theme = gitkraft_core::theme_by_index(8); // Dracula
        let ui = UiTheme::from_core(&core_theme);
        assert_eq!(
            ui.bg,
            Color::Rgb(
                core_theme.background.r,
                core_theme.background.g,
                core_theme.background.b
            )
        );
        assert_eq!(
            ui.accent,
            Color::Rgb(
                core_theme.accent.r,
                core_theme.accent.g,
                core_theme.accent.b
            )
        );
    }
}
