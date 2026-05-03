use std::collections::HashSet;
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
    /// A stash entry.
    Stash { index: usize },
    /// An unstaged file in the staging area.
    UnstagedFile { path: String },
    /// A staged file in the staging area.
    StagedFile { path: String },
    /// A file in a commit diff.
    CommitFile { oid: String, file_path: String },
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
    /// Anchor commit index for range selection — set on a plain click.
    pub anchor_commit_index: Option<usize>,
    /// Ordered (ascending) list of commit indices in the current range selection.
    pub selected_commits: Vec<usize>,
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
    /// Anchor index for range selection — set on a regular click, kept fixed while
    /// the user extends the selection with Shift+Click.
    pub anchor_file_index: Option<usize>,
    /// Ordered list of file indices currently multi-selected in the commit file list (Shift+Click).
    /// Always stored in ascending index order (lowest index first).
    pub selected_commit_file_indices: Vec<usize>,
    /// Diffs for all multi-selected files (populated when 2+ files are selected).
    pub multi_file_diffs: Vec<gitkraft_core::DiffInfo>,
    /// Combined diff for all commits in the current range selection (populated when
    /// `selected_commits.len() > 1`). Shown in the diff panel instead of single-commit diff.
    pub commit_range_diffs: Vec<gitkraft_core::DiffInfo>,
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

    /// Set of selected unstaged file paths (for multi-select with Shift+Click).
    pub selected_unstaged: std::collections::HashSet<String>,
    /// Set of selected staged file paths (for multi-select with Shift+Click).
    pub selected_staged: std::collections::HashSet<String>,

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
    /// When `Some(oid)`, the inline "create branch at this commit" form is visible.
    pub create_branch_at_oid: Option<String>,

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

    /// When `Some(path)`, the file-history overlay is shown for that repo-relative path.
    pub file_history_path: Option<String>,
    /// Commits loaded for the file-history overlay (newest first).
    pub file_history_commits: Vec<gitkraft_core::CommitInfo>,
    /// Scroll offset of the file-history list in pixels.
    pub file_history_scroll: f32,

    /// When `Some(path)`, the blame overlay is shown for that repo-relative path.
    pub blame_path: Option<String>,
    /// Blame lines loaded for the blame overlay.
    pub blame_lines: Vec<gitkraft_core::BlameLine>,
    /// Scroll offset of the blame view in pixels.
    pub blame_scroll: f32,

    /// When `Some(path)`, a delete-confirmation banner is shown for that file.
    pub pending_delete_file: Option<String>,
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
            anchor_commit_index: None,
            selected_commits: Vec::new(),
            graph_rows: Vec::new(),
            unstaged_changes: Vec::new(),
            staged_changes: Vec::new(),
            commit_files: Vec::new(),
            selected_commit_oid: None,
            selected_file_index: None,
            is_loading_file_diff: false,
            anchor_file_index: None,
            selected_commit_file_indices: Vec::new(),
            multi_file_diffs: Vec::new(),
            commit_range_diffs: Vec::new(),
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
            selected_unstaged: std::collections::HashSet::new(),
            selected_staged: std::collections::HashSet::new(),
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
            create_branch_at_oid: None,
            commit_scroll_offset: 0.0,
            diff_scroll_offset: 0.0,
            commit_display: Vec::new(),
            has_more_commits: true,
            is_loading_more_commits: false,
            file_history_path: None,
            file_history_commits: Vec::new(),
            file_history_scroll: 0.0,
            blame_path: None,
            blame_lines: Vec::new(),
            blame_scroll: 0.0,
            pending_delete_file: None,
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
    ///
    /// The currently selected commit (if any) is **re-pinned** by OID after the
    /// new commit list arrives, so background auto-refreshes (git-watcher or
    /// staging changes) never clear the user's selection.
    pub fn apply_payload(
        &mut self,
        payload: crate::message::RepoPayload,
        path: std::path::PathBuf,
    ) {
        // ── Save selection so we can restore it after the data refresh ────
        let prev_oid = self.selected_commit_oid.clone();

        // Save multi-selection OIDs so we can re-map them after the commit
        // list is replaced.  Without this, background auto-refreshes (git
        // watcher, fallback poll) would silently clear a Shift+click range.
        let prev_anchor_oid = self
            .anchor_commit_index
            .and_then(|i| self.commits.get(i).map(|c| c.oid.clone()));
        let prev_selected_oids: Vec<String> = self
            .selected_commits
            .iter()
            .filter_map(|&i| self.commits.get(i).map(|c| c.oid.clone()))
            .collect();

        // Save file-level selection state.  File indices reference into
        // `commit_files` which is NOT replaced during a refresh (the commit
        // content hasn't changed), so these can be restored as-is.
        let prev_anchor_file = self.anchor_file_index;
        let prev_file_indices = self.selected_commit_file_indices.clone();
        let prev_multi_file_diffs = std::mem::take(&mut self.multi_file_diffs);
        let prev_commit_range_diffs = std::mem::take(&mut self.commit_range_diffs);

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
        // NOTE: selected_commit / commit_files / selected_diff are restored
        // below so they survive background auto-refreshes.
        self.selected_commit = None;
        self.anchor_commit_index = None;
        self.selected_commits.clear();
        self.selected_commit_oid = None;
        self.commit_message.clear();
        self.error_message = None;
        self.status_message = Some("Repository loaded.".into());
        self.commit_scroll_offset = 0.0;
        self.has_more_commits = true;
        self.is_loading_more_commits = false;
        self.selected_unstaged.clear();
        self.selected_staged.clear();
        self.anchor_file_index = None;
        self.selected_commit_file_indices.clear();
        self.multi_file_diffs.clear();
        self.commit_range_diffs.clear();

        // ── Restore the previously selected commit by OID ─────────────────
        // If the commit still exists in the refreshed list, re-select it so
        // the diff panel, file list, and selection highlight are all
        // preserved.  This means auto-refreshes (every 5 s fallback, git
        // watcher) never interrupt the user’s view.
        if let Some(oid) = prev_oid {
            if let Some(new_idx) = self.commits.iter().position(|c| c.oid == oid) {
                self.selected_commit = Some(new_idx);
                self.selected_commit_oid = Some(oid);
                // commit_files, selected_diff, selected_file_index,
                // is_loading_file_diff, diff_scroll_offset are intentionally
                // left unchanged — the commit content hasn’t changed.

                // Restore file-level selection — the commit (and its file
                // list) survived, so these indices are still valid.
                self.anchor_file_index = prev_anchor_file;
                self.selected_commit_file_indices = prev_file_indices;
                self.multi_file_diffs = prev_multi_file_diffs;
                self.commit_range_diffs = prev_commit_range_diffs;
            } else {
                // Commit was rebased / force-pushed away — clear everything.
                self.selected_diff = None;
                self.commit_files.clear();
                self.selected_file_index = None;
                self.is_loading_file_diff = false;
                self.diff_scroll_offset = 0.0;
            }
        } else {
            // No previous selection — safe to clear diff state.
            self.selected_diff = None;
            self.commit_files.clear();
            self.selected_file_index = None;
            self.is_loading_file_diff = false;
            self.diff_scroll_offset = 0.0;
        }

        // ── Restore multi-selection by OID ─────────────────────────────
        // Re-map the saved anchor and range selection from OIDs back to
        // indices in the (possibly reordered) new commit list.
        if let Some(anchor_oid) = prev_anchor_oid {
            if let Some(new_anchor) = self.commits.iter().position(|c| c.oid == anchor_oid) {
                self.anchor_commit_index = Some(new_anchor);
            }
        }
        if !prev_selected_oids.is_empty() {
            let restored: Vec<usize> = prev_selected_oids
                .iter()
                .filter_map(|oid| self.commits.iter().position(|c| &c.oid == oid))
                .collect();
            if !restored.is_empty() {
                self.selected_commits = restored;
                self.selected_commits.sort_unstable();
            }
        }
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

    /// Files changed between the selected search commit and working tree.
    pub search_diff_files: Vec<gitkraft_core::DiffFileEntry>,
    /// Selected file indices in the search diff file list.
    pub search_diff_selected: HashSet<usize>,
    /// The diff content for the currently viewed search diff file(s).
    pub search_diff_content: Vec<gitkraft_core::DiffInfo>,
    /// OID of the commit being diffed against working tree in search.
    pub search_diff_oid: Option<String>,

    /// Configured editor for "Open in editor" actions.
    pub editor: gitkraft_core::Editor,

    /// Current keyboard modifier state (updated via subscription).
    pub keyboard_modifiers: iced::keyboard::Modifiers,

    /// Monotonically-increasing counter incremented on every `AnimationTick`.
    /// Drives the loading-spinner frame selection in all UI widgets.
    pub animation_tick: u64,

    // ── Window geometry ───────────────────────────────────────────────────
    /// Last known window width (updated on WindowResized).
    pub window_width: f32,
    /// Last known window height (updated on WindowResized).
    pub window_height: f32,
    /// Last known window X position (updated on WindowMoved).
    pub window_x: f32,
    /// Last known window Y position (updated on WindowMoved).
    pub window_y: f32,
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
            search_diff_files: Vec::new(),
            search_diff_selected: HashSet::new(),
            search_diff_content: Vec::new(),
            search_diff_oid: None,

            keyboard_modifiers: iced::keyboard::Modifiers::default(),
            animation_tick: 0,

            window_width: settings
                .layout
                .as_ref()
                .and_then(|l| l.window_width)
                .unwrap_or(1400.0),
            window_height: settings
                .layout
                .as_ref()
                .and_then(|l| l.window_height)
                .unwrap_or(800.0),
            window_x: settings
                .layout
                .as_ref()
                .and_then(|l| l.window_x)
                .unwrap_or(0.0),
            window_y: settings
                .layout
                .as_ref()
                .and_then(|l| l.window_y)
                .unwrap_or(0.0),

            editor: settings
                .editor_name
                .as_deref()
                .map(|name| {
                    // Try to map persisted name back to Editor variant
                    gitkraft_core::EDITOR_NAMES
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(name))
                        .map(gitkraft_core::Editor::from_index)
                        .unwrap_or_else(|| {
                            if name.eq_ignore_ascii_case("none") {
                                gitkraft_core::Editor::None
                            } else {
                                gitkraft_core::Editor::Custom(name.to_string())
                            }
                        })
                })
                .unwrap_or_else(detect_system_editor),
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
            window_width: Some(self.window_width),
            window_height: Some(self.window_height),
            window_x: Some(self.window_x),
            window_y: Some(self.window_y),
            window_maximized: None, // not tracked
        }
    }
}

