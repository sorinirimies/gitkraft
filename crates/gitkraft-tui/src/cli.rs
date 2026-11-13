//! Theme names for `--theme` and `--list-themes` (the arguments themselves are parsed in `forge_cli`).

use crate::features::theme::palette::theme_names as nuances_names;
use gitkraft_core::{SYSTEM_THEME_NAME, is_system_name};

/// Every selectable theme name: `System` followed by all core presets.
pub fn theme_names() -> Vec<&'static str> {
    std::iter::once(SYSTEM_THEME_NAME)
        .chain(nuances_names().iter().copied())
        .collect()
}

/// Resolve a user-supplied theme name (case-insensitive) to its canonical
/// persisted name (`"System"` or a preset name).
pub fn resolve_theme(name: &str) -> Option<&'static str> {
    if is_system_name(name) {
        return Some(SYSTEM_THEME_NAME);
    }
    let want = tui_nuances::Theme::by_name(name)?.name;
    nuances_names().iter().copied().find(|n| *n == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_start_with_system_and_cover_all_presets() {
        let n = theme_names();
        assert_eq!(n[0], "System");
        assert_eq!(n.len(), crate::features::theme::palette::theme_count() + 1);
    }

    #[test]
    fn resolve_is_case_insensitive() {
        assert_eq!(resolve_theme("system"), Some("System"));
        assert_eq!(resolve_theme("Dracula"), Some("dracula"));
        assert_eq!(resolve_theme("Tokyo Night"), Some("tokyo-night"));
        assert_eq!(resolve_theme("nope"), None);
    }
}
