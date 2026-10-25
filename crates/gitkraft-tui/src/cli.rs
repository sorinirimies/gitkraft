//! Command-line handling for theme selection (`--theme`, `--list-themes`).

use crate::features::theme::palette::theme_names as nuances_names;
use gitkraft_core::{is_system_name, SYSTEM_THEME_NAME};

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

/// Parsed command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub repo_path: Option<String>,
    pub theme: Option<String>,
    pub list_themes: bool,
}

/// Parse arguments (without the program name).
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--list-themes" {
            cli.list_themes = true;
        } else if a == "--theme" {
            cli.theme = Some(it.next().ok_or("--theme requires a value")?);
        } else if let Some(v) = a.strip_prefix("--theme=") {
            cli.theme = Some(v.to_string());
        } else if a.starts_with("--") {
            return Err(format!("unknown option '{a}'"));
        } else if cli.repo_path.is_none() {
            cli.repo_path = Some(a);
        }
    }
    Ok(cli)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Cli, String> {
        parse_args(a.iter().map(|s| s.to_string()))
    }

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

    #[test]
    fn parses_flags_and_repo() {
        assert_eq!(p(&[]).unwrap(), Cli::default());
        let c = p(&["/tmp/r", "--theme", "Nord"]).unwrap();
        assert_eq!(c.repo_path.as_deref(), Some("/tmp/r"));
        assert_eq!(c.theme.as_deref(), Some("Nord"));
        assert_eq!(
            p(&["--theme=System"]).unwrap().theme.as_deref(),
            Some("System")
        );
        assert!(p(&["--list-themes"]).unwrap().list_themes);
        assert!(p(&["--theme"]).is_err());
        assert!(p(&["--bogus"]).is_err());
    }
}
