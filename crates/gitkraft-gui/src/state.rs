use std::path::PathBuf;

use gitkraft_core::*;

use crate::theme::ThemeColors;

/// Top-level application state for the GitKraft GUI.
pub struct GitKraft {
    // ── Repository ────────────────────────────────────────────────────────
    /// Path to the currently opened repository (workdir root).
    pub repo_path: Option<PathBuf>,
    /// High-level information about the opened repository.
    pub repo_info: Option<RepoInfo>,

    // ── Branches ──────────────────────────────────────────────────────────
    /// All branches (local + remote) in the repository.
    pub branches: Vec<BranchInfo>,
    /// Name of the currently checked-out branch.
    pub current_branch: Option<String>,

    // ── Commits ───────────────────────────────────────────────────────────
    /// Commit log (newest first).
    pub commits: Vec<CommitInfo>,
    /// Index into `commits` of the currently selected commit.
    pub selected_commit: Option<usize>,

    // ── Graph ─────────────────────────────────────────────────────────────
    /// Per-commit graph layout rows for branch visualisation.
    pub graph_rows: Vec<gitkraft_core::GraphRow>,

    // ── Diff / Staging ────────────────────────────────────────────────────
    /// Unstaged (working-directory) changes.
    pub unstaged_changes: Vec<DiffInfo>,
    /// Staged (index) changes.
    pub staged_changes: Vec<DiffInfo>,
    /// The diff currently displayed in the diff viewer panel.
    pub selected_diff: Option<DiffInfo>,
    /// Text in the commit-message input.
    pub commit_message: String,

    // ── Stash ─────────────────────────────────────────────────────────────
    /// All stash entries.
    pub stashes: Vec<StashEntry>,

    // ── Remotes ───────────────────────────────────────────────────────────
    /// Configured remotes.
    pub remotes: Vec<RemoteInfo>,

    // ── UI state ──────────────────────────────────────────────────────────
    /// Whether the commit detail pane is visible.
    pub show_commit_detail: bool,
    /// Whether the left sidebar is expanded.
    pub sidebar_expanded: bool,

    // ── Feedback ──────────────────────────────────────────────────────────
    /// Transient status-bar message (e.g. "Branch created").
    pub status_message: Option<String>,
    /// Error message shown in a banner / toast.
    pub error_message: Option<String>,
    /// True while an async operation is in flight.
    pub is_loading: bool,

    // ── Theme ─────────────────────────────────────────────────────────────
    /// Index into `gitkraft_core::THEME_NAMES` for the currently active theme.
    pub current_theme_index: usize,

    // ── Branch creation ───────────────────────────────────────────────────
    /// Text in the "new branch name" input.
    pub new_branch_name: String,
    /// Whether the inline branch-creation UI is visible.
    pub show_branch_create: bool,

    // ── Stash message ─────────────────────────────────────────────────────
    /// Text in the "stash message" input.
    pub stash_message: String,

    // ── Persistence ───────────────────────────────────────────────────────
    /// Recently opened repositories (loaded from settings on startup).
    pub recent_repos: Vec<gitkraft_core::RepoHistoryEntry>,
}

impl GitKraft {
    /// Create a fresh application state with sensible defaults.
    ///
    /// Loads persisted settings (theme, recent repos) from disk when available.
    pub fn new() -> Self {
        // Attempt to load persisted settings; fall back to defaults on any error.
        let settings =
            gitkraft_core::features::persistence::ops::load_settings().unwrap_or_default();

        let current_theme_index = settings
            .theme_name
            .as_deref()
            .map(gitkraft_core::theme_index_by_name)
            .unwrap_or(0);

        let recent_repos = settings.recent_repos;

        Self {
            repo_path: None,
            repo_info: None,

            branches: Vec::new(),
            current_branch: None,

            commits: Vec::new(),
            selected_commit: None,
            graph_rows: Vec::new(),

            unstaged_changes: Vec::new(),
            staged_changes: Vec::new(),
            selected_diff: None,
            commit_message: String::new(),

            stashes: Vec::new(),
            remotes: Vec::new(),

            show_commit_detail: false,
            sidebar_expanded: true,

            status_message: None,
            error_message: None,
            is_loading: false,

            current_theme_index,

            new_branch_name: String::new(),
            show_branch_create: false,

            stash_message: String::new(),

            recent_repos,
        }
    }

    /// Whether a repository is currently open.
    pub fn has_repo(&self) -> bool {
        self.repo_path.is_some()
    }

    /// Helper: the display name for the repo (last component of the path).
    pub fn repo_display_name(&self) -> &str {
        self.repo_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("GitKraft")
    }

    /// Derive the full [`ThemeColors`] from the currently active core theme.
    ///
    /// Call this at the top of view functions:
    /// ```ignore
    /// let c = state.colors();
    /// ```
    pub fn colors(&self) -> ThemeColors {
        ThemeColors::from_core(&gitkraft_core::theme_by_index(self.current_theme_index))
    }

    /// Return the `iced::Theme` that should be used as the base Iced palette.
    ///
    /// We pick `Theme::Dark` or `Theme::Light` based on the core theme's
    /// `is_dark` flag. The actual colours come from [`Self::colors()`], but
    /// Iced's built-in widgets still need a base theme for defaults.
    pub fn iced_theme(&self) -> iced::Theme {
        let core = gitkraft_core::theme_by_index(self.current_theme_index);
        if core.is_dark {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    /// The display name of the currently active theme.
    pub fn current_theme_name(&self) -> &'static str {
        gitkraft_core::THEME_NAMES
            .get(self.current_theme_index)
            .copied()
            .unwrap_or("Default")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let state = GitKraft::new();
        assert!(state.repo_path.is_none());
        assert!(!state.has_repo());
        assert_eq!(state.repo_display_name(), "GitKraft");
        assert!(state.commits.is_empty());
        assert!(state.sidebar_expanded);
        // Default theme index should be valid
        assert!(state.current_theme_index < gitkraft_core::THEME_COUNT);
    }

    #[test]
    fn repo_display_name_extracts_basename() {
        let mut state = GitKraft::new();
        state.repo_path = Some(std::path::PathBuf::from("/home/user/my-project"));
        assert_eq!(state.repo_display_name(), "my-project");
    }

    #[test]
    fn colors_returns_theme_colors() {
        let state = GitKraft::new();
        let c = state.colors();
        // The default theme (index 0) is dark, so background should be dark
        assert!(c.bg.r < 0.5);
    }

    #[test]
    fn iced_theme_matches_is_dark() {
        let mut state = GitKraft::new();
        // Index 0 = Default (dark)
        state.current_theme_index = 0;
        let iced_t = state.iced_theme();
        assert_eq!(iced_t, iced::Theme::Dark);

        // Index 11 = Solarized Light (light)
        state.current_theme_index = 11;
        let iced_t = state.iced_theme();
        assert_eq!(iced_t, iced::Theme::Light);
    }

    #[test]
    fn current_theme_name_round_trips() {
        let mut state = GitKraft::new();
        state.current_theme_index = 8;
        assert_eq!(state.current_theme_name(), "Dracula");
        state.current_theme_index = 0;
        assert_eq!(state.current_theme_name(), "Default");
    }
}
