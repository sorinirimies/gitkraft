//! Unified message type for the entire GitKraft GUI application.
//!
//! Every user interaction, async result callback, and internal event is
//! represented as a variant of [`Message`]. The top-level `update` function
//! pattern-matches on these and delegates to the appropriate feature handler.

use std::path::PathBuf;

use gitkraft_core::{DiffFileEntry, DiffInfo, StashEntry};

// ── Payload types ─────────────────────────────────────────────────────────────

/// Payload returned after successfully opening / refreshing a repository.
///
/// This is now a re-export of the shared core type so both GUI and TUI use
/// the same struct definition.
pub type RepoPayload = gitkraft_core::RepoSnapshot;

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
    /// Diff a file from a commit against the working tree.
    DiffFileWithWorkingTree(String, String), // (oid, file_path)
    /// Result of diffing a file with the working tree.
    DiffWithWorkingTreeLoaded(Result<DiffInfo, String>),
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
    /// Toggle selection of an unstaged file (Shift+Click).
    ToggleSelectUnstaged(String),
    /// Toggle selection of a staged file (Shift+Click).
    ToggleSelectStaged(String),
    /// Stage all currently selected unstaged files.
    StageSelected,
    /// Unstage all currently selected staged files.
    UnstageSelected,
    /// Discard all currently selected unstaged files.
    DiscardSelected,
    /// Discard a staged file (unstage + discard working dir changes).
    DiscardStagedFile(String),
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
    /// User right-clicked a stash entry.
    OpenStashContextMenu(usize),
    /// User right-clicked a file in the commit diff file list.
    OpenCommitFileContextMenu(String, String), // (oid, file_path)
    /// User right-clicked an unstaged file.
    OpenUnstagedFileContextMenu(String),
    /// User right-clicked a staged file.
    OpenStagedFileContextMenu(String),
    /// User wants to view the diff of a stash entry.
    ViewStashDiff(usize),
    /// Stash diff loaded.
    StashDiffLoaded(Result<Vec<DiffInfo>, String>),
    /// Apply a stash (like pop but keeps the stash).
    StashApply(usize),

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
    /// Zoom in (increase UI scale).
    ZoomIn,
    /// Zoom out (decrease UI scale).
    ZoomOut,
    /// Reset zoom to 100%.
    ZoomReset,
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

    /// Cherry-pick a commit by OID onto the current branch.
    CherryPickCommit(String),

    /// Begin an inline "create branch at this commit" form.
    BeginCreateBranchAtCommit(String),

    /// User confirmed branch creation at a specific commit.
    ConfirmCreateBranchAtCommit,

    /// User cancelled branch creation at a specific commit.
    CancelCreateBranchAtCommit,

    /// Execute a commit action that requires no further user input.
    /// Actions that need input (branch name, tag name) continue to use the
    /// existing `BeginCreate*` messages.
    ExecuteCommitAction(String, gitkraft_core::CommitAction),

    // ── Commit actions ────────────────────────────────────────────────────────────
    /// Checkout a specific commit in detached HEAD mode.
    CheckoutCommitDetached(String),

    /// Rebase the current branch on top of a specific commit.
    RebaseOntoCommit(String),

    /// Revert a specific commit (creates a revert commit).
    RevertCommit(String),

    /// git reset --soft `oid` — move HEAD, keep staged + working changes.
    ResetSoft(String),
    /// git reset --mixed `oid` — move HEAD and unstage; keep working directory.
    ResetMixed(String),
    /// git reset --hard `oid` — move HEAD and discard all uncommitted changes.
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

    /// User selected a different editor from the picker.
    EditorChanged(gitkraft_core::Editor),
    /// Background `save_editor` completed (fire-and-forget, errors logged).
    EditorSaved(Result<(), String>),
    /// User clicked a recent repository entry on the welcome screen.
    OpenRecentRepo(PathBuf),
    // ── Search ────────────────────────────────────────────────────────────
    /// Toggle the search overlay.
    ToggleSearch,
    /// User typed in the search input.
    SearchQueryChanged(String),
    /// Search results arrived from the background.
    SearchResultsLoaded(Result<Vec<gitkraft_core::CommitInfo>, String>),
    /// User selected a search result (by index).
    SelectSearchResult(usize),
    /// User confirmed the selected search result (Enter).
    ConfirmSearchResult,
    /// File list for commit-vs-workdir diff loaded.
    SearchDiffFilesLoaded(Result<Vec<gitkraft_core::DiffFileEntry>, String>),
    /// User toggled selection of a file in the search diff file list.
    ToggleSearchDiffFile(usize),
    /// User requested to view diff of selected search file against working tree.
    ViewSearchDiffFile(usize),
    /// The diff content for a search file loaded.
    SearchFileDiffLoaded(Result<gitkraft_core::DiffInfo, String>),
    /// User clicked "Select All" / "Deselect All" in search diff.
    ToggleSearchDiffSelectAll,
    /// Diff all selected files against working tree (combined view).
    DiffSelectedFiles,
    /// Combined diffs for selected files loaded.
    SearchMultiDiffLoaded(Result<Vec<gitkraft_core::DiffInfo>, String>),
    /// Go back from file diff view to file list in search.
    SearchDiffBack,
    /// User right-clicked a search result — open commit context menu.
    OpenSearchResultContextMenu(usize),

    /// File system change detected — auto-refresh staging area.
    FileSystemChanged,

    /// Open a file in the configured editor.
    OpenInEditor(String),
    /// Open a file in the system's default program.
    OpenInDefaultProgram(String),
    /// Show a file in the system file manager.
    ShowInFolder(String),

    /// Keyboard modifier state changed (e.g. Shift pressed/released).
    ModifiersChanged(iced::keyboard::Modifiers),
    /// Multiple commit file diffs loaded for a multi-file selection.
    CommitMultiDiffLoaded(Result<Vec<gitkraft_core::DiffInfo>, String>),

    // ── File history overlay ──────────────────────────────────────────────────
    /// Open the file-history overlay for a specific repo-relative path.
    ViewFileHistory(String),
    /// Background load of file-history commits completed.
    FileHistoryLoaded(Result<(String, Vec<gitkraft_core::CommitInfo>), String>),
    /// Close the file-history overlay without taking an action.
    CloseFileHistory,
    /// User scrolled the file-history list.
    FileHistoryScrolled(f32),
    /// User selected a commit in file history — load its diff and close overlay.
    SelectFileHistoryCommit(String),

    // ── Blame overlay ─────────────────────────────────────────────────────────
    /// Open the blame overlay for a specific repo-relative path.
    ViewFileBlame(String),
    /// Background load of blame lines completed.
    FileBlameLoaded(Result<(String, Vec<gitkraft_core::BlameLine>), String>),
    /// Close the blame overlay.
    CloseFileBlame,
    /// User scrolled the blame view.
    BlameScrolled(f32),

    // ── File deletion ─────────────────────────────────────────────────────────
    /// Show delete-confirmation banner for a working-tree file.
    DeleteFile(String),
    /// User confirmed the deletion.
    ConfirmDeleteFile,
    /// User cancelled the deletion.
    CancelDeleteFile,

    /// Diff multiple files from a specific commit against the current working tree.
    DiffMultiWithWorkingTree(String, Vec<String>), // (oid, file_paths)

    /// Restore a single file from a specific commit to the working directory.
    CheckoutFileAtCommit(String, String), // (oid, file_path)
    /// Restore multiple files from a specific commit to the working directory.
    CheckoutMultiFilesAtCommit(String, Vec<String>), // (oid, file_paths)

    /// Cherry-pick a list of commits (by OID) onto the current branch.
    CherryPickCommits(Vec<String>),
    /// Revert a list of commits (by OID) in order.
    RevertCommits(Vec<String>),

    /// Combined range diff across multiple selected commits was loaded.
    CommitRangeDiffLoaded(Result<Vec<gitkraft_core::DiffInfo>, String>),

    /// The application window was resized.
    WindowResized(f32, f32), // (width, height)
    /// The application window was moved.
    WindowMoved(f32, f32), // (x, y)

    /// Open the GUI settings file (settings.json) in the configured editor.
    /// Triggered by Ctrl/Cmd + , (like Zed).
    OpenSettingsFile,

    /// Shift+Down arrow — extends range selection in the file list (if files are
    /// loaded) or in the commit log (fallback).
    ShiftArrowDown,
    /// Shift+Up arrow — same as above but upward.
    ShiftArrowUp,

    /// Fired ~10× per second to advance loading-spinner animation frames.
    /// Only emitted while at least one tab is loading.
    AnimationTick,

    /// No-op (used for disabled buttons, etc.).
    Noop,
}
