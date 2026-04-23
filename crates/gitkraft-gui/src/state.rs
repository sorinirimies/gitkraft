use std::path::PathBuf;

use gitkraft_core::*;
use iced::{Color, Point, Task};

use crate::message::Message;
use crate::theme::ThemeColors;

// ── Pane resize ───────────────────────────────────────────────────────────────

/// Which vertical divider the user is currently dragging (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    /// The divider between the sidebar and the commit-log panel.
    SidebarRight,
    /// The divider between the commit-log panel and the diff panel.
    CommitLogRight,
    /// The divider between the diff-viewer file list and the diff content
    /// (only visible when a multi-file commit is selected).
    DiffFileListRight,
}

/// Which horizontal divider the user is currently dragging (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTargetH {
    /// The divider between the middle row and the staging area.
    StagingTop,
}

/// What item was right-clicked to open the context menu.
#[derive(Debug, Clone)]
pub enum ContextMenu {
    /// A local branch.
    Branch {
        name: String,
        is_current: bool,
        /// Index in the filtered local-branch list, used to approximate
        /// the menu's on-screen position.
        local_index: usize,
    },
    /// A remote-tracking branch (e.g. origin/feature-x).
    RemoteBranch { name: String },
    /// A commit in the log.
    Commit { index: usize, oid: String },
}

// ── Per-repository tab state ──────────────────────────────────────────────────

/// Per-repository state — one instance per open tab.
pub struct RepoTab {
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
    /// Per-commit graph layout rows for branch visualisation.
    pub graph_rows: Vec<gitkraft_core::GraphRow>,

    // ── Diff / Staging ────────────────────────────────────────────────────
    /// Unstaged (working-directory) changes.
    pub unstaged_changes: Vec<DiffInfo>,
    /// Staged (index) changes.
    pub staged_changes: Vec<DiffInfo>,
    /// Lightweight file list for the currently selected commit (path + status only).
    pub commit_files: Vec<gitkraft_core::DiffFileEntry>,
    /// OID of the currently selected commit (needed for on-demand file diff loading).
    pub selected_commit_oid: Option<String>,
    /// Index of the selected file in `commit_files`.
    pub selected_file_index: Option<usize>,
    /// True while a single-file diff is being loaded.
    pub is_loading_file_diff: bool,
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

    // ── Per-tab UI state ──────────────────────────────────────────────────
    /// Whether the commit detail pane is visible.
    pub show_commit_detail: bool,
    /// Text in the "new branch name" input.
    pub new_branch_name: String,
    /// Whether the inline branch-creation UI is visible.
    pub show_branch_create: bool,
    /// Whether the Local branches section is expanded.
    pub local_branches_expanded: bool,
    /// Whether the Remote branches section is expanded.
    pub remote_branches_expanded: bool,
    /// Text in the "stash message" input.
    pub stash_message: String,

    /// File path pending discard confirmation (None = no pending discard).
    pub pending_discard: Option<String>,

    // ── Feedback ──────────────────────────────────────────────────────────
    /// Transient status-bar message (e.g. "Branch created").
    pub status_message: Option<String>,
    /// Error message shown in a banner / toast.
    pub error_message: Option<String>,
    /// True while an async operation is in flight.
    pub is_loading: bool,
    /// Cursor position captured at the moment the context menu was opened.
    /// Used to anchor the menu so it doesn't follow the mouse after appearing.
    pub context_menu_pos: (f32, f32),

    /// Currently open context menu, if any.
    pub context_menu: Option<ContextMenu>,
    /// Name of the branch currently being renamed (None = not renaming).
    pub rename_branch_target: Option<String>,
    /// The new name being typed in the rename input.
    pub rename_branch_input: String,

    /// When `Some(oid)`, the tag-creation inline form is visible, targeting that OID.
    pub create_tag_target_oid: Option<String>,
    /// True when creating an annotated tag; false for a lightweight tag.
    pub create_tag_annotated: bool,
    /// The tag name the user is typing.
    pub create_tag_name: String,
    /// The annotated tag message the user is typing (only used when `create_tag_annotated` is true).
    pub create_tag_message: String,

    /// Current scroll offset of the commit log in pixels.
    /// Tracked via `on_scroll` so virtual scrolling can render only the
    /// visible window of rows.
    pub commit_scroll_offset: f32,

