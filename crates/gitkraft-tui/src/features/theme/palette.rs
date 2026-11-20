use std::sync::OnceLock;

use ratatui::style::Color;
use tui_nuances::Theme;

use gitkraft_core::SYSTEM_THEME_NAME;

/// All the colour slots the UI needs.  Every view file should obtain a
/// `UiTheme` via `app.theme()` and use these fields instead of hard-coding
/// `Color::*` values.
#[derive(Debug)]
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
    /// Eight colours cycled across branch lanes in the commit graph.
    pub graph_colors: [Color; 8],
}

impl UiTheme {
    /// Derive the UI colours from a [`tui_nuances::Theme`] — the single source
    /// of truth for every palette.
    pub fn from_nuances(t: &Theme) -> Self {
        Self {
            border_active: t.border_focused,
            border_inactive: t.border,
            text_primary: t.foreground,
            text_secondary: t.secondary,
            text_muted: t.muted,
            accent: t.accent,
            success: t.success,
            warning: t.warning,
            error: t.error,
            diff_add: t.success,
            diff_del: t.error,
            diff_context: t.muted,
            diff_hunk: t.info,
            sel_bg: t.selection_bg,
            bg: t.background.unwrap_or(Color::Reset),
            graph_colors: std::array::from_fn(|i| t.color(i)),
        }
    }
}

/// A Dodai design-system theme as a `tui-nuances` one, so the TUI draws it like any other.
fn dodai_theme(index: usize) -> Theme {
    let t = gitkraft_core::theme_by_index(index);
    let c = |r: gitkraft_core::Rgb| Color::Rgb(r.r, r.g, r.b);
    Theme {
        name: gitkraft_core::THEME_NAMES[index].to_string(),
        background: Some(c(t.background)),
        surface: Some(c(t.surface)),
        foreground: c(t.text_primary),
        muted: c(t.text_muted),
        disabled: c(t.text_muted),
        border: c(t.border),
        border_focused: c(t.accent),
        title: c(t.accent),
        primary: c(t.accent),
        secondary: c(t.text_secondary),
        accent: c(t.accent),
        success: c(t.success),
        warning: c(t.warning),
        error: c(t.error),
        info: c(t.diff_hunk),
        selection_bg: c(t.selection),
        selection_fg: c(t.text_primary),
        highlight: c(t.warning),
        palette: t.graph_colors.iter().map(|r| c(*r)).collect(),
        bull: c(t.success),
        bear: c(t.error),
        neutral: c(t.text_muted),
        axis: c(t.border),
        grid: c(t.border),
        crosshair: c(t.accent),
        tooltip_fg: c(t.text_primary),
        tooltip_bg: c(t.surface),
    }
}

/// The themes in picker order: the Dodai design system first, then the `tui-nuances` ones.
fn themes() -> &'static [Theme] {
    static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let first = gitkraft_core::THEME_NAMES.len() - dodai_count();
        let mut all: Vec<Theme> = (first..gitkraft_core::THEME_NAMES.len())
            .map(dodai_theme)
            .collect();
        all.extend(Theme::all());
        all
    })
}

fn dodai_count() -> usize {
    gitkraft_core::THEME_COUNT - gitkraft_core::DODAI_FIRST
}

/// Names of every theme, in picker order (Dodai first).
pub fn theme_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut v: Vec<&'static str> =
            gitkraft_core::THEME_NAMES[gitkraft_core::DODAI_FIRST..].to_vec();
        v.extend(Theme::names());
        v
    })
}

/// Number of themes (excluding the "System" pseudo-theme).
pub fn theme_count() -> usize {
    theme_names().len()
}

/// Index of a theme by name (case/space/accent-insensitive); the default Dodai theme if unknown.
pub fn index_by_name(name: &str) -> usize {
    let names = theme_names();
    let exact = |wanted: &str| names.iter().position(|n| n.eq_ignore_ascii_case(wanted));
    exact(name)
        .or_else(|| name.strip_prefix("Dodai · ").and_then(exact))
        .or_else(|| Theme::by_name(name).and_then(|t| names.iter().position(|n| *n == t.name)))
        .unwrap_or_else(default_index)
}

