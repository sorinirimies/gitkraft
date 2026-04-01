use std::path::PathBuf;

use gitkraft_core::*;

/// Which panel is currently focused / active in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePanel {
    Sidebar,
    CommitList,
    DiffView,
    StagingArea,
}

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
    /// Which panel is currently active / focused.
    pub active_panel: ActivePanel,
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
    /// The Iced theme used for rendering.
    pub theme: iced::Theme,

    // ── Branch creation ───────────────────────────────────────────────────
    /// Text in the "new branch name" input.
    pub new_branch_name: String,
    /// Whether the inline branch-creation UI is visible.
    pub show_branch_create: bool,

    // ── Stash message ─────────────────────────────────────────────────────
    /// Text in the "stash message" input.
    pub stash_message: String,
}

impl GitKraft {
    /// Create a fresh application state with sensible defaults.
    pub fn new() -> Self {
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

            active_panel: ActivePanel::CommitList,
            show_commit_detail: false,
            sidebar_expanded: true,

            status_message: None,
            error_message: None,
            is_loading: false,

            theme: iced::Theme::Dark,

            new_branch_name: String::new(),
            show_branch_create: false,

            stash_message: String::new(),
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
}