    /// Current scroll offset of the diff viewer in pixels.
    pub diff_scroll_offset: f32,
    /// Pre-computed display strings for each commit:
    /// `(truncated_summary, relative_time, truncated_author)`.
    /// Computed once when commits load to avoid per-frame string allocations.
    pub commit_display: Vec<(String, String, String)>,

    /// Whether there are potentially more commits to load beyond those already shown.
    pub has_more_commits: bool,
    /// Guard: true while a background load-more task is in flight (prevents duplicates).
    pub is_loading_more_commits: bool,
}

impl RepoTab {
    /// Create an empty tab (no repo open — shows welcome screen).
    pub fn new_empty() -> Self {
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
            commit_files: Vec::new(),
            selected_commit_oid: None,
            selected_file_index: None,
            is_loading_file_diff: false,
            selected_diff: None,
            commit_message: String::new(),
            stashes: Vec::new(),
            remotes: Vec::new(),
            show_commit_detail: false,
            new_branch_name: String::new(),
            show_branch_create: false,
            local_branches_expanded: true,
            remote_branches_expanded: true,
            stash_message: String::new(),
            pending_discard: None,
            status_message: None,
            error_message: None,
            is_loading: false,
            context_menu: None,
            context_menu_pos: (0.0, 0.0),
            rename_branch_target: None,
            rename_branch_input: String::new(),
            create_tag_target_oid: None,
            create_tag_annotated: false,
            create_tag_name: String::new(),
            create_tag_message: String::new(),
            commit_scroll_offset: 0.0,
            diff_scroll_offset: 0.0,
            commit_display: Vec::new(),
            has_more_commits: true,
            is_loading_more_commits: false,
        }
    }

    /// Whether a repository is currently open in this tab.
    pub fn has_repo(&self) -> bool {
        self.repo_path.is_some()
    }

    /// Display name for the tab (last path component, or "New Tab").
    pub fn display_name(&self) -> &str {
        self.repo_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("New Tab")
    }

    /// Apply a full repo payload to this tab, resetting transient UI state.
    pub fn apply_payload(
        &mut self,
        payload: crate::message::RepoPayload,
        path: std::path::PathBuf,
    ) {
        self.current_branch = payload.info.head_branch.clone();
        self.repo_path = Some(path);
        self.repo_info = Some(payload.info);
        self.branches = payload.branches;
        self.commits = payload.commits;
        self.graph_rows = payload.graph_rows;
        self.unstaged_changes = payload.unstaged;
        self.staged_changes = payload.staged;
        self.stashes = payload.stashes;
        self.remotes = payload.remotes;

        // Reset transient UI state.
        self.selected_commit = None;
        self.selected_diff = None;
        self.commit_files.clear();
        self.selected_commit_oid = None;
        self.selected_file_index = None;
        self.is_loading_file_diff = false;
        self.commit_message.clear();
        self.error_message = None;
        self.status_message = Some("Repository loaded.".into());
        self.commit_scroll_offset = 0.0;
        self.diff_scroll_offset = 0.0;
        self.has_more_commits = true;
        self.is_loading_more_commits = false;
    }
}

// ── Top-level application state ───────────────────────────────────────────────

/// Top-level application state for the GitKraft GUI.
pub struct GitKraft {
    // ── Tabs ──────────────────────────────────────────────────────────────
    /// All open repository tabs.
    pub tabs: Vec<RepoTab>,
    /// Index of the currently active/visible tab.
    pub active_tab: usize,

    // ── UI state (global, not per-tab) ────────────────────────────────────
    /// Whether the left sidebar is expanded.
    pub sidebar_expanded: bool,

    // ── Pane widths / heights (pixels) ────────────────────────────────────
    /// Width of the left sidebar in pixels.
    pub sidebar_width: f32,
    /// Width of the commit-log panel in pixels.
    pub commit_log_width: f32,
    /// Height of the staging area in pixels.
    pub staging_height: f32,
    /// Width of the diff file-list sidebar in pixels.
    pub diff_file_list_width: f32,

    /// UI scale factor (1.0 = default). Adjusted with Ctrl+/Ctrl- keyboard shortcuts.
    pub ui_scale: f32,

