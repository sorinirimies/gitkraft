//! Application metadata shown in the "About" dialogs of the GUI and TUI.

/// Application name.
pub const APP_NAME: &str = "GitKraft";
/// Current version (taken from the workspace `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Short project description.
pub const DESCRIPTION: &str =
    "A modern Git IDE with a desktop GUI (Iced) and a terminal UI (Ratatui), \
     sharing one Rust core.";
/// Author's display name.
pub const AUTHOR_NAME: &str = "Sorin Irimies";
/// Author's GitHub handle.
pub const AUTHOR_GITHUB: &str = "sorinirimies";
/// Author's GitHub profile URL.
pub const AUTHOR_URL: &str = "https://github.com/sorinirimies";
/// Project repository URL.
pub const REPO_URL: &str = "https://github.com/sorinirimies/gitkraft";
/// License identifier.
pub const LICENSE: &str = "MIT";

/// Version label such as `v1.1.6`.
pub fn version_label() -> String {
    format!("v{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_label_has_prefix() {
        assert_eq!(version_label(), format!("v{}", VERSION));
        assert!(!VERSION.is_empty());
    }
}
