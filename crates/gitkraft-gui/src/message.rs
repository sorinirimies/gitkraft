//! Unified message type for the entire GitKraft GUI application.
//!
//! Every user interaction, async result callback, and internal event is
//! represented as a variant of [`Message`]. The top-level `update` function
//! pattern-matches on these and delegates to the appropriate feature handler.

use std::path::PathBuf;

use gitkraft_core::{BranchInfo, CommitInfo, DiffFileEntry, DiffInfo, GraphRow, RemoteInfo, RepoInfo, StashEntry};

// ── Payload types ─────────────────────────────────────────────────────────────

/// Payload returned after successfully opening / refreshing a repository.
/// Bundles every piece of state the UI needs so we can update in one shot.
#[derive(Debug, Clone)]
pub struct RepoPayload {
    pub info: RepoInfo,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
    pub graph_rows: Vec<GraphRow>,
    pub unstaged: Vec<DiffInfo>,
    pub staged: Vec<DiffInfo>,
    pub stashes: Vec<StashEntry>,
    pub remotes: Vec<RemoteInfo>,
}

/// Payload returned after a staging operation (stage / unstage / discard).
#[derive(Debug, Clone)]
pub struct StagingPayload {
    pub unstaged: Vec<DiffInfo>,
    pub staged: Vec<DiffInfo>,
}

/// Payload returned by a lazy-load page request.
#[derive(Debug, Clone)]
pub struct CommitPage {
    pub commits: Vec<gitkraft_core::CommitInfo>,
    pub graph_rows: Vec<gitkraft_core::GraphRow>,
}