    // ── Drag state ────────────────────────────────────────────────────────
    /// Which vertical divider is being dragged (if any).
    pub dragging: Option<DragTarget>,
    /// Which horizontal divider is being dragged (if any).
    pub dragging_h: Option<DragTargetH>,
    /// Last known mouse X position during a drag (absolute window coords).
    pub drag_start_x: f32,
    /// Last known mouse Y position during a drag (absolute window coords).
    pub drag_start_y: f32,
    /// Whether the first move event has been received for the current vertical drag.
    /// `false` right after `PaneDragStart` — the first `PaneDragMove` sets the
    /// real start position instead of computing a bogus delta from 0.0.
    pub drag_initialized: bool,
    /// Same as `drag_initialized` but for horizontal drags.
    pub drag_initialized_h: bool,

    // ── Cursor ────────────────────────────────────────────────────────────
    /// Last known cursor position in window coordinates.
    /// Updated on every mouse-move event so context menus open at the
    /// exact spot the user right-clicked.
    pub cursor_pos: Point,

    // ── Theme ─────────────────────────────────────────────────────────────
    /// Index into `gitkraft_core::THEME_NAMES` for the currently active theme.
    pub current_theme_index: usize,

    // ── Persistence ───────────────────────────────────────────────────────
    /// Recently opened repositories (loaded from settings on startup).
    pub recent_repos: Vec<gitkraft_core::RepoHistoryEntry>,

    // ── Search ────────────────────────────────────────────────────────────
    /// Whether the search overlay is visible.
    pub search_visible: bool,
    /// Current search query text.
    pub search_query: String,
    /// Search results (commit infos matching the query).
    pub search_results: Vec<gitkraft_core::CommitInfo>,
    /// Index of the selected search result.
    pub search_selected: Option<usize>,
}

impl Default for GitKraft {
    fn default() -> Self {
        Self::new()
    }
}

impl GitKraft {
    /// Build application state from persisted [`AppSettings`].
    ///
    /// Starts with a single empty tab regardless of what was saved — callers
    /// that want to restore the full session should use
    /// [`Self::new_with_session_paths`] instead.
    fn from_settings(settings: gitkraft_core::AppSettings) -> Self {
        let current_theme_index = settings
            .theme_name
            .as_deref()
            .map(gitkraft_core::theme_index_by_name)
            .unwrap_or(0);

        let recent_repos = settings.recent_repos;

        let (
            sidebar_width,
            commit_log_width,
            staging_height,
            diff_file_list_width,
            sidebar_expanded,
            ui_scale,
        ) = if let Some(ref layout) = settings.layout {
            (
                layout.sidebar_width.unwrap_or(220.0),
                layout.commit_log_width.unwrap_or(500.0),
                layout.staging_height.unwrap_or(200.0),
                layout.diff_file_list_width.unwrap_or(180.0),
                layout.sidebar_expanded.unwrap_or(true),
                layout.ui_scale.unwrap_or(1.0),
            )
        } else {
            (220.0, 500.0, 200.0, 180.0, true, 1.0)
        };

        Self {
            tabs: vec![RepoTab::new_empty()],
            active_tab: 0,

            sidebar_expanded,

            sidebar_width,
            commit_log_width,
            staging_height,
            diff_file_list_width,

            ui_scale,

            dragging: None,
            dragging_h: None,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_initialized: false,
            drag_initialized_h: false,
            cursor_pos: Point::ORIGIN,

            current_theme_index,

            recent_repos,

            search_visible: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: None,
        }
    }

    /// Create a fresh application state with sensible defaults.
    ///
    /// Loads persisted settings (theme, recent repos) from disk when available.
    /// Always starts with one empty tab — use [`Self::new_with_session_paths`] to
    /// restore the full multi-tab session.
    pub fn new() -> Self {
        Self::from_settings(
            gitkraft_core::features::persistence::ops::load_settings().unwrap_or_default(),
        )
    }