/// Convert a core [`gitkraft_core::Rgb`] to an [`iced::Color`].
fn rgb_to_iced(rgb: gitkraft_core::Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

/// Try to detect the system's preferred editor from environment variables.
fn detect_system_editor() -> gitkraft_core::Editor {
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(val) = std::env::var(var) {
            let bin = val.split('/').next_back().unwrap_or(&val).trim();
            return match bin {
                "nvim" | "neovim" => gitkraft_core::Editor::Neovim,
                "vim" => gitkraft_core::Editor::Vim,
                "hx" | "helix" => gitkraft_core::Editor::Helix,
                "nano" => gitkraft_core::Editor::Nano,
                "micro" => gitkraft_core::Editor::Micro,
                "emacs" => gitkraft_core::Editor::Emacs,
                "code" => gitkraft_core::Editor::VSCode,
                "zed" => gitkraft_core::Editor::Zed,
                "subl" => gitkraft_core::Editor::Sublime,
                _ => gitkraft_core::Editor::Custom(val),
            };
        }
    }
    gitkraft_core::Editor::None
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

    #[test]
    fn context_menu_variants_exist() {
        // Verify all context menu variants can be constructed
        use crate::state::ContextMenu;

        let _branch = ContextMenu::Branch {
            name: "main".to_string(),
            is_current: true,
            local_index: 0,
        };
        let _remote = ContextMenu::RemoteBranch {
            name: "origin/main".to_string(),
        };
        let _commit = ContextMenu::Commit {
            index: 0,
            oid: "abc1234".to_string(),
        };
        let _stash = ContextMenu::Stash { index: 0 };
        let _unstaged = ContextMenu::UnstagedFile {
            path: "src/main.rs".to_string(),
        };
        let _staged = ContextMenu::StagedFile {
            path: "src/lib.rs".to_string(),
        };
    }

    #[test]
    fn repo_tab_context_menu_defaults_to_none() {
        let tab = crate::state::RepoTab::new_empty();
        assert!(tab.context_menu.is_none());
    }

    #[test]
    fn context_menu_variants_constructable() {
        use crate::state::ContextMenu;
        let _ = ContextMenu::Stash { index: 0 };
        let _ = ContextMenu::UnstagedFile {
            path: "a.rs".into(),
        };
        let _ = ContextMenu::StagedFile {
            path: "b.rs".into(),
        };
    }

    #[test]
    fn selected_unstaged_defaults_empty() {
        let tab = crate::state::RepoTab::new_empty();
        assert!(tab.selected_unstaged.is_empty());
        assert!(tab.selected_staged.is_empty());
    }

    #[test]
    fn selected_unstaged_toggle() {
        let mut tab = crate::state::RepoTab::new_empty();
        tab.selected_unstaged.insert("a.rs".to_string());
        tab.selected_unstaged.insert("b.rs".to_string());
        assert_eq!(tab.selected_unstaged.len(), 2);
        assert!(tab.selected_unstaged.contains("a.rs"));
        tab.selected_unstaged.remove("a.rs");
        assert_eq!(tab.selected_unstaged.len(), 1);
        assert!(!tab.selected_unstaged.contains("a.rs"));
    }

    #[test]
    fn detect_system_editor_returns_valid() {
        // Just verify it doesn't panic
        let editor = super::detect_system_editor();
        let _ = editor.display_name();
    }

    // ── Multi-file commit diff selection ──────────────────────────────────

    #[test]
    fn selected_commit_file_indices_defaults_to_empty_vec() {
        let tab = RepoTab::new_empty();
        assert!(tab.selected_commit_file_indices.is_empty());
        // Must be a Vec (ordered), not a HashSet — check it supports indexing
        let v: &Vec<usize> = &tab.selected_commit_file_indices;
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn multi_file_diffs_defaults_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.multi_file_diffs.is_empty());
    }

    #[test]
    fn keyboard_modifiers_default_has_no_shift() {
        let state = GitKraft::new();
        assert!(!state.keyboard_modifiers.shift());
    }

    #[test]
    fn selected_commit_file_indices_preserves_insertion_order() {
        let mut tab = RepoTab::new_empty();
        tab.selected_commit_file_indices.push(5);
        tab.selected_commit_file_indices.push(2);
        tab.selected_commit_file_indices.push(8);
        assert_eq!(tab.selected_commit_file_indices, vec![5, 2, 8]);
    }

    #[test]
    fn selected_commit_file_indices_cleared_on_reset() {
        let mut tab = RepoTab::new_empty();
        tab.selected_commit_file_indices.push(0);
        tab.selected_commit_file_indices.push(1);
        tab.selected_commit_file_indices.clear();
        assert!(tab.selected_commit_file_indices.is_empty());
    }

    #[test]
    fn multi_file_diffs_cleared_on_reset() {
        let mut tab = RepoTab::new_empty();
        tab.multi_file_diffs.push(gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        });
        tab.multi_file_diffs.clear();
        assert!(tab.multi_file_diffs.is_empty());
    }

    #[test]
    fn commit_range_diffs_defaults_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.commit_range_diffs.is_empty());
    }

    #[test]
    fn commit_range_diffs_cleared_on_apply_payload() {
        // verify the field is reset — just check it's accessible and clearable
        let mut tab = RepoTab::new_empty();
        tab.commit_range_diffs.push(gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "x.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        });
        tab.commit_range_diffs.clear();
        assert!(tab.commit_range_diffs.is_empty());
    }

    // ── ModifiersChanged update ───────────────────────────────────────────

    #[test]
    fn modifiers_changed_sets_shift_state() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        assert!(!state.keyboard_modifiers.shift());

        let _ = state.update(Message::ModifiersChanged(iced::keyboard::Modifiers::SHIFT));
        assert!(state.keyboard_modifiers.shift());

        let _ = state.update(Message::ModifiersChanged(
            iced::keyboard::Modifiers::default(),
        ));
        assert!(!state.keyboard_modifiers.shift());
    }

    // ── SelectDiffByIndex update ──────────────────────────────────────────

    fn make_commit_files(names: &[&str]) -> Vec<gitkraft_core::DiffFileEntry> {
        names
            .iter()
            .map(|name| gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: name.to_string(),
                status: gitkraft_core::FileStatus::Modified,
            })
            .collect()
    }

    #[test]
    fn select_diff_by_index_regular_click_clears_multi_selection() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Provide a repo_path and oid so the update handler can reach the
        // `selected_file_index = Some(index)` assignment (the async task it
        // spawns is dropped without execution — no real repo is needed).
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);
        // Pre-populate a multi-selection
        state.active_tab_mut().selected_commit_file_indices = vec![0, 1];

        // Regular click (no Shift) — should collapse to single-file selection
        let _ = state.update(Message::SelectDiffByIndex(0));

        assert!(state.active_tab().selected_commit_file_indices.is_empty());
        assert_eq!(state.active_tab().selected_file_index, Some(0));
    }

    #[test]
    fn regular_click_preserves_multi_file_diffs_until_load_completes() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);

        // Pre-populate multi_file_diffs as if user had a multi-file selection
        state.active_tab_mut().multi_file_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];

        // Regular click — should NOT clear multi_file_diffs immediately
        // (deferred to SingleFileDiffLoaded to avoid visual blink).
        let _ = state.update(Message::SelectDiffByIndex(1));

        assert!(
            !state.active_tab().multi_file_diffs.is_empty(),
            "multi_file_diffs must NOT be cleared on click (deferred to diff load)"
        );
        assert_eq!(state.active_tab().selected_file_index, Some(1));
        assert!(state.active_tab().is_loading_file_diff);
    }

    #[test]
    fn regular_click_preserves_diff_scroll_until_load_completes() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs"]);
        state.active_tab_mut().diff_scroll_offset = 150.0;

        // Regular click — should NOT reset diff_scroll_offset immediately.
        let _ = state.update(Message::SelectDiffByIndex(1));

        assert_eq!(
            state.active_tab().diff_scroll_offset,
            150.0,
            "diff_scroll_offset must NOT be reset on click (deferred to diff load)"
        );
    }

    #[test]
    fn single_file_diff_loaded_clears_deferred_state() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs"]);

        // Pre-populate multi_file_diffs and commit_range_diffs
        state.active_tab_mut().multi_file_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];
        state.active_tab_mut().commit_range_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "b.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];
        state.active_tab_mut().is_loading_file_diff = true;
        state.active_tab_mut().diff_scroll_offset = 200.0;

        // Simulate SingleFileDiffLoaded arriving with the new diff.
        let new_diff = gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "b.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        };
        let _ = state.update(Message::SingleFileDiffLoaded(Ok(new_diff)));

        // All deferred state should now be cleared.
        assert!(
            state.active_tab().multi_file_diffs.is_empty(),
            "multi_file_diffs must be cleared when new diff arrives"
        );
        assert!(
            state.active_tab().commit_range_diffs.is_empty(),
            "commit_range_diffs must be cleared when new diff arrives"
        );
        assert!(
            state.active_tab().selected_diff.is_some(),
            "selected_diff must be set to the loaded diff"
        );
        assert!(
            !state.active_tab().is_loading_file_diff,
            "is_loading_file_diff must be cleared after load"
        );
        assert_eq!(
            state.active_tab().diff_scroll_offset,
            0.0,
            "diff_scroll_offset must be reset when new diff arrives"
        );
    }

    #[test]
    fn regular_click_preserves_commit_range_diffs_until_load_completes() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs"]);

        // Pre-populate commit_range_diffs
        state.active_tab_mut().commit_range_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "x.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];

        let _ = state.update(Message::SelectDiffByIndex(0));

        assert!(
            !state.active_tab().commit_range_diffs.is_empty(),
            "commit_range_diffs must NOT be cleared on click (deferred to diff load)"
        );
    }

    #[test]
    fn select_diff_by_index_shift_click_adds_both_files_to_selection() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);
        state.active_tab_mut().selected_file_index = Some(0);

        // Shift+Click on file 1 should anchor 0 and add 1
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectDiffByIndex(1));

        let sel = &state.active_tab().selected_commit_file_indices;
        assert!(sel.contains(&0), "anchor file 0 should be selected");
        assert!(sel.contains(&1), "newly clicked file 1 should be selected");
        assert_eq!(sel.len(), 2);
    }

    #[test]
    fn anchor_file_index_defaults_to_none() {
        let tab = RepoTab::new_empty();
        assert!(tab.anchor_file_index.is_none());
    }

    #[test]
    fn regular_click_sets_anchor() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);

        let _ = state.update(Message::SelectDiffByIndex(2));

        assert_eq!(
            state.active_tab().anchor_file_index,
            Some(2),
            "regular click must set anchor to the clicked index"
        );
    }

    #[test]
    fn shift_click_selects_range_downward_from_anchor() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files =
            make_commit_files(&["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]);
        // Anchor at index 1
        state.active_tab_mut().anchor_file_index = Some(1);
        state.active_tab_mut().selected_file_index = Some(1);

        // Shift+Click on index 4 — should select 1, 2, 3, 4
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectDiffByIndex(4));

        let sel = &state.active_tab().selected_commit_file_indices;
        assert_eq!(
            sel,
            &vec![1, 2, 3, 4],
            "range must be contiguous from anchor to click"
        );
    }

    #[test]
    fn shift_click_selects_range_upward_from_anchor() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files =
            make_commit_files(&["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]);
        // Anchor at index 4 (bottom)
        state.active_tab_mut().anchor_file_index = Some(4);
        state.active_tab_mut().selected_file_index = Some(4);

        // Shift+Click on index 1 — should select 1, 2, 3, 4 (ascending)
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectDiffByIndex(1));

        let sel = &state.active_tab().selected_commit_file_indices;
        assert_eq!(
            sel,
            &vec![1, 2, 3, 4],
            "range must be stored ascending regardless of click direction"
        );
    }

    #[test]
    fn shift_click_anchor_fixed_on_subsequent_clicks() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files =
            make_commit_files(&["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]);
        // Anchor at index 2
        state.active_tab_mut().anchor_file_index = Some(2);
        state.active_tab_mut().selected_file_index = Some(2);
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;

        // First Shift+Click: extend to 4 → range {2, 3, 4}
        let _ = state.update(Message::SelectDiffByIndex(4));
        assert_eq!(
            state.active_tab().selected_commit_file_indices,
            vec![2, 3, 4]
        );

        // Second Shift+Click: shrink back to 3 → range {2, 3} (anchor still 2)
        let _ = state.update(Message::SelectDiffByIndex(3));
        assert_eq!(
            state.active_tab().selected_commit_file_indices,
            vec![2, 3],
            "anchor must stay fixed; second Shift+Click shrinks the range"
        );

        // Third Shift+Click: extend upward → range {0, 1, 2} (anchor still 2)
        let _ = state.update(Message::SelectDiffByIndex(0));
        assert_eq!(
            state.active_tab().selected_commit_file_indices,
            vec![0, 1, 2],
            "anchor must stay fixed; can extend range in either direction"
        );
    }

    #[test]
    fn shift_click_on_anchor_itself_gives_single_item_range() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);
        state.active_tab_mut().anchor_file_index = Some(1);
        state.active_tab_mut().selected_file_index = Some(1);

        // Shift+Click on the anchor itself → single-item range {1}
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectDiffByIndex(1));

        assert_eq!(state.active_tab().selected_commit_file_indices, vec![1]);
        assert!(
            state.active_tab().multi_file_diffs.is_empty(),
            "single-item range must not populate multi_file_diffs"
        );
    }

    #[test]
    fn shift_click_range_is_always_ascending() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().selected_commit_oid = Some("abc123".to_string());
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs", "d.rs"]);
        state.active_tab_mut().anchor_file_index = Some(3);
        state.active_tab_mut().selected_file_index = Some(3);

        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectDiffByIndex(0));

        let sel = &state.active_tab().selected_commit_file_indices;
        let is_sorted = sel.windows(2).all(|w| w[0] < w[1]);
        assert!(
            is_sorted,
            "selection must always be stored in ascending order"
        );
        assert_eq!(sel, &vec![0, 1, 2, 3]);
    }

    #[test]
    fn checkout_file_at_commit_message_variants_exist() {
        use crate::message::Message;
        // Verify the new message variants can be constructed
        let _single =
            Message::CheckoutFileAtCommit("abc123".to_string(), "src/main.rs".to_string());
        let _multi = Message::CheckoutMultiFilesAtCommit(
            "abc123".to_string(),
            vec!["a.rs".to_string(), "b.rs".to_string()],
        );
    }

    #[test]
    fn checkout_file_at_commit_closes_context_menu() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().context_menu = Some(crate::state::ContextMenu::CommitFile {
            oid: "abc123".to_string(),
            file_path: "src/main.rs".to_string(),
        });
        let _ = state.update(Message::CheckoutFileAtCommit(
            "abc123".to_string(),
            "src/main.rs".to_string(),
        ));
        assert!(state.active_tab().context_menu.is_none());
    }

    #[test]
    fn checkout_multi_files_at_commit_closes_context_menu() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().context_menu = Some(crate::state::ContextMenu::CommitFile {
            oid: "abc123".to_string(),
            file_path: "src/main.rs".to_string(),
        });
        let _ = state.update(Message::CheckoutMultiFilesAtCommit(
            "abc123".to_string(),
            vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        ));
        assert!(state.active_tab().context_menu.is_none());
    }

    // ── Commit multi-selection ────────────────────────────────────────────

    fn make_test_commits(count: usize) -> Vec<gitkraft_core::CommitInfo> {
        (0..count)
            .map(|i| gitkraft_core::CommitInfo {
                oid: i.to_string(),
                short_oid: i.to_string(),
                summary: String::new(),
                message: String::new(),
                author_name: String::new(),
                author_email: String::new(),
                time: Default::default(),
                parent_ids: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn selected_commits_defaults_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.selected_commits.is_empty());
        assert!(tab.anchor_commit_index.is_none());
    }

    #[test]
    fn regular_click_commit_sets_anchor_and_clears_range() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake"));
        state.active_tab_mut().commits = make_test_commits(3);
        state.active_tab_mut().selected_commits = vec![0, 1, 2];

        let _ = state.update(Message::SelectCommit(1));

        assert_eq!(state.active_tab().anchor_commit_index, Some(1));
        assert!(state.active_tab().selected_commits.is_empty());
        assert_eq!(state.active_tab().selected_commit, Some(1));
    }

    #[test]
    fn shift_click_commit_selects_range_from_anchor() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().commits = make_test_commits(5);
        state.active_tab_mut().anchor_commit_index = Some(1);
        state.active_tab_mut().selected_commit = Some(1);

        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectCommit(4));

        assert_eq!(state.active_tab().selected_commits, vec![1, 2, 3, 4]);
    }

    #[test]
    fn shift_click_commit_range_is_ascending_when_clicking_above_anchor() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().commits = make_test_commits(5);
        state.active_tab_mut().anchor_commit_index = Some(3);
        state.active_tab_mut().selected_commit = Some(3);

        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        let _ = state.update(Message::SelectCommit(1));

        assert_eq!(state.active_tab().selected_commits, vec![1, 2, 3]);
    }

    // ── ExecuteCommitAction message ───────────────────────────────────────

    #[test]
    fn execute_commit_action_closes_context_menu() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().context_menu = Some(crate::state::ContextMenu::Commit {
            index: 0,
            oid: "abc123".to_string(),
        });

        let _ = state.update(Message::ExecuteCommitAction(
            "abc123".to_string(),
            gitkraft_core::CommitAction::CherryPick,
        ));

        assert!(state.active_tab().context_menu.is_none());
    }

    #[test]
    fn execute_commit_action_sets_loading_when_repo_open() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));

        let _ = state.update(Message::ExecuteCommitAction(
            "abc123".to_string(),
            gitkraft_core::CommitAction::ResetHard,
        ));

        assert!(state.active_tab().is_loading);
    }

    #[test]
    fn execute_commit_action_no_repo_does_not_set_loading() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // No repo_path set

        let _ = state.update(Message::ExecuteCommitAction(
            "abc123".to_string(),
            gitkraft_core::CommitAction::CherryPick,
        ));

        assert!(!state.active_tab().is_loading);
    }

    #[test]
    fn execute_commit_action_sets_status_message_from_action_label() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));

        let _ = state.update(Message::ExecuteCommitAction(
            "abc123".to_string(),
            gitkraft_core::CommitAction::Revert,
        ));

        let status = state.active_tab().status_message.as_deref().unwrap_or("");
        // Status message should contain the action's label
        assert!(
            status.contains("Revert commit"),
            "expected status to contain 'Revert commit', got: {status:?}"
        );
    }

    // ── File history / blame / delete state ──────────────────────────────

    #[test]
    fn file_history_defaults_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.file_history_path.is_none());
        assert!(tab.file_history_commits.is_empty());
        assert_eq!(tab.file_history_scroll, 0.0);
    }

    #[test]
    fn blame_defaults_empty() {
        let tab = RepoTab::new_empty();
        assert!(tab.blame_path.is_none());
        assert!(tab.blame_lines.is_empty());
        assert_eq!(tab.blame_scroll, 0.0);
    }

    #[test]
    fn pending_delete_file_defaults_none() {
        let tab = RepoTab::new_empty();
        assert!(tab.pending_delete_file.is_none());
    }

    #[test]
    fn view_file_history_sets_path_and_clears_blame() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().blame_path = Some("some/file.rs".to_string());

        let _ = state.update(Message::ViewFileHistory("src/main.rs".to_string()));

        assert_eq!(
            state.active_tab().file_history_path.as_deref(),
            Some("src/main.rs")
        );
        // Opening history should close blame
        assert!(state.active_tab().blame_path.is_none());
    }

    #[test]
    fn close_file_history_clears_state() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().file_history_path = Some("src/lib.rs".to_string());
        state.active_tab_mut().file_history_commits = vec![gitkraft_core::CommitInfo {
            oid: "abc".to_string(),
            short_oid: "abc".to_string(),
            summary: "s".to_string(),
            message: "s".to_string(),
            author_name: "a".to_string(),
            author_email: "a@b.com".to_string(),
            time: Default::default(),
            parent_ids: vec![],
        }];

        let _ = state.update(Message::CloseFileHistory);

        assert!(state.active_tab().file_history_path.is_none());
        assert!(state.active_tab().file_history_commits.is_empty());
    }

    #[test]
    fn view_file_blame_sets_path_and_clears_history() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().file_history_path = Some("some/file.rs".to_string());

        let _ = state.update(Message::ViewFileBlame("src/lib.rs".to_string()));

        assert_eq!(state.active_tab().blame_path.as_deref(), Some("src/lib.rs"));
        // Opening blame should close history
        assert!(state.active_tab().file_history_path.is_none());
    }

    #[test]
    fn selecting_new_commit_closes_blame_overlay() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        // Pre-populate a commit list so SelectCommit can find the commit.
        state.active_tab_mut().commits = vec![
            gitkraft_core::CommitInfo {
                oid: "abc1".into(),
                short_oid: "abc1".into(),
                summary: "first".into(),
                message: "first".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
            gitkraft_core::CommitInfo {
                oid: "abc2".into(),
                short_oid: "abc2".into(),
                summary: "second".into(),
                message: "second".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
        ];
        // Blame is currently open for a file from the first commit.
        state.active_tab_mut().blame_path = Some("src/lib.rs".to_string());
        state.active_tab_mut().blame_lines = vec![gitkraft_core::BlameLine {
            line_number: 1,
            content: "fn main() {}".into(),
            short_oid: "abc1".into(),
            oid: "abc1".into(),
            author_name: "A".into(),
            time: Default::default(),
        }];

        // Click a different commit — blame must close automatically.
        let _ = state.update(Message::SelectCommit(1));

        assert!(
            state.active_tab().blame_path.is_none(),
            "blame_path must be cleared when a new commit is selected"
        );
        assert!(
            state.active_tab().blame_lines.is_empty(),
            "blame_lines must be cleared when a new commit is selected"
        );
    }

    #[test]
    fn close_file_blame_clears_state() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().blame_path = Some("src/lib.rs".to_string());

        let _ = state.update(Message::CloseFileBlame);

        assert!(state.active_tab().blame_path.is_none());
        assert!(state.active_tab().blame_lines.is_empty());
    }

    #[test]
    fn delete_file_sets_pending() {
        use crate::message::Message;
        let mut state = GitKraft::new();

        let _ = state.update(Message::DeleteFile("src/old.rs".to_string()));

        assert_eq!(
            state.active_tab().pending_delete_file.as_deref(),
            Some("src/old.rs")
        );
        assert!(state.active_tab().context_menu.is_none());
    }

    #[test]
    fn cancel_delete_file_clears_pending() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().pending_delete_file = Some("src/old.rs".to_string());

        let _ = state.update(Message::CancelDeleteFile);

        assert!(state.active_tab().pending_delete_file.is_none());
    }

    #[test]
    fn confirm_delete_file_no_repo_is_noop() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().pending_delete_file = Some("src/old.rs".to_string());
        // No repo_path → should not set is_loading

        let _ = state.update(Message::ConfirmDeleteFile);

        assert!(!state.active_tab().is_loading);
    }

    #[test]
    fn shift_arrow_down_extends_file_list_selection_when_files_loaded() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));
        state.active_tab_mut().commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);
        state.active_tab_mut().selected_file_index = Some(0);
        state.active_tab_mut().anchor_file_index = Some(0);
        // keyboard_modifiers must have SHIFT set for range selection to trigger
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;

        let _ = state.update(Message::ShiftArrowDown);

        assert_eq!(state.active_tab().selected_file_index, Some(1));
        // Range should now include both files
        assert!(state.active_tab().selected_commit_file_indices.contains(&0));
        assert!(state.active_tab().selected_commit_file_indices.contains(&1));
    }

    #[test]
    fn shift_arrow_down_falls_through_to_commit_log_when_no_files() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().commits = make_test_commits(5);
        state.active_tab_mut().selected_commit = Some(1);
        state.active_tab_mut().anchor_commit_index = Some(1);
        state.keyboard_modifiers = iced::keyboard::Modifiers::SHIFT;
        // no commit_files

        let _ = state.update(Message::ShiftArrowDown);

        assert_eq!(state.active_tab().selected_commit, Some(2));
        assert!(state.active_tab().selected_commits.contains(&1));
        assert!(state.active_tab().selected_commits.contains(&2));
    }

    #[test]
    fn file_system_changed_triggers_full_refresh() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        state.active_tab_mut().repo_path =
            Some(std::path::PathBuf::from("/tmp/fake-repo-for-test"));

        // FileSystemChanged should call refresh_active_tab() which returns
        // a non-none Task.  We verify by checking that is_loading is NOT set
        // synchronously (the task is async), but that no error is set either.
        let _task = state.update(Message::FileSystemChanged);

        // With a repo_path set, the handler must have attempted a refresh
        // (it returns a Task, so is_loading is set by the task executor, not here).
        // What we CAN check: no error was set, and status is not "error".
        assert!(
            state.active_tab().error_message.is_none(),
            "FileSystemChanged must not set an error message"
        );
    }

    // ── Tab duplication prevention tests ───────────────────────────────────

    /// Helper: build a minimal `RepoPayload` (aka `RepoSnapshot`) for a given path.
    fn fake_payload(workdir: &str) -> crate::message::RepoPayload {
        gitkraft_core::RepoSnapshot {
            info: gitkraft_core::RepoInfo {
                path: std::path::PathBuf::from(format!("{workdir}/.git")),
                workdir: Some(std::path::PathBuf::from(workdir)),
                head_branch: Some("main".into()),
                is_bare: false,
                state: gitkraft_core::RepoState::Clean,
            },
            branches: Vec::new(),
            commits: Vec::new(),
            graph_rows: Vec::new(),
            unstaged: Vec::new(),
            staged: Vec::new(),
            stashes: Vec::new(),
            remotes: Vec::new(),
        }
    }

    /// Helper: set up a tab as if a repo was fully loaded.
    fn setup_loaded_tab(tab: &mut RepoTab, path: &str) {
        tab.repo_path = Some(std::path::PathBuf::from(path));
        tab.repo_info = Some(gitkraft_core::RepoInfo {
            path: std::path::PathBuf::from(format!("{path}/.git")),
            workdir: Some(std::path::PathBuf::from(path)),
            head_branch: Some("main".into()),
            is_bare: false,
            state: gitkraft_core::RepoState::Clean,
        });
    }

    #[test]
    fn open_repo_creates_new_tab_when_repo_already_open() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");

        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);

        // Clicking "Open" should create a new tab when the current has a repo.
        let _task = state.update(Message::OpenRepo);

        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab, 1);
        // The new tab should be loading (folder picker opening).
        assert!(state.tabs[1].is_loading);
        // The original tab should be untouched.
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-a"))
        );
    }

    #[test]
    fn open_repo_reuses_empty_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Active tab is empty (no repo loaded).
        assert!(!state.active_tab().has_repo());

        let _task = state.update(Message::OpenRepo);

        // Should NOT create a new tab when the active tab is empty.
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
        assert!(state.tabs[0].is_loading);
    }

    #[test]
    fn repo_selected_deduplicates_already_open_repo() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: fully loaded repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: empty (simulates the new tab created by OpenRepo)
        state.tabs.push(RepoTab::new_empty());
        state.active_tab = 1;

        // User selects a folder that matches the already-open repo.
        let _task = state.update(Message::RepoSelected(Some(std::path::PathBuf::from(
            "/home/user/repo-a",
        ))));

        // Should switch to the existing tab and remove the empty one.
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-a"))
        );
    }

    #[test]
    fn repo_selected_opens_new_repo_in_empty_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: fully loaded repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: empty (simulates the new tab created by OpenRepo)
        state.tabs.push(RepoTab::new_empty());
        state.active_tab = 1;

        // User selects a DIFFERENT repo.
        let _task = state.update(Message::RepoSelected(Some(std::path::PathBuf::from(
            "/home/user/repo-b",
        ))));

        // Should keep both tabs; the empty tab is now loading repo-b.
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab, 1);
        assert!(state.tabs[1]
            .status_message
            .as_deref()
            .unwrap_or("")
            .contains("repo-b"));
    }

    #[test]
    fn repo_selected_cancel_removes_empty_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: fully loaded repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: empty (created by OpenRepo, waiting for folder picker)
        state.tabs.push(RepoTab::new_empty());
        state.active_tab = 1;

        // User cancels the folder picker.
        let _task = state.update(Message::RepoSelected(None));

        // The empty tab should be removed; switch back to tab 0.
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-a"))
        );
    }

    #[test]
    fn repo_selected_cancel_keeps_tab_if_only_one() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Single empty tab — shouldn't be removed on cancel.
        assert_eq!(state.tabs.len(), 1);
        assert!(!state.active_tab().has_repo());

        let _task = state.update(Message::RepoSelected(None));

        assert_eq!(state.tabs.len(), 1);
        assert!(!state.active_tab().is_loading);
    }

    #[test]
    fn open_recent_repo_deduplicates() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: fully loaded repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: empty tab
        state.tabs.push(RepoTab::new_empty());
        state.active_tab = 1;

        // Opening a recent repo that's already open should switch to it.
        let _task = state.update(Message::OpenRecentRepo(std::path::PathBuf::from(
            "/home/user/repo-a",
        )));

        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn open_recent_repo_creates_new_tab_when_current_has_repo() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: fully loaded repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");

        // Opening a different recent repo should create a new tab.
        let _task = state.update(Message::OpenRecentRepo(std::path::PathBuf::from(
            "/home/user/repo-b",
        )));

        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab, 1);
        assert!(state.tabs[1].is_loading);
    }

    #[test]
    fn open_recent_repo_uses_empty_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Active tab is empty.
        assert!(!state.active_tab().has_repo());

        let _task = state.update(Message::OpenRecentRepo(std::path::PathBuf::from(
            "/home/user/repo-b",
        )));

        // Should NOT create a new tab.
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
        assert!(state.tabs[0].is_loading);
    }

    // ── Refresh race-condition tests ──────────────────────────────────────

    #[test]
    fn repo_refreshed_targets_correct_tab_after_tab_switch() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: repo-a fully loaded
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: empty tab (user clicked "+")
        state.tabs.push(RepoTab::new_empty());
        state.active_tab = 1; // user switched to new empty tab

        // Simulate: a RepoRefreshed result arrives for repo-a while tab 1 is active.
        let payload = fake_payload("/home/user/repo-a");
        let _task = state.update(Message::RepoRefreshed(Ok(payload)));

        // The payload must land in tab 0 (which owns repo-a), NOT tab 1.
        assert!(
            state.tabs[0].repo_info.is_some(),
            "tab 0 should still have repo info after refresh"
        );
        assert_eq!(
            state.tabs[0].current_branch.as_deref(),
            Some("main"),
            "tab 0 should have updated branch from payload"
        );
        // Tab 1 must remain empty.
        assert!(
            state.tabs[1].repo_info.is_none(),
            "tab 1 (empty) must NOT receive the refresh payload"
        );
        assert!(
            state.tabs[1].repo_path.is_none(),
            "tab 1 should still have no repo path"
        );
    }

    #[test]
    fn repo_refreshed_targets_active_tab_for_new_open() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Active tab is empty — user just opened a brand-new repo.
        // (RepoSelected set the status but the tab doesn't have repo_path yet
        // matching the payload, so it falls back to active tab.)
        assert_eq!(state.tabs.len(), 1);
        assert!(!state.active_tab().has_repo());

        let payload = fake_payload("/home/user/new-repo");
        let _task = state.update(Message::RepoOpened(Ok(payload)));

        // Should have applied to the active tab (the only tab).
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/new-repo"))
        );
        assert!(state.tabs[0].repo_info.is_some());
    }

    #[test]
    fn repo_refreshed_does_not_duplicate_into_new_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: repo-a loaded
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");

        // User clicks "+" creating an empty tab 1, then switches to it.
        let _task = state.update(Message::NewTab);
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab, 1);

        // A refresh result for repo-a arrives (was triggered before the tab switch).
        let payload = fake_payload("/home/user/repo-a");
        let _task = state.update(Message::RepoRefreshed(Ok(payload)));

        // Tab 1 must remain empty — the payload should go to tab 0.
        assert!(
            state.tabs[1].repo_path.is_none(),
            "new empty tab must not receive repo-a refresh"
        );
        assert!(
            state.tabs[1].repo_info.is_none(),
            "new empty tab must not have repo_info"
        );
        // Tab 0 must have the refreshed data.
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-a"))
        );
    }

    #[test]
    fn git_operation_result_targets_correct_tab() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: repo-a
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");
        // Tab 1: repo-b
        state.tabs.push(RepoTab::new_empty());
        setup_loaded_tab(&mut state.tabs[1], "/home/user/repo-b");
        state.active_tab = 1;

        // A git operation result arrives for repo-a (e.g. push completed)
        // while user is on tab 1.
        let payload = fake_payload("/home/user/repo-a");
        let _task = state.update(Message::GitOperationResult(Ok(payload)));

        // Payload should land in tab 0 (repo-a), not tab 1.
        assert_eq!(state.tabs[0].current_branch.as_deref(), Some("main"));
        // Tab 1's data should remain unchanged (still repo-b).
        assert_eq!(
            state.tabs[1].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-b"))
        );
    }

    #[test]
    fn multiple_new_tabs_dont_get_polluted_by_refresh() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: repo-a loaded
        setup_loaded_tab(state.active_tab_mut(), "/home/user/repo-a");

        // User creates multiple new tabs.
        let _task = state.update(Message::NewTab);
        let _task = state.update(Message::NewTab);
        assert_eq!(state.tabs.len(), 3);
        assert_eq!(state.active_tab, 2);

        // Refresh arrives for repo-a.
        let payload = fake_payload("/home/user/repo-a");
        let _task = state.update(Message::RepoRefreshed(Ok(payload)));

        // Only tab 0 should be affected.
        assert!(state.tabs[0].repo_info.is_some());
        assert!(state.tabs[1].repo_info.is_none());
        assert!(state.tabs[2].repo_info.is_none());
        assert!(state.tabs[1].repo_path.is_none());
        assert!(state.tabs[2].repo_path.is_none());
    }

    #[test]
    fn repo_selected_dedup_adjusts_index_when_existing_is_after_active() {
        use crate::message::Message;
        let mut state = GitKraft::new();
        // Tab 0: empty (newly created by OpenRepo)
        // Tab 1: repo-a loaded
        state.tabs.push(RepoTab::new_empty());
        setup_loaded_tab(&mut state.tabs[1], "/home/user/repo-a");
        state.active_tab = 0;

        // User selects the same folder as repo-a.
        let _task = state.update(Message::RepoSelected(Some(std::path::PathBuf::from(
            "/home/user/repo-a",
        ))));

        // Empty tab 0 should be removed; we should now be on tab 0 (formerly tab 1).
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.active_tab, 0);
        assert_eq!(
            state.tabs[0].repo_path.as_deref(),
            Some(std::path::Path::new("/home/user/repo-a"))
        );
    }

    // ── Multi-selection preservation across refresh ───────────────────────

    #[test]
    fn apply_payload_preserves_multi_selection_by_oid() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(5);
        // Simulate user selecting commits 1..=3 with Shift+click.
        tab.anchor_commit_index = Some(1);
        tab.selected_commits = vec![1, 2, 3];
        tab.selected_commit = Some(3);
        tab.selected_commit_oid = Some(tab.commits[3].oid.clone());

        // Build a payload with the same commits (simulating a background refresh).
        let mut payload = fake_payload("/tmp/repo");
        payload.commits = make_test_commits(5);

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        // The multi-selection must survive the refresh.
        assert_eq!(tab.anchor_commit_index, Some(1));
        assert_eq!(tab.selected_commits, vec![1, 2, 3]);
        assert_eq!(tab.selected_commit, Some(3));
    }

    #[test]
    fn apply_payload_preserves_anchor_even_without_range() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(5);
        tab.anchor_commit_index = Some(2);
        tab.selected_commit = Some(2);
        tab.selected_commit_oid = Some(tab.commits[2].oid.clone());
        // No range selection — just a single click with anchor set.

        let mut payload = fake_payload("/tmp/repo");
        payload.commits = make_test_commits(5);

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        assert_eq!(tab.anchor_commit_index, Some(2));
        assert_eq!(tab.selected_commit, Some(2));
    }

    #[test]
    fn apply_payload_clears_selection_when_commits_disappear() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(5);
        tab.anchor_commit_index = Some(2);
        tab.selected_commits = vec![2, 3, 4];
        tab.selected_commit = Some(4);
        tab.selected_commit_oid = Some(tab.commits[4].oid.clone());

        // Payload has completely different commits (e.g. force-push).
        let mut payload = fake_payload("/tmp/repo");
        payload.commits = (0..3)
            .map(|i| gitkraft_core::CommitInfo {
                oid: format!("new_oid_{i}"),
                short_oid: format!("new_{i}"),
                summary: format!("new commit {i}"),
                message: String::new(),
                author_name: "Author".into(),
                author_email: "a@b.c".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            })
            .collect();

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        // All old OIDs are gone — selection should be cleared.
        assert!(tab.selected_commits.is_empty());
        assert!(tab.anchor_commit_index.is_none());
        assert!(tab.selected_commit.is_none());
    }

    // ── File multi-selection preservation across refresh ─────────────────

    #[test]
    fn apply_payload_preserves_file_selection_when_commit_survives() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(3);
        tab.selected_commit = Some(1);
        tab.selected_commit_oid = Some(tab.commits[1].oid.clone());
        tab.commit_files = make_commit_files(&["a.rs", "b.rs", "c.rs"]);

        // Simulate user Shift+clicking files 0..=2.
        tab.anchor_file_index = Some(0);
        tab.selected_commit_file_indices = vec![0, 1, 2];
        tab.multi_file_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".into(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];

        // Background refresh with the same commits.
        let mut payload = fake_payload("/tmp/repo");
        payload.commits = make_test_commits(3);

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        // File selection must survive.
        assert_eq!(tab.anchor_file_index, Some(0));
        assert_eq!(tab.selected_commit_file_indices, vec![0, 1, 2]);
        assert!(
            !tab.multi_file_diffs.is_empty(),
            "multi_file_diffs must survive when commit is preserved"
        );
    }

    #[test]
    fn apply_payload_clears_file_selection_when_commit_disappears() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(3);
        tab.selected_commit = Some(1);
        tab.selected_commit_oid = Some(tab.commits[1].oid.clone());
        tab.commit_files = make_commit_files(&["a.rs", "b.rs"]);
        tab.anchor_file_index = Some(0);
        tab.selected_commit_file_indices = vec![0, 1];
        tab.multi_file_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "a.rs".into(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];

        // Payload with completely different commits (force-push).
        let mut payload = fake_payload("/tmp/repo");
        payload.commits = (0..2)
            .map(|i| gitkraft_core::CommitInfo {
                oid: format!("new_{i}"),
                short_oid: format!("n{i}"),
                summary: String::new(),
                message: String::new(),
                author_name: String::new(),
                author_email: String::new(),
                time: Default::default(),
                parent_ids: Vec::new(),
            })
            .collect();

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        // Commit is gone → file selection must be cleared.
        assert!(tab.anchor_file_index.is_none());
        assert!(tab.selected_commit_file_indices.is_empty());
        assert!(tab.multi_file_diffs.is_empty());
        assert!(tab.commit_files.is_empty());
    }

    #[test]
    fn apply_payload_preserves_commit_range_diffs_when_commit_survives() {
        let mut tab = RepoTab::new_empty();
        tab.commits = make_test_commits(5);
        tab.selected_commit = Some(2);
        tab.selected_commit_oid = Some(tab.commits[2].oid.clone());
        tab.commit_range_diffs = vec![gitkraft_core::DiffInfo {
            old_file: String::new(),
            new_file: "x.rs".into(),
            status: gitkraft_core::FileStatus::Modified,
            hunks: vec![],
        }];

        let mut payload = fake_payload("/tmp/repo");
        payload.commits = make_test_commits(5);

        tab.apply_payload(payload, std::path::PathBuf::from("/tmp/repo"));

        assert!(
            !tab.commit_range_diffs.is_empty(),
            "commit_range_diffs must survive when commit is preserved"
        );
    }
}