// ── Message enum ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // ── Tabs ──────────────────────────────────────────────────────────────
    /// User clicked a tab in the tab bar.
    SwitchTab(usize),
    /// User clicked the "+" button to open a new empty tab.
    NewTab,
    /// User clicked the close (×) button on a tab.
    CloseTab(usize),

    // ── Repository ────────────────────────────────────────────────────────
    /// User clicked "Open Repository" — launch the folder picker.
    OpenRepo,
    /// User clicked "Init Repository" — launch the folder picker for init.
    InitRepo,
    /// Folder picker returned a path (or was cancelled → `None`).
    RepoSelected(Option<PathBuf>),
    /// Folder picker returned a path for init (or was cancelled → `None`).
    RepoInitSelected(Option<PathBuf>),
    /// Async repo-open completed.
    RepoOpened(Result<RepoPayload, String>),
    /// User requested a full refresh of the current repo.
    RefreshRepo,
    /// Async refresh completed.
    RepoRefreshed(Result<RepoPayload, String>),

    // ── Branches ──────────────────────────────────────────────────────────
    /// User clicked a branch name → checkout that branch.
    CheckoutBranch(String),
    /// Async checkout completed.
    BranchCheckedOut(Result<(), String>),
    /// User submitted the new-branch form.
    CreateBranch,
    /// User is typing a new branch name.
    NewBranchNameChanged(String),
    /// Async branch creation completed.
    BranchCreated(Result<(), String>),
    /// User clicked the delete button next to a branch.
    DeleteBranch(String),
    /// Async branch deletion completed.
    BranchDeleted(Result<(), String>),
    /// Toggle visibility of the new-branch inline form.
    ToggleBranchCreate,
    /// Collapse or expand the Local branches section.
    ToggleLocalBranches,
    /// Collapse or expand the Remote branches section.
    ToggleRemoteBranches,

    // ── Commits ───────────────────────────────────────────────────────────
    /// User clicked a commit row in the log.
    SelectCommit(usize),
    /// Async commit file-list load completed (lightweight — no diff content).
    CommitFileListLoaded(Result<Vec<DiffFileEntry>, String>),
    /// Async single-file diff load completed.
    SingleFileDiffLoaded(Result<DiffInfo, String>),
    /// The commit log scrollable was scrolled.
    /// Carries `(absolute_y, relative_y)` — absolute for virtual-window
    /// positioning, relative (0.0 = top, 1.0 = bottom) for load-more trigger.
    CommitLogScrolled(f32, f32),
    /// The diff viewer scrollable was scrolled — carries `absolute_y`.
    DiffViewScrolled(f32),
    /// A lazy-loaded page of additional commits was fetched from the background.
    MoreCommitsLoaded(Result<CommitPage, String>),

    // ── Staging ───────────────────────────────────────────────────────────
    /// Stage a single file by its path.
    StageFile(String),
    /// Unstage a single file by its path.
    UnstageFile(String),
    /// Stage all unstaged files.
    StageAll,
    /// Unstage all staged files.
    UnstageAll,
    /// Discard working-directory changes for a file.
    DiscardFile(String),
    /// User confirmed the discard for a file.
    ConfirmDiscard(String),
    /// User cancelled a pending discard.
    CancelDiscard,
    /// Async staging operation completed.
    StagingUpdated(Result<StagingPayload, String>),

    // ── Commit creation ───────────────────────────────────────────────────
    /// User is typing in the commit-message input.
    CommitMessageChanged(String),
    /// User clicked "Commit".
    CreateCommit,
    /// Async commit creation completed.
    CommitCreated(Result<(), String>),

    // ── Stash ─────────────────────────────────────────────────────────────
    /// Save the current working state as a stash.
    StashSave,
    /// Pop (apply + drop) a stash by index.
    StashPop(usize),
    /// Drop (delete) a stash by index without applying.
    StashDrop(usize),
    /// Async stash operation completed.
    StashUpdated(Result<Vec<StashEntry>, String>),
    /// User is typing in the stash-message input.
    StashMessageChanged(String),

    // ── Remotes ───────────────────────────────────────────────────────────
    /// Fetch from the first configured remote.
    Fetch,
    /// Async fetch completed.
    FetchCompleted(Result<(), String>),

    // ── UI ────────────────────────────────────────────────────────────────
    /// User clicked a file in the commit-diff file list (by index into `commit_files`).
    SelectDiffByIndex(usize),
    /// User clicked a file in the staging area to view its diff.
    SelectDiff(DiffInfo),
    /// Dismiss the current error banner.
    DismissError,
    /// Toggle the left sidebar.
    ToggleSidebar,
    /// Close the current repository and return to the welcome screen.
    CloseRepo,

    // ── Pane resize ───────────────────────────────────────────────────────
    /// User pressed the mouse button on a vertical divider to start dragging.
    PaneDragStart(crate::state::DragTarget, f32),
    /// User pressed the mouse button on the horizontal staging divider.
    PaneDragStartH(crate::state::DragTargetH, f32),
    /// Mouse moved during a drag — `(x, y)` in window coordinates.
    PaneDragMove(f32, f32),
    /// Mouse button released — stop dragging.
    PaneDragEnd,
    // ── Async persistence results ─────────────────────────────────────
    /// Background `record_repo_opened` + `load_settings` completed.
    /// Carries the refreshed recent-repos list (or an error string).
    RepoRecorded(Result<Vec<gitkraft_core::RepoHistoryEntry>, String>),
    /// Background `load_settings` completed (e.g. after closing a repo).
    SettingsLoaded(Result<Vec<gitkraft_core::RepoHistoryEntry>, String>),
    /// Background `save_theme` completed (fire-and-forget, errors logged).
    ThemeSaved(Result<(), String>),
    /// Background layout save completed (fire-and-forget, errors logged).
    LayoutSaved(Result<(), String>),
    /// Layout loaded from persisted settings on startup.
    LayoutLoaded(Result<Option<gitkraft_core::LayoutSettings>, String>),
    /// Background session save completed (fire-and-forget).
    SessionSaved(Result<(), String>),
    /// Async restore of a specific tab (by index) completed on startup.
    RepoRestoredAt(usize, Result<RepoPayload, String>),

    // ── Context menus ─────────────────────────────────────────────────────────────
    /// User right-clicked a local branch.
    /// Payload: (branch_name, index_in_local_list, is_current_branch).
    OpenBranchContextMenu(String, usize, bool),


    /// User right-clicked a remote branch.
    OpenRemoteBranchContextMenu(String),

    /// Checkout a remote branch (creates local tracking branch).
    CheckoutRemoteBranch(String),

    /// Delete a remote branch.
    DeleteRemoteBranch(String),
    /// User right-clicked a commit row.
    OpenCommitContextMenu(usize),

    /// Dismiss the context menu without taking an action.
    CloseContextMenu,

    // ── Branch actions ────────────────────────────────────────────────────────────
    /// Push the named branch to its default remote.
    PushBranch(String),

    /// Pull the current branch from its remote, rebasing local commits on top.
    PullBranch(String),

    /// Rebase the current HEAD onto `target` (a branch name or OID string).
    RebaseOnto(String),

    /// Begin an inline rename: record the branch being renamed and pre-fill input.
    BeginRenameBranch(String),

    /// User is typing in the rename input.
    RenameBranchInputChanged(String),

    /// User confirmed the rename.
    ConfirmRenameBranch,

    /// User cancelled the rename.
    CancelRename,

    /// Merge a named branch into the current HEAD branch.
    MergeBranch(String),

    /// Begin an inline tag-creation form at the given commit OID.
    /// The bool indicates whether this is an annotated tag (true) or lightweight (false).
    BeginCreateTag(String, bool),

    /// User is typing in the tag name input.
    TagNameChanged(String),

    /// User is typing in the annotated tag message input.
    TagMessageChanged(String),

    /// User confirmed tag creation.
    ConfirmCreateTag,

    /// User cancelled tag creation.
    CancelCreateTag,

    // ── Commit actions ────────────────────────────────────────────────────────────
    /// Checkout a specific commit in detached HEAD mode.
    CheckoutCommitDetached(String),

    /// Rebase the current branch on top of a specific commit.
    RebaseOntoCommit(String),

    /// Revert a specific commit (creates a revert commit).
    RevertCommit(String),

    /// git reset --soft <oid> — move HEAD, keep staged + working changes.
    ResetSoft(String),
    /// git reset --mixed <oid> — move HEAD and unstage; keep working directory.
    ResetMixed(String),
    /// git reset --hard <oid> — move HEAD and discard all uncommitted changes.
    ResetHard(String),

    // ── Shared ───────────────────────────────────────────────────────────────────
    /// Copy a string to the system clipboard.
    CopyText(String),

    /// Generic result for any git operation that produces a full repo refresh.
    /// The operation itself is responsible for a descriptive error string.
    GitOperationResult(Result<RepoPayload, String>),

    /// User selected a different theme from the picker (by index into
    /// `gitkraft_core::THEME_NAMES`).
    ThemeChanged(usize),
    /// User clicked a recent repository entry on the welcome screen.
    OpenRecentRepo(PathBuf),
    /// No-op (used for disabled buttons, etc.).
    Noop,
}