/// Index of the theme a fresh install starts with.
pub fn default_index() -> usize {
    theme_names()
        .iter()
        .position(|n| *n == gitkraft_core::THEME_NAMES[gitkraft_core::DEFAULT_THEME])
        .unwrap_or(0)
}

/// Index of the theme matching the OS appearance (dark if undetectable).
pub fn system_index() -> usize {
    let dark = gitkraft_core::system_dark_cached().unwrap_or(true);
    index_by_name(if dark { "default" } else { "default-light" })
}

/// Number of picker rows: "System" followed by every theme.
pub fn picker_len() -> usize {
    theme_count() + 1
}

/// Label of picker row `i` (`0` = "System", `n + 1` = preset `n`).
pub fn picker_name(i: usize) -> &'static str {
    if i == 0 {
        SYSTEM_THEME_NAME
    } else {
        theme_names().get(i - 1).copied().unwrap_or("default")
    }
}

/// Map a theme index to a concrete `UiTheme` (out of range → first theme).
/// The raw `tui-nuances` theme at `index` (falls back to the first theme).
pub fn nuances_theme_for_index(index: usize) -> Theme {
    let all = themes();
    all.get(index).unwrap_or(&all[0]).clone()
}

pub fn theme_for_index(index: usize) -> UiTheme {
    let all = themes();
    UiTheme::from_nuances(all.get(index).unwrap_or(&all[0]))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_themes_resolve() {
        for i in 0..theme_count() {
            let theme = theme_for_index(i);
            assert_ne!(
                format!("{:?}", theme.border_active),
                format!("{:?}", theme.border_inactive),
                "theme index {i} has identical active and inactive border colors"
            );
        }
    }

    #[test]
    fn nuances_is_the_source_and_covers_gitkraft_themes() {
        assert!(theme_count() >= 43);
        for n in [
            "dracula",
            "nord",
            "synthwave",
            "kanagawa-wave",
            "default-light",
        ] {
            assert!(theme_names().contains(&n), "{n} missing");
        }
        // Legacy gitkraft display names still resolve.
        assert_eq!(
            index_by_name("Catppuccin Frappé"),
            index_by_name("catppuccin-frappe")
        );
        assert_ne!(index_by_name("Dracula"), 0);
    }

    #[test]
    fn picker_lists_system_first_then_all_themes() {
        assert_eq!(picker_len(), theme_count() + 1);
        assert_eq!(picker_name(0), "System");
        assert_eq!(picker_name(1), theme_names()[0]);
        assert_eq!(
            picker_name(picker_len() - 1),
            theme_names()[theme_count() - 1]
        );
    }

    #[test]
    fn out_of_range_returns_first() {
        let d = theme_for_index(0);
        let oob = theme_for_index(999);
        assert_eq!(
            format!("{:?}", d.border_active),
            format!("{:?}", oob.border_active)
        );
    }

    #[test]
    fn from_nuances_maps_slots() {
        let t = Theme::dracula();
        let ui = UiTheme::from_nuances(&t);
        assert_eq!(ui.accent, t.accent);
        assert_eq!(Some(ui.bg), t.background);
        assert_eq!(ui.graph_colors[0], t.color(0));
    }

    #[test]
    fn the_dodai_themes_come_first_and_a_fresh_install_starts_on_dodai_dark() {
        assert!(theme_names()[0].starts_with("Dodai"));
        assert_eq!(theme_names().len(), dodai_count() + Theme::names().len());
        assert_eq!(theme_names()[default_index()], "Dodai · Dark");
        assert_eq!(
            index_by_name("Dodai · Cyberpunk · Toxic"),
            index_by_name("Cyberpunk · Toxic")
        );
        assert_ne!(index_by_name("Cyberpunk · Toxic"), default_index());
    }
}