    /// Create state and also return the saved tab paths for startup restore.
    ///
    /// Call this from `main.rs` instead of [`Self::new`]; it sets up loading tabs
    /// for every path in the persisted session and returns those paths so the
    /// caller can spawn parallel `load_repo_at` tasks.
    pub fn new_with_session_paths() -> (Self, Vec<PathBuf>) {
        let settings =
            gitkraft_core::features::persistence::ops::load_settings().unwrap_or_default();
        let open_tabs = settings.open_tabs.clone();
        let active_tab_index = settings.active_tab_index;

        let mut state = Self::from_settings(settings);

        if !open_tabs.is_empty() {
            state.tabs = open_tabs
                .iter()
                .map(|path| {
                    let mut tab = RepoTab::new_empty();
                    // Set the path now so the tab bar shows the right name
                    // while the repo is being loaded in the background.
                    tab.repo_path = Some(path.clone());
                    if path.exists() {
                        tab.is_loading = true;
                        tab.status_message = Some(format!(
                            "Loading {}…",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    } else {
                        tab.error_message =
                            Some(format!("Repository not found: {}", path.display()));
                    }
                    tab
                })
                .collect();
            state.active_tab = active_tab_index.min(state.tabs.len().saturating_sub(1));
        }

        (state, open_tabs)
    }

    /// Paths of all tabs where a repository has been fully loaded
    /// (`repo_info` is populated). Used to persist the multi-tab session.
    pub fn open_tab_paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter(|t| t.repo_info.is_some())
            .filter_map(|t| t.repo_path.clone())
            .collect()
    }

    /// Get a reference to the currently active tab.
    pub fn active_tab(&self) -> &RepoTab {
        &self.tabs[self.active_tab]
    }

    /// Get a mutable reference to the currently active tab.
    pub fn active_tab_mut(&mut self) -> &mut RepoTab {
        &mut self.tabs[self.active_tab]
    }

    /// Whether the active tab has a repository open.
    pub fn has_repo(&self) -> bool {
        self.active_tab().has_repo()
    }

    /// Helper: the display name for the active tab's repo.
    pub fn repo_display_name(&self) -> &str {
        self.active_tab().display_name()
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

    /// Return a **custom** `iced::Theme` whose `Palette` is derived from the
    /// active core theme.
    ///
    /// This is the key to making every built-in Iced widget (text inputs,
    /// pick-lists, scrollbars, buttons without explicit `.style()`, etc.)
    /// inherit the correct background, text, accent, success and danger
    /// colours.  Without this, Iced falls back to its generic Dark/Light
    /// palette and the UI looks wrong for every non-default theme.
    pub fn iced_theme(&self) -> iced::Theme {
        let core = gitkraft_core::theme_by_index(self.current_theme_index);
        let name = self.current_theme_name().to_string();

        let palette = iced::theme::Palette {
            background: rgb_to_iced(core.background),
            text: rgb_to_iced(core.text_primary),
            primary: rgb_to_iced(core.accent),
            success: rgb_to_iced(core.success),
            warning: rgb_to_iced(core.warning),
            danger: rgb_to_iced(core.error),
        };

        iced::Theme::custom(name, palette)
    }

    /// The display name of the currently active theme.
    pub fn current_theme_name(&self) -> &'static str {
        gitkraft_core::THEME_NAMES
            .get(self.current_theme_index)
            .copied()
            .unwrap_or("Default")
    }

    /// Refresh all data for the currently active tab's repository.
    ///
    /// Returns [`Task::none()`] if no repository is open in the active tab.
    pub fn refresh_active_tab(&mut self) -> Task<Message> {
        match self.active_tab().repo_path.clone() {
            Some(path) => crate::features::repo::commands::refresh_repo(path),
            None => Task::none(),
        }
    }

    /// Handle a `Result<(), String>` from a git operation that should trigger
    /// a full repository refresh on success.
    ///
    /// * `Ok(())` — clears `is_loading`, sets `status_message`, refreshes.
    /// * `Err(e)` — clears `is_loading`, sets `error_message`, returns
    ///   [`Task::none()`].
    pub fn on_ok_refresh(
        &mut self,
        result: Result<(), String>,
        ok_msg: &str,
        err_prefix: &str,
    ) -> Task<Message> {
        match result {
            Ok(()) => {
                {
                    let tab = self.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = Some(ok_msg.to_string());
                }
                self.refresh_active_tab()
            }
            Err(e) => {
                let tab = self.active_tab_mut();
                tab.is_loading = false;
                tab.error_message = Some(format!("{err_prefix}: {e}"));
                tab.status_message = None;
                Task::none()
            }
        }
    }

    /// Build a [`LayoutSettings`] snapshot from the current pane dimensions.
    pub fn current_layout(&self) -> gitkraft_core::LayoutSettings {
        gitkraft_core::LayoutSettings {
            sidebar_width: Some(self.sidebar_width),
            commit_log_width: Some(self.commit_log_width),
            staging_height: Some(self.staging_height),
            diff_file_list_width: Some(self.diff_file_list_width),
            sidebar_expanded: Some(self.sidebar_expanded),
            ui_scale: Some(self.ui_scale),
        }
    }
}

/// Convert a core [`gitkraft_core::Rgb`] to an [`iced::Color`].
fn rgb_to_iced(rgb: gitkraft_core::Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults() {
        let state = GitKraft::new();
        assert!(state.active_tab().repo_path.is_none());
        assert!(!state.has_repo());
        assert_eq!(state.repo_display_name(), "New Tab");
        assert!(state.active_tab().commits.is_empty());
        assert!(state.sidebar_expanded);
        // Default theme index should be valid
        assert!(state.current_theme_index < gitkraft_core::THEME_COUNT);
        // Pane defaults
        assert!(state.sidebar_width > 0.0);
        assert!(state.commit_log_width > 0.0);
        assert!(state.staging_height > 0.0);
        assert!(state.dragging.is_none());
        assert!(state.dragging_h.is_none());
        // Should start with one empty tab
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn repo_display_name_extracts_basename() {
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path = Some(std::path::PathBuf::from("/home/user/my-project"));
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
    fn iced_theme_is_custom_with_correct_palette() {
        let mut state = GitKraft::new();

        // Index 0 = Default (dark) — custom theme with dark background
        state.current_theme_index = 0;
        let iced_t = state.iced_theme();
        let pal = iced_t.palette();
        assert!(pal.background.r < 0.5, "Default theme bg should be dark");
        assert_eq!(iced_t.to_string(), "Default");

        // Index 11 = Solarized Light — custom theme with light background
        state.current_theme_index = 11;
        let iced_t = state.iced_theme();
        let pal = iced_t.palette();
        assert!(pal.background.r > 0.5, "Solarized Light bg should be light");
        assert_eq!(iced_t.to_string(), "Solarized Light");

        // Index 12 = Gruvbox Dark — accent should come from core
        state.current_theme_index = 12;
        let iced_t = state.iced_theme();
        let pal = iced_t.palette();
        let core = gitkraft_core::theme_by_index(12);
        let expected_accent = rgb_to_iced(core.accent);
        assert!(
            (pal.primary.r - expected_accent.r).abs() < 0.01
                && (pal.primary.g - expected_accent.g).abs() < 0.01
                && (pal.primary.b - expected_accent.b).abs() < 0.01,
            "Gruvbox Dark accent should match core accent"
        );
    }

    #[test]
    fn iced_theme_name_round_trips_through_core() {
        // Ensure the custom theme name matches a core THEME_NAMES entry so
        // that ThemeColors::from_theme() can map it back to the right index.
        for i in 0..gitkraft_core::THEME_COUNT {
            let mut state = GitKraft::new();
            state.current_theme_index = i;
            let iced_t = state.iced_theme();
            let name = iced_t.to_string();
            let resolved = gitkraft_core::theme_index_by_name(&name);
            assert_eq!(
                resolved,
                i,
                "theme index {i} ({}) did not round-trip through iced_theme name",
                gitkraft_core::THEME_NAMES[i]
            );
        }
    }

    #[test]
    fn current_theme_name_round_trips() {
        let mut state = GitKraft::new();
        state.current_theme_index = 8;
        assert_eq!(state.current_theme_name(), "Dracula");
        state.current_theme_index = 0;
        assert_eq!(state.current_theme_name(), "Default");
    }

    #[test]
    fn repo_tab_new_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.repo_path.is_none());
        assert!(!tab.has_repo());
        assert_eq!(tab.display_name(), "New Tab");
        assert!(tab.commits.is_empty());
        assert!(tab.branches.is_empty());
        assert!(!tab.is_loading);
    }

    #[test]
    fn repo_tab_display_name_with_path() {
        let mut tab = RepoTab::new_empty();
        tab.repo_path = Some(std::path::PathBuf::from("/some/path/cool-repo"));
        assert!(tab.has_repo());
        assert_eq!(tab.display_name(), "cool-repo");
    }

    #[test]
    fn search_defaults() {
        let state = GitKraft::new();
        assert!(!state.search_visible);
        assert!(state.search_query.is_empty());
        assert!(state.search_results.is_empty());
        assert!(state.search_selected.is_none());
    }
}
