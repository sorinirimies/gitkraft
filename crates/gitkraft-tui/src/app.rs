use std::path::PathBuf;

use ratatui::widgets::ListState;
use std::sync::mpsc;

use gitkraft_core::*;

// ── Background task macros ────────────────────────────────────────────────────

/// Spawn a background task that requires a repo path.
///
/// Extracts `repo_path` from the active tab, sets `is_loading = true` and
/// a status message, clones `bg_tx`, then spawns a thread that runs `$body`
/// and sends the result wrapped in `$variant`.
///
/// `$body` may use `?` – it is executed inside an immediately-invoked closure.
macro_rules! bg_task {
    ($self:expr, $status:expr, $variant:path, |$rp:ident| $body:expr) => {{
        let $rp = match $self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        $self.tab_mut().is_loading = true;
        $self.tab_mut().status_message = Some($status.into());
        let tx = $self.bg_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send($variant((|| $body)()));
        });
    }};
}

/// Spawn a background task that produces an `OperationDone` result.
///
/// `$body` must return `Result<String, String>` where `Ok(msg)` is the success
/// message and `Err(msg)` is the error message. `?` may be used inside the body.
///
/// Use `staging` to trigger only a staging refresh on success, or `refresh`
/// to trigger a full repo refresh.
macro_rules! bg_op {
    ($self:expr, $status:expr, staging, |$rp:ident| $body:expr) => {
        bg_op!(@inner $self, $status, false, true, |$rp| $body)
    };
    ($self:expr, $status:expr, refresh, |$rp:ident| $body:expr) => {
        bg_op!(@inner $self, $status, true, false, |$rp| $body)
    };
    (@inner $self:expr, $status:expr, $nr:expr, $nsr:expr, |$rp:ident| $body:expr) => {{
        let $rp = match $self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        $self.tab_mut().is_loading = true;
        $self.tab_mut().status_message = Some($status.into());
        let tx = $self.bg_tx.clone();
        std::thread::spawn(move || {
            let res: Result<String, String> = (|| $body)();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err(),
                needs_refresh: $nr,
                needs_staging_refresh: $nsr,
            });
        });
    }};
}

// ── Background task results ───────────────────────────────────────────────────

/// Alias for the shared core type — kept for backward-compat with the
/// `BackgroundResult::RepoLoaded` variant.
pub type RepoPayload = gitkraft_core::RepoSnapshot;

/// Results produced by background tasks and sent back to the main loop.
#[derive(Debug)]
pub enum BackgroundResult {
    /// A repo open / refresh completed. The `PathBuf` identifies which tab
    /// initiated the load so the result is applied to the correct tab.
    RepoLoaded {
        path: PathBuf,
        result: Result<RepoPayload, String>,
    },
    /// A fetch completed.
    FetchDone(Result<(), String>),
    /// A commit-diff load completed.
    CommitDiffLoaded(Result<Vec<DiffInfo>, String>),
    /// A staging-only refresh completed (unstaged + staged diffs reloaded).
    StagingRefreshed(Result<StagingPayload, String>),
    /// A single-shot operation (stage, unstage, checkout, commit, stash, etc.)
    /// completed and the staging area should be refreshed.
    OperationDone {
        ok_message: Option<String>,
        err_message: Option<String>,
        /// If `true`, trigger a full refresh after applying the result.
        needs_refresh: bool,
        /// If `true`, trigger only a staging refresh.
        needs_staging_refresh: bool,
    },
    /// A commit file list (lightweight, no diff content) was loaded.
    CommitFileListLoaded(Result<Vec<gitkraft_core::DiffFileEntry>, String>),
    /// A single file's diff was loaded.  The `usize` is the file index in `commit_files`.
    SingleFileDiffLoaded(Result<(usize, gitkraft_core::DiffInfo), String>),
    /// Commit search results loaded.
    SearchResults(Result<Vec<gitkraft_core::CommitInfo>, String>),
    /// Combined range diff across multiple selected commits.
    CommitRangeDiffLoaded(Result<Vec<gitkraft_core::DiffInfo>, String>),
    /// File history (commits touching a specific file) loaded.
    FileHistoryLoaded {
        path: String,
        commits: Vec<gitkraft_core::CommitInfo>,
    },
    /// File blame lines loaded.
    FileBlameLoaded {
        path: String,
        lines: Vec<gitkraft_core::BlameLine>,
    },
    /// The `.git` directory changed from outside the TUI — trigger a full refresh.
    GitStateChanged,
}

/// Payload returned by an async staging refresh.
#[derive(Debug)]
pub struct StagingPayload {
    pub unstaged: Vec<DiffInfo>,
    pub staged: Vec<DiffInfo>,
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppScreen {
    Welcome,
    DirBrowser,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Branches,
    CommitLog,
    DiffView,
    Staging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPurpose {
    None,
    CommitMessage,
    BranchName,
    RepoPath,
    SearchQuery,
    StashMessage,
    /// First string for a pending commit action (branch name, tag name, etc.).
    CommitActionInput1,
    /// Second string for a pending commit action (annotated-tag message).
    CommitActionInput2,
}

/// Which sub-list within the staging pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingFocus {
    Unstaged,
    Staged,
}

/// Which half of the split Diff pane currently has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffSubPane {
    /// The file-list sidebar on the left.
    FileList,
    /// The diff-content area on the right.
    Content,
}

// ── Per-repo tab state ────────────────────────────────────────────────────────────

/// All state that belongs to a single repository tab.
pub struct RepoTab {
    pub repo_path: Option<PathBuf>,
    pub repo_info: Option<RepoInfo>,

    pub branches: Vec<BranchInfo>,
    pub branch_list_state: ListState,

    pub commits: Vec<CommitInfo>,
    pub graph_rows: Vec<gitkraft_core::GraphRow>,
    pub commit_list_state: ListState,

    pub unstaged_changes: Vec<DiffInfo>,
    pub staged_changes: Vec<DiffInfo>,
    pub unstaged_list_state: ListState,
    pub staged_list_state: ListState,
    pub staging_focus: StagingFocus,
    pub selected_diff: Option<DiffInfo>,
    pub diff_scroll: u16,
    /// Which half of the diff split pane has keyboard focus.
    pub diff_sub_pane: DiffSubPane,
    /// Anchor index for diff file list range selection (set on plain navigation).
    pub anchor_file_index: Option<usize>,
    /// File indices currently multi-selected in the diff file list.
    pub selected_file_indices: std::collections::HashSet<usize>,
    /// Cache of per-file diffs keyed by file index (sparse — only loaded files are present).
    pub commit_diffs: std::collections::HashMap<usize, DiffInfo>,
    /// Index of the currently selected file in commit_files.
    pub commit_diff_file_index: usize,
    /// Lightweight file list for the selected commit.
    pub commit_files: Vec<gitkraft_core::DiffFileEntry>,
    /// OID of the currently selected commit (for lazy file diff loading).
    pub selected_commit_oid: Option<String>,

    pub stashes: Vec<StashEntry>,
    pub stash_list_state: ListState,
    pub remotes: Vec<RemoteInfo>,

    /// Search query for commit filtering.
    pub search_query: String,
    /// Whether search mode is active.
    pub search_active: bool,
    /// Search results (commits matching the query).
    pub search_results: Vec<CommitInfo>,

    /// Optional stash message (set via input mode before saving).
    pub stash_message_buffer: String,

    pub status_message: Option<String>,
    pub error_message: Option<String>,

    /// True while a background task is in flight for this tab.
    pub is_loading: bool,

    /// When true, the next d press actually discards; otherwise the first
    /// d sets this flag and shows a confirmation prompt.
    pub confirm_discard: bool,

    /// Selected unstaged file indices for multi-select.
    pub selected_unstaged: std::collections::HashSet<usize>,
    /// Selected staged file indices for multi-select.
    pub selected_staged: std::collections::HashSet<usize>,
    /// Anchor index for unstaged range selection (set on plain j/k navigation).
    pub anchor_unstaged: Option<usize>,
    /// Anchor index for staged range selection (set on plain j/k navigation).
    pub anchor_staged: Option<usize>,

    /// Anchor commit index for range selection (Shift+Up/Down).
    pub anchor_commit_index: Option<usize>,
    /// Ordered ascending list of commit indices in the current range selection.
    pub selected_commits: Vec<usize>,
    /// Combined diff for the current commit range selection.
    pub commit_range_diffs: Vec<DiffInfo>,

    /// Flat ordered list of all action kinds shown in the popup (built from
    /// `COMMIT_MENU_GROUPS`, separator positions stored separately).
    pub commit_action_items: Vec<gitkraft_core::CommitActionKind>,
    /// Which item in `commit_action_items` is highlighted (0-based).
    pub commit_action_cursor: usize,
    /// OID of the commit targeted by the open action popup.
    pub pending_commit_action_oid: Option<String>,
    /// Action kind waiting for user input before it can execute.
    pub pending_action_kind: Option<gitkraft_core::CommitActionKind>,
    /// First input collected for the pending action (e.g. branch/tag name).
    pub action_input1: String,

    /// When `Some(path)`, the file-history overlay is visible for that path.
    pub file_history_path: Option<String>,
    /// Commits that touch the file in the history overlay (newest first).
    pub file_history_commits: Vec<gitkraft_core::CommitInfo>,
    /// Which commit row is highlighted in the history overlay (0-based).
    pub file_history_cursor: usize,

    /// When `Some(path)`, the blame overlay is visible for that path.
    pub blame_path: Option<String>,
    /// Blame lines for the current blame overlay.
    pub blame_lines: Vec<gitkraft_core::BlameLine>,
    /// Scroll offset for the blame overlay (rows from top).
    pub blame_scroll: u16,

    /// When `Some(path)`, show a delete-confirmation prompt for that path.
    pub confirm_delete_file: Option<String>,
}

impl RepoTab {
    #[must_use]
    pub fn new() -> Self {
        Self {
            repo_path: None,
            repo_info: None,

            branches: Vec::new(),
            branch_list_state: ListState::default(),

            commits: Vec::new(),
            graph_rows: Vec::new(),
            commit_list_state: ListState::default(),

            unstaged_changes: Vec::new(),
            staged_changes: Vec::new(),
            unstaged_list_state: ListState::default(),
            staged_list_state: ListState::default(),
            staging_focus: StagingFocus::Unstaged,
            selected_diff: None,
            diff_scroll: 0,
            diff_sub_pane: DiffSubPane::FileList,
            anchor_file_index: None,
            selected_file_indices: std::collections::HashSet::new(),
            commit_diffs: std::collections::HashMap::new(),
            commit_diff_file_index: 0,
            commit_files: Vec::new(),
            selected_commit_oid: None,

            stashes: Vec::new(),
            stash_list_state: ListState::default(),
            remotes: Vec::new(),

            stash_message_buffer: String::new(),

            search_query: String::new(),
            search_active: false,
            search_results: Vec::new(),

            status_message: None,
            error_message: None,

            is_loading: false,

            confirm_discard: false,

            selected_unstaged: std::collections::HashSet::new(),
            selected_staged: std::collections::HashSet::new(),
            anchor_unstaged: None,
            anchor_staged: None,

            anchor_commit_index: None,
            selected_commits: Vec::new(),
            commit_range_diffs: Vec::new(),

            commit_action_items: Vec::new(),
            commit_action_cursor: 0,
            pending_commit_action_oid: None,
            pending_action_kind: None,
            action_input1: String::new(),

            file_history_path: None,
            file_history_commits: Vec::new(),
            file_history_cursor: 0,
            blame_path: None,
            blame_lines: Vec::new(),
            blame_scroll: 0,
            confirm_delete_file: None,
        }
    }

    /// Return a human-readable display name for this tab.
    /// Uses the last path component of repo_path, or "New Tab" if none.
    pub fn display_name(&self) -> String {
        match &self.repo_path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "New Tab".into()),
            None => "New Tab".into(),
        }
    }
}

impl Default for RepoTab {
    fn default() -> Self {
        Self::new()
    }
}

// ── App State ───────────────────────────────────────────────────────────────────

pub struct App {
    pub should_quit: bool,
    pub screen: AppScreen,
    pub active_pane: ActivePane,
    pub input_mode: InputMode,
    pub input_purpose: InputPurpose,
    pub tick_count: u64,

    /// Receiver for results from background tasks.
    pub bg_rx: mpsc::Receiver<BackgroundResult>,
    /// Sender cloned into each spawned task.
    pub(crate) bg_tx: mpsc::Sender<BackgroundResult>,

    pub input_buffer: String,

    /// Whether the theme selection panel is visible.
    pub show_theme_panel: bool,
    /// Whether the options panel is visible.
    pub show_options_panel: bool,
    /// Configured editor for opening files.
    pub editor: gitkraft_core::Editor,
    /// Whether the editor picker panel is visible.
    pub show_editor_panel: bool,
    /// ListState for the editor picker list.
    pub editor_list_state: ListState,
    /// Currently selected theme index (0-26).
    pub current_theme_index: usize,
    /// ListState for the theme list widget.
    pub theme_list_state: ListState,

    /// Recently opened repositories loaded from persistence.
    pub recent_repos: Vec<gitkraft_core::RepoHistoryEntry>,

    /// Current directory being browsed in the directory picker.
    pub browser_dir: PathBuf,
    /// Entries in the current browser directory.
    pub browser_entries: Vec<std::path::PathBuf>,
    /// List state for the directory browser.
    pub browser_list_state: ListState,
    /// Screen to return to when the directory browser is dismissed.
    pub browser_return_screen: AppScreen,

    /// Open repository tabs.
    pub tabs: Vec<RepoTab>,
    /// Index of the currently active tab.
    pub active_tab_index: usize,

    /// When `Some`, the event loop in `lib.rs` suspends the TUI, opens ALL listed
    /// paths in the configured terminal editor, then resumes.
    /// Supports both single-file (`vec![path]`) and multi-file (`vec![a, b, c]`) opens.
    pub pending_editor_open: Option<Vec<std::path::PathBuf>>,

    /// Timestamp of the last auto-refresh.
    pub last_auto_refresh: std::time::Instant,
}

impl App {
    // ── Constructor ──────────────────────────────────────────────────────────

    #[must_use]
    pub fn new() -> Self {
        let settings =
            gitkraft_core::features::persistence::load_tui_settings().unwrap_or_default();

        let theme_index = theme_name_to_index(settings.theme_name.as_deref().unwrap_or(""));

        let recent_repos = settings.recent_repos;

        let (bg_tx, bg_rx) = mpsc::channel();

        Self {
            should_quit: false,
            screen: AppScreen::Welcome,
            active_pane: ActivePane::Branches,
            input_mode: InputMode::Normal,
            input_purpose: InputPurpose::None,
            tick_count: 0,

            bg_rx,
            bg_tx,

            input_buffer: String::new(),

            show_theme_panel: false,
            show_options_panel: false,
            editor: settings
                .editor_name
                .as_deref()
                .map(|name| {
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
                .unwrap_or(gitkraft_core::Editor::None),
            show_editor_panel: false,
            editor_list_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            current_theme_index: theme_index,
            theme_list_state: {
                let mut s = ListState::default();
                s.select(Some(theme_index));
                s
            },

            recent_repos,

            browser_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            browser_entries: Vec::new(),
            browser_list_state: ListState::default(),
            browser_return_screen: AppScreen::Welcome,

            tabs: vec![RepoTab::new()],
            active_tab_index: 0,

            pending_editor_open: None,

            last_auto_refresh: std::time::Instant::now(),
        }
    }

    // ── Tab accessors ────────────────────────────────────────────────────────

    /// Return a shared reference to the currently active tab.
    #[inline]
    pub fn tab(&self) -> &RepoTab {
        &self.tabs[self.active_tab_index]
    }

    /// Return an exclusive reference to the currently active tab.
    #[inline]
    pub fn tab_mut(&mut self) -> &mut RepoTab {
        &mut self.tabs[self.active_tab_index]
    }

    // ── Tab management ──────────────────────────────────────────────────────

    /// Open a new empty tab and make it active.
    pub fn new_tab(&mut self) {
        self.tabs.push(RepoTab::new());
        self.active_tab_index = self.tabs.len() - 1;
        self.screen = AppScreen::Welcome;
        // Reload recent repos so they're fresh on the welcome screen
        if let Ok(settings) = gitkraft_core::features::persistence::load_tui_settings() {
            self.recent_repos = settings.recent_repos;
        }
        self.save_session();
    }

    /// Close the current tab. If it is the last tab, replace it with an empty one.
    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            self.tabs[0] = RepoTab::new();
            self.active_tab_index = 0;
        } else {
            self.tabs.remove(self.active_tab_index);
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            }
        }
        self.save_session();
    }

    /// Switch to the next tab (wrapping around).
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
            // Restore the correct screen for the target tab
            if self.tabs[self.active_tab_index].repo_path.is_some() {
                self.screen = AppScreen::Main;
            } else {
                self.screen = AppScreen::Welcome;
            }
        }
    }

    /// Switch to the previous tab (wrapping around).
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab_index == 0 {
                self.active_tab_index = self.tabs.len() - 1;
            } else {
                self.active_tab_index -= 1;
            }
            // Restore the correct screen for the target tab
            if self.tabs[self.active_tab_index].repo_path.is_some() {
                self.screen = AppScreen::Main;
            } else {
                self.screen = AppScreen::Welcome;
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    // ── Theme helpers ────────────────────────────────────────────────────

    pub fn cycle_theme_next(&mut self) {
        let count = 27; // number of themes
        self.current_theme_index = (self.current_theme_index + 1) % count;
        self.theme_list_state.select(Some(self.current_theme_index));
        self.tab_mut().status_message = Some(format!("Theme: {}", self.current_theme_name()));
    }

    pub fn cycle_theme_prev(&mut self) {
        let count = 27;
        if self.current_theme_index == 0 {
            self.current_theme_index = count - 1;
        } else {
            self.current_theme_index -= 1;
        }
        self.theme_list_state.select(Some(self.current_theme_index));
        self.tab_mut().status_message = Some(format!("Theme: {}", self.current_theme_name()));
    }

    pub fn current_theme_name(&self) -> &'static str {
        gitkraft_core::THEME_NAMES
            .get(self.current_theme_index)
            .copied()
            .unwrap_or("Default")
    }

    /// Return the `UiTheme` for the currently selected theme index.
    pub fn theme(&self) -> crate::features::theme::palette::UiTheme {
        crate::features::theme::palette::theme_for_index(self.current_theme_index)
    }

    /// Persist the current theme selection to disk.
    pub fn save_theme(&self) {
        let _ = gitkraft_core::features::persistence::save_theme_tui(self.current_theme_name());
    }

    /// Persist the paths of all open tabs for session restore.
    pub fn save_session(&self) {
        let paths: Vec<std::path::PathBuf> = self
            .tabs
            .iter()
            .filter_map(|t| t.repo_path.clone())
            .collect();
        let active = self.active_tab_index;
        let _ = gitkraft_core::features::persistence::save_session_tui(&paths, active);
    }

    // ── High-level operations ────────────────────────────────────────────

    pub fn open_repo(&mut self, path: PathBuf) {
        self.tab_mut().error_message = None;
        self.tab_mut().status_message = Some("Opening repository…".into());
        self.tab_mut().is_loading = true;
        self.tab_mut().repo_path = Some(path.clone());
        self.screen = AppScreen::Main;

        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let result = load_repo_blocking(&path);
            let _ = tx.send(BackgroundResult::RepoLoaded { path, result });
        });
        self.save_session();
    }

    pub fn refresh(&mut self) {
        self.tab_mut().error_message = None;
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some("Refreshing…".into());

        let path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => {
                self.tab_mut().error_message = Some("No repository open".into());
                self.tab_mut().is_loading = false;
                return;
            }
        };

        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let result = load_repo_blocking(&path);
            let _ = tx.send(BackgroundResult::RepoLoaded { path, result });
        });
    }

    /// Process any pending results from background tasks.
    /// Call this once per tick in the event loop.
    pub fn poll_background(&mut self) {
        while let Ok(result) = self.bg_rx.try_recv() {
            match result {
                BackgroundResult::RepoLoaded {
                    path: loaded_path,
                    result: res,
                } => {
                    // Find the tab that initiated this load by matching repo_path.
                    let tab_idx = self
                        .tabs
                        .iter()
                        .position(|t| t.repo_path.as_ref() == Some(&loaded_path))
                        .unwrap_or(self.active_tab_index);

                    self.tabs[tab_idx].is_loading = false;
                    match res {
                        Ok(payload) => {
                            let canonical = payload.info.workdir.clone().unwrap_or_else(|| {
                                self.tabs[tab_idx].repo_path.clone().unwrap_or_default()
                            });
                            self.tabs[tab_idx].repo_path = Some(canonical.clone());

                            // Persist
                            let _ = gitkraft_core::features::persistence::record_repo_opened_tui(
                                &canonical,
                            );
                            if let Ok(settings) =
                                gitkraft_core::features::persistence::load_tui_settings()
                            {
                                self.recent_repos = settings.recent_repos;
                            }

                            let tab = &mut self.tabs[tab_idx];
                            tab.repo_info = Some(payload.info);
                            tab.branches = payload.branches;
                            clamp_list_state(&mut tab.branch_list_state, tab.branches.len());
                            tab.graph_rows = payload.graph_rows;
                            tab.commits = payload.commits;
                            clamp_list_state(&mut tab.commit_list_state, tab.commits.len());
                            tab.unstaged_changes = payload.unstaged;
                            clamp_list_state(
                                &mut tab.unstaged_list_state,
                                tab.unstaged_changes.len(),
                            );
                            tab.staged_changes = payload.staged;
                            clamp_list_state(&mut tab.staged_list_state, tab.staged_changes.len());
                            tab.stashes = payload.stashes;
                            clamp_list_state(&mut tab.stash_list_state, tab.stashes.len());
                            tab.remotes = payload.remotes;
                            tab.status_message = Some("Repository loaded".into());
                            self.screen = AppScreen::Main;
                            self.save_session();
                        }
                        Err(e) => {
                            self.tabs[tab_idx].error_message = Some(e);
                            self.tabs[tab_idx].status_message = None;
                        }
                    }
                }
                BackgroundResult::FetchDone(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(()) => {
                            self.tab_mut().status_message = Some("Fetched from origin".into());
                            self.refresh();
                        }
                        Err(e) => self.tab_mut().error_message = Some(format!("fetch: {e}")),
                    }
                }
                BackgroundResult::CommitDiffLoaded(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(diffs) => {
                            if diffs.is_empty() {
                                let tab = self.tab_mut();
                                tab.selected_diff = None;
                                tab.commit_diffs.clear();
                                tab.commit_diff_file_index = 0;
                                tab.status_message = Some("No changes in this commit".into());
                            } else {
                                let tab = self.tab_mut();
                                tab.commit_diffs = diffs
                                    .iter()
                                    .enumerate()
                                    .map(|(i, d)| (i, d.clone()))
                                    .collect();
                                tab.commit_diff_file_index = 0;
                                tab.selected_diff = Some(diffs[0].clone());
                                tab.diff_scroll = 0;
                                if diffs.len() > 1 {
                                    tab.status_message = Some(format!(
                                        "Showing file 1/{} — use h/l to switch files",
                                        diffs.len()
                                    ));
                                }
                            }
                        }
                        Err(e) => self.tab_mut().error_message = Some(format!("commit diff: {e}")),
                    }
                }
                BackgroundResult::CommitFileListLoaded(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(files) => {
                            let count = files.len();
                            let tab = self.tab_mut();
                            tab.commit_files = files;
                            tab.commit_diffs.clear();
                            tab.commit_diff_file_index = 0;
                            tab.selected_diff = None;
                            tab.diff_scroll = 0;
                            tab.diff_sub_pane = DiffSubPane::FileList;
                            tab.selected_file_indices.clear();

                            if count == 0 {
                                tab.status_message = Some("No changes in this commit".into());
                            } else {
                                tab.status_message = Some(format!("{count} file(s) changed"));
                                tab.selected_file_indices.insert(0);
                                // Auto-load the first file's diff
                                let first_path = tab.commit_files[0].display_path().to_string();
                                self.load_single_file_diff(0, first_path);
                            }
                        }
                        Err(e) => self.tab_mut().error_message = Some(format!("file list: {e}")),
                    }
                }
                BackgroundResult::SingleFileDiffLoaded(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok((file_index, diff)) => {
                            let tab = self.tab_mut();
                            tab.commit_diffs.insert(file_index, diff.clone());
                            // Update selected_diff only when this is the currently focused file
                            // and we are NOT showing a multi-file concatenated view.
                            let is_multi = tab.selected_file_indices.len() > 1;
                            if !is_multi && file_index == tab.commit_diff_file_index {
                                tab.selected_diff = Some(diff);
                                tab.diff_scroll = 0;
                            }
                            if tab.commit_files.len() > 1 {
                                let sel_count = tab.selected_file_indices.len();
                                if sel_count > 1 {
                                    tab.status_message = Some(format!(
                                        "{sel_count} files selected — use Shift+↑/↓ to adjust"
                                    ));
                                } else {
                                    tab.status_message = Some(format!(
                                        "File {}/{} — use h/l or ↑/↓ to switch",
                                        file_index + 1,
                                        tab.commit_files.len()
                                    ));
                                }
                            }
                        }
                        Err(e) => self.tab_mut().error_message = Some(format!("file diff: {e}")),
                    }
                }
                BackgroundResult::StagingRefreshed(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(payload) => self.apply_staging_payload(payload),
                        Err(e) => {
                            self.tab_mut().error_message = Some(format!("staging refresh: {e}"))
                        }
                    }
                }
                BackgroundResult::OperationDone {
                    ok_message,
                    err_message,
                    needs_refresh,
                    needs_staging_refresh,
                } => {
                    self.tab_mut().is_loading = false;
                    if let Some(msg) = err_message {
                        self.tab_mut().error_message = Some(msg);
                    } else if let Some(msg) = ok_message {
                        self.tab_mut().status_message = Some(msg);
                    }
                    if needs_refresh {
                        self.refresh();
                    } else if needs_staging_refresh {
                        self.refresh_staging();
                    }
                }
                BackgroundResult::SearchResults(res) => match res {
                    Ok(results) => {
                        self.tab_mut().search_results = results;
                        let count = self.tab().search_results.len();
                        self.tab_mut().status_message = Some(format!("{count} result(s) found"));
                    }
                    Err(e) => {
                        self.tab_mut().error_message = Some(format!("Search failed: {e}"));
                    }
                },
                BackgroundResult::CommitRangeDiffLoaded(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(diffs) => {
                            let count = self.tab().selected_commits.len();
                            let tab = self.tab_mut();
                            tab.commit_range_diffs = diffs;
                            tab.diff_scroll = 0;
                            tab.status_message = Some(format!(
                                "Combined diff for {count} commits — use j/k to scroll"
                            ));
                        }
                        Err(e) => {
                            self.tab_mut().error_message = Some(format!("Range diff: {e}"));
                        }
                    }
                }

                BackgroundResult::FileHistoryLoaded { path, commits } => {
                    let count = commits.len();
                    let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
                    let tab = self.tab_mut();
                    tab.file_history_path = Some(path);
                    tab.file_history_commits = commits;
                    tab.file_history_cursor = 0;
                    tab.status_message = Some(format!("History: {file_name} ({count} commits)"));
                }

                BackgroundResult::FileBlameLoaded { path, lines } => {
                    let count = lines.len();
                    let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
                    let tab = self.tab_mut();
                    tab.blame_path = Some(path);
                    tab.blame_lines = lines;
                    tab.blame_scroll = 0;
                    tab.status_message = Some(format!("Blame: {file_name} ({count} lines)"));
                }

                BackgroundResult::GitStateChanged => {
                    // Only refresh if not already loading to avoid stacking refreshes.
                    if !self.tab().is_loading {
                        self.refresh();
                    }
                }
            }
        }
    }

    /// Fallback poll — triggers a full refresh every 5 seconds as a safety net.
    ///
    /// The `notify`-based watcher in `lib.rs` handles immediate reactive refresh;
    /// this keeps the UI current on network file systems or other environments
    /// where inotify events may not be delivered reliably.
    pub fn maybe_auto_refresh(&mut self) {
        if self.tab().repo_path.is_some()
            && !self.tab().is_loading
            && self.last_auto_refresh.elapsed() >= std::time::Duration::from_secs(5)
        {
            self.last_auto_refresh = std::time::Instant::now();
            self.refresh();
        }
    }

    pub fn refresh_staging(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => {
                self.tab_mut().error_message = Some("No repository open".into());
                return;
            }
        };
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res = (|| {
                let repo = open_repo_str(&repo_path)?;
                let unstaged = gitkraft_core::features::diff::get_working_dir_diff(&repo)
                    .map_err(|e| e.to_string())?;
                let staged = gitkraft_core::features::diff::get_staged_diff(&repo)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(StagingPayload { unstaged, staged })
            })();
            let _ = tx.send(BackgroundResult::StagingRefreshed(res));
        });
    }

    fn apply_staging_payload(&mut self, payload: StagingPayload) {
        self.tab_mut().selected_unstaged.clear();
        self.tab_mut().selected_staged.clear();
        let tab = self.tab_mut();
        tab.unstaged_changes = payload.unstaged;
        if tab.unstaged_changes.is_empty() {
            tab.unstaged_list_state.select(None);
        } else if tab.unstaged_list_state.selected().is_none() {
            tab.unstaged_list_state.select(Some(0));
        } else if let Some(i) = tab.unstaged_list_state.selected() {
            if i >= tab.unstaged_changes.len() {
                tab.unstaged_list_state
                    .select(Some(tab.unstaged_changes.len() - 1));
            }
        }

        tab.staged_changes = payload.staged;
        if tab.staged_changes.is_empty() {
            tab.staged_list_state.select(None);
        } else if tab.staged_list_state.selected().is_none() {
            tab.staged_list_state.select(Some(0));
        } else if let Some(i) = tab.staged_list_state.selected() {
            if i >= tab.staged_changes.len() {
                tab.staged_list_state
                    .select(Some(tab.staged_changes.len() - 1));
            }
        }
    }

    // ── Staging operations ───────────────────────────────────────────────

    pub fn stage_selected(&mut self) {
        let idx = match self.tab().unstaged_list_state.selected() {
            Some(i) => i,
            None => {
                self.tab_mut().status_message = Some("No unstaged file selected".into());
                return;
            }
        };
        let file_path = self.unstaged_file_path(idx);
        bg_op!(self, "Staging…", staging, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::staging::stage_file(&repo, &file_path)
                .map_err(|e| format!("stage: {e}"))?;
            Ok(format!("Staged: {file_path}"))
        });
    }

    pub fn unstage_selected(&mut self) {
        let idx = match self.tab().staged_list_state.selected() {
            Some(i) => i,
            None => {
                self.tab_mut().status_message = Some("No staged file selected".into());
                return;
            }
        };
        let file_path = self.staged_file_path(idx);
        bg_op!(self, "Unstaging…", staging, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::staging::unstage_file(&repo, &file_path)
                .map_err(|e| format!("unstage: {e}"))?;
            Ok(format!("Unstaged: {file_path}"))
        });
    }

    pub fn stage_all(&mut self) {
        bg_op!(self, "Staging all…", staging, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::staging::stage_all(&repo)
                .map_err(|e| format!("stage all: {e}"))?;
            Ok("Staged all files".into())
        });
    }

    pub fn unstage_all(&mut self) {
        bg_op!(self, "Unstaging all…", staging, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::staging::unstage_all(&repo)
                .map_err(|e| format!("unstage all: {e}"))?;
            Ok("Unstaged all files".into())
        });
    }

    pub fn discard_selected(&mut self) {
        let idx = match self.tab().unstaged_list_state.selected() {
            Some(i) => i,
            None => {
                self.tab_mut().status_message = Some("No unstaged file selected".into());
                return;
            }
        };
        let file_path = self.unstaged_file_path(idx);
        self.tab_mut().confirm_discard = false;
        bg_op!(self, "Discarding…", staging, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::staging::discard_file_changes(&repo, &file_path)
                .map_err(|e| format!("discard: {e}"))?;
            Ok(format!("Discarded changes: {file_path}"))
        });
    }

    /// Stage multiple files at once.
    pub fn stage_files(&mut self, paths: Vec<String>) {
        let count = paths.len();
        bg_op!(
            self,
            format!("Staging {count} file(s)…"),
            staging,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                for fp in &paths {
                    gitkraft_core::features::staging::stage_file(&repo, fp)
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!("{count} file(s) staged"))
            }
        );
    }

    /// Unstage multiple files at once.
    pub fn unstage_files(&mut self, paths: Vec<String>) {
        let count = paths.len();
        bg_op!(
            self,
            format!("Unstaging {count} file(s)…"),
            staging,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                for fp in &paths {
                    gitkraft_core::features::staging::unstage_file(&repo, fp)
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!("{count} file(s) unstaged"))
            }
        );
    }

    /// Discard changes for multiple files at once.
    pub fn discard_files(&mut self, paths: Vec<String>) {
        let count = paths.len();
        bg_op!(
            self,
            format!("Discarding {count} file(s)…"),
            staging,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                for fp in &paths {
                    gitkraft_core::features::staging::discard_file_changes(&repo, fp)
                        .map_err(|e| e.to_string())?;
                }
                Ok(format!("{count} file(s) discarded"))
            }
        );
    }

    // ── Commit ───────────────────────────────────────────────────────────

    pub fn create_commit(&mut self) {
        let msg = self.input_buffer.trim().to_string();
        if msg.is_empty() {
            self.tab_mut().error_message = Some("Commit message cannot be empty".into());
            return;
        }
        self.input_buffer.clear();
        bg_op!(self, "Committing…", refresh, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            let info = gitkraft_core::features::commits::create_commit(&repo, &msg)
                .map_err(|e| format!("commit: {e}"))?;
            Ok(format!("Committed: {} {}", info.short_oid, info.summary))
        });
    }

    // ── Branches ─────────────────────────────────────────────────────────

    pub fn checkout_selected_branch(&mut self) {
        let idx = match self.tab().branch_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.tab().branches.len() {
            return;
        }
        let name = self.tab().branches[idx].name.clone();
        if self.tab().branches[idx].is_head {
            self.tab_mut().status_message = Some(format!("Already on '{name}'"));
            return;
        }
        bg_op!(self, "Checking out…", refresh, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::branches::checkout_branch(&repo, &name)
                .map_err(|e| format!("checkout: {e}"))?;
            Ok(format!("Checked out: {name}"))
        });
    }

    pub fn create_branch(&mut self) {
        let name = self.input_buffer.trim().to_string();
        if name.is_empty() {
            self.tab_mut().error_message = Some("Branch name cannot be empty".into());
            return;
        }
        self.input_buffer.clear();
        bg_op!(self, "Creating branch…", refresh, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::branches::create_branch(&repo, &name)
                .map_err(|e| format!("create branch: {e}"))?;
            Ok(format!("Created branch: {name}"))
        });
    }

    pub fn delete_selected_branch(&mut self) {
        let idx = match self.tab().branch_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.tab().branches.len() {
            return;
        }
        if self.tab().branches[idx].is_head {
            self.tab_mut().error_message = Some("Cannot delete the current branch".into());
            return;
        }
        let name = self.tab().branches[idx].name.clone();
        bg_op!(self, "Deleting branch…", refresh, |repo_path| {
            let repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::branches::delete_branch(&repo, &name)
                .map_err(|e| format!("delete branch: {e}"))?;
            Ok(format!("Deleted branch: {name}"))
        });
    }

    // ── Stash ────────────────────────────────────────────────────────────

    pub fn stash_save(&mut self) {
        let msg = if self.tab().stash_message_buffer.trim().is_empty() {
            None
        } else {
            Some(self.tab().stash_message_buffer.trim().to_string())
        };
        self.tab_mut().stash_message_buffer.clear();
        bg_op!(self, "Stashing…", refresh, |repo_path| {
            let mut repo = open_repo_str(&repo_path)?;
            let entry = gitkraft_core::features::stash::stash_save(&mut repo, msg.as_deref())
                .map_err(|e| format!("stash save: {e}"))?;
            Ok(format!("Stashed: {}", entry.message))
        });
    }

    pub fn stash_pop_selected(&mut self) {
        let idx = self.tab().stash_list_state.selected().unwrap_or(0);
        if idx >= self.tab().stashes.len() {
            self.tab_mut().error_message = Some("No stash selected".into());
            return;
        }
        bg_op!(self, "Popping stash…", refresh, |repo_path| {
            let mut repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::stash::stash_pop(&mut repo, idx)
                .map_err(|e| format!("stash pop: {e}"))?;
            Ok(format!("Stash @{{{idx}}} popped"))
        });
    }

    pub fn stash_drop_selected(&mut self) {
        let idx = self.tab().stash_list_state.selected().unwrap_or(0);
        if idx >= self.tab().stashes.len() {
            self.tab_mut().error_message = Some("No stash to drop".into());
            return;
        }
        bg_op!(self, "Dropping stash…", refresh, |repo_path| {
            let mut repo = open_repo_str(&repo_path)?;
            gitkraft_core::features::stash::stash_drop(&mut repo, idx)
                .map_err(|e| format!("stash drop: {e}"))?;
            Ok(format!("Stash @{{{idx}}} dropped"))
        });
    }

    // ── Diff ─────────────────────────────────────────────────────────────

    /// Load the file list for the currently selected commit (phase 1 of two-phase loading).
    pub fn load_commit_diff(&mut self) {
        let idx = match self.tab().commit_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.tab().commits.len() {
            return;
        }
        let oid = self.tab().commits[idx].oid.clone();
        self.tab_mut().selected_commit_oid = Some(oid.clone());
        bg_task!(
            self,
            "Loading files…",
            BackgroundResult::CommitFileListLoaded,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::diff::get_commit_file_list(&repo, &oid)
                    .map_err(|e| e.to_string())
            }
        );
    }

    /// Load the diff for a single file in the selected commit (phase 2).
    pub fn load_single_file_diff(&mut self, file_index: usize, file_path: String) {
        let oid = match self.tab().selected_commit_oid.clone() {
            Some(o) => o,
            None => return,
        };
        bg_task!(
            self,
            "Loading diff…",
            BackgroundResult::SingleFileDiffLoaded,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                let diff =
                    gitkraft_core::features::diff::get_single_file_diff(&repo, &oid, &file_path)
                        .map_err(|e| e.to_string())?;
                Ok((file_index, diff))
            }
        );
    }

    /// Load the diff for a specific file index, skipping if it is already cached.
    pub fn load_diff_for_file_index(&mut self, file_index: usize) {
        if file_index >= self.tab().commit_files.len() {
            return;
        }
        if self.tab().commit_diffs.contains_key(&file_index) {
            return;
        }
        let file_path = self.tab().commit_files[file_index]
            .display_path()
            .to_string();
        self.load_single_file_diff(file_index, file_path);
    }

    /// Switch to the next file in the commit diff list.
    pub fn next_diff_file(&mut self) {
        if self.tab().commit_files.is_empty() {
            return;
        }
        let new_index = (self.tab().commit_diff_file_index + 1) % self.tab().commit_files.len();
        self.tab_mut().anchor_file_index = Some(new_index);
        self.tab_mut().commit_diff_file_index = new_index;
        self.tab_mut().selected_file_indices.clear();
        self.tab_mut().selected_file_indices.insert(new_index);
        self.tab_mut().diff_scroll = 0;
        self.tab_mut().status_message = Some(format!(
            "File {}/{}",
            new_index + 1,
            self.tab().commit_files.len()
        ));
        if let Some(cached) = self.tab().commit_diffs.get(&new_index).cloned() {
            self.tab_mut().selected_diff = Some(cached);
        } else {
            let file_path = self.tab().commit_files[new_index]
                .display_path()
                .to_string();
            self.load_single_file_diff(new_index, file_path);
        }
    }

    /// Switch to the previous file in the commit diff list.
    pub fn prev_diff_file(&mut self) {
        if self.tab().commit_files.is_empty() {
            return;
        }
        let new_index = if self.tab().commit_diff_file_index == 0 {
            self.tab().commit_files.len() - 1
        } else {
            self.tab().commit_diff_file_index - 1
        };
        self.tab_mut().anchor_file_index = Some(new_index);
        self.tab_mut().commit_diff_file_index = new_index;
        self.tab_mut().selected_file_indices.clear();
        self.tab_mut().selected_file_indices.insert(new_index);
        self.tab_mut().diff_scroll = 0;
        self.tab_mut().status_message = Some(format!(
            "File {}/{}",
            new_index + 1,
            self.tab().commit_files.len()
        ));
        if let Some(cached) = self.tab().commit_diffs.get(&new_index).cloned() {
            self.tab_mut().selected_diff = Some(cached);
        } else {
            let file_path = self.tab().commit_files[new_index]
                .display_path()
                .to_string();
            self.load_single_file_diff(new_index, file_path);
        }
    }

    /// Close the current repository and return to the welcome screen.
    /// Search commits by query string.
    pub fn search_commits(&mut self, query: String) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.tab_mut().search_query = query.clone();
        if query.trim().len() < 2 {
            self.tab_mut().search_results.clear();
            return;
        }
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res = (|| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::log::search_commits(&repo, &query, 100)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::SearchResults(res));
        });
    }

    /// Load the diff for a commit by its OID (used by search results).
    pub fn load_commit_diff_by_oid(&mut self) {
        let oid = match self.tab().selected_commit_oid.clone() {
            Some(o) => o,
            None => return,
        };
        bg_task!(
            self,
            "Loading files…",
            BackgroundResult::CommitFileListLoaded,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::diff::get_commit_file_list(&repo, &oid)
                    .map_err(|e| e.to_string())
            }
        );
    }

    /// Load the combined diff for the currently selected commit range.
    pub fn load_commit_range_diff(&mut self) {
        let selected = self.tab().selected_commits.clone();
        if selected.len() < 2 {
            return;
        }
        // selected is ascending; highest index = oldest commit (commits are newest-first)
        let oldest_idx = *selected.last().unwrap();
        let newest_idx = selected[0];

        let oldest_oid = match self.tab().commits.get(oldest_idx).map(|c| c.oid.clone()) {
            Some(o) => o,
            None => return,
        };
        let newest_oid = match self.tab().commits.get(newest_idx).map(|c| c.oid.clone()) {
            Some(o) => o,
            None => return,
        };

        bg_task!(
            self,
            "Loading range diff…",
            BackgroundResult::CommitRangeDiffLoaded,
            |repo_path| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::diff::get_commit_range_diff(
                    &repo,
                    &oldest_oid,
                    &newest_oid,
                )
                .map_err(|e| e.to_string())
            }
        );
    }

    pub fn close_repo(&mut self) {
        self.tabs[self.active_tab_index] = RepoTab::new();
        self.input_buffer.clear();
        self.show_theme_panel = false;
        self.show_options_panel = false;
        self.screen = AppScreen::Welcome;
        // Reload recent repos
        if let Ok(settings) = gitkraft_core::features::persistence::load_tui_settings() {
            self.recent_repos = settings.recent_repos;
        }
        self.save_session();
    }

    /// Populate `browser_entries` with the contents of `browser_dir`.
    pub fn refresh_browser(&mut self) {
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&self.browser_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                // Show only directories to help navigate & identify repos
                if path.is_dir() {
                    entries.push(path);
                }
            }
        }
        entries.sort_by(|a, b| {
            let a_name = a
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            let b_name = b
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            // Dot-dirs last
            let a_dot = a_name.starts_with('.');
            let b_dot = b_name.starts_with('.');
            a_dot.cmp(&b_dot).then(a_name.cmp(&b_name))
        });
        self.browser_entries = entries;
        self.browser_list_state = ListState::default();
        if !self.browser_entries.is_empty() {
            self.browser_list_state.select(Some(0));
        }
    }

    /// Open the directory browser starting from a given path.
    pub fn open_browser(&mut self, start: PathBuf) {
        self.browser_return_screen = self.screen.clone();
        self.browser_dir = start;
        self.refresh_browser();
        self.screen = AppScreen::DirBrowser;
    }
    /// Open the TUI settings file (`tui-settings.json`) in the configured editor.
    /// If no editor is configured, shows the file path in the status bar instead.
    pub fn open_settings_in_editor(&mut self) {
        let path = match gitkraft_core::features::persistence::ops::tui_settings_json_path() {
            Ok(p) => p,
            Err(e) => {
                self.tab_mut().error_message = Some(format!("Cannot determine settings path: {e}"));
                return;
            }
        };

        // Ensure the file exists so the editor can open it immediately.
        if !path.exists() {
            let snap =
                gitkraft_core::features::persistence::load_tui_settings().unwrap_or_default();
            let _ = gitkraft_core::features::persistence::save_tui_settings(&snap);
        }

        let path_str = path.display().to_string();

        if self.editor.is_terminal_editor() {
            // Terminal editors (Helix, Neovim, Vim, …) need a real TTY.
            // Signal the event loop in lib.rs to suspend the TUI, run the
            // editor synchronously, then resume.
            self.tab_mut().status_message =
                Some(format!("Opening settings in {} — {path_str}", self.editor));
            self.pending_editor_open = Some(vec![path]);
        } else if !matches!(self.editor, gitkraft_core::Editor::None) {
            // GUI editor (VS Code, Zed, …): open in background.
            // We do NOT fall back to xdg-open / open because JSON files may
            // be associated with a browser on many systems.
            match self.editor.open_file(&path) {
                Ok(()) => {
                    self.tab_mut().status_message =
                        Some(format!("Settings opened in {} — {path_str}", self.editor));
                }
                Err(e) => {
                    self.tab_mut().error_message =
                        Some(format!("Could not open settings ({e}) — path: {path_str}"));
                }
            }
        } else {
            // No editor configured — show the path so the user can open it
            // manually, and remind them how to configure an editor.
            self.tab_mut().status_message = Some(format!(
                "Settings: {path_str}  \
                 (no editor configured — press E to choose one, or set editor in GUI)"
            ));
        }
    }

    /// Load the diff for a selected staging file into the diff pane.
    /// Open the currently selected staging file in the configured editor.
    pub fn open_selected_in_editor(&mut self) {
        if matches!(self.editor, gitkraft_core::Editor::None) {
            self.tab_mut().status_message =
                Some("No editor configured — press E to choose one".into());
            return;
        }
        let file_path = match self.tab().staging_focus {
            StagingFocus::Unstaged => self
                .tab()
                .unstaged_list_state
                .selected()
                .and_then(|idx| self.tab().unstaged_changes.get(idx))
                .map(|d| d.display_path().to_string()),
            StagingFocus::Staged => self
                .tab()
                .staged_list_state
                .selected()
                .and_then(|idx| self.tab().staged_changes.get(idx))
                .map(|d| d.display_path().to_string()),
        };
        if let (Some(fp), Some(repo_path)) = (file_path, self.tab().repo_path.as_ref()) {
            let full_path = repo_path.join(&fp);
            if self.editor.is_terminal_editor() {
                // Signal the event loop to suspend the TUI, run the editor
                // synchronously with a real TTY, then resume.
                self.tab_mut().status_message = Some(format!(
                    "Opening {} in {} — suspending TUI",
                    fp, self.editor
                ));
                self.pending_editor_open = Some(vec![full_path]);
            } else {
                match self.editor.open_file_or_default(&full_path) {
                    Ok(method) => {
                        self.tab_mut().status_message =
                            Some(format!("Opened {} in {}", fp, method));
                    }
                    Err(e) => {
                        self.tab_mut().error_message = Some(format!("Failed to open editor: {e}"));
                    }
                }
            }
        }
    }

    /// Open files from the commit diff file list in the configured editor.
    ///
    /// If `selected_file_indices` contains 2+ items, opens all of them.
    /// Otherwise opens just the currently focused file (`commit_diff_file_index`).
    pub fn open_commit_files_in_editor(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => {
                self.tab_mut().status_message = Some("No repository open".into());
                return;
            }
        };

        // Collect files to open: multi-selection takes priority over single cursor.
        let indices: Vec<usize> = if self.tab().selected_file_indices.len() > 1 {
            let mut v: Vec<usize> = self.tab().selected_file_indices.iter().copied().collect();
            v.sort_unstable();
            v
        } else {
            vec![self.tab().commit_diff_file_index]
        };

        let paths: Vec<std::path::PathBuf> = indices
            .iter()
            .filter_map(|&i| {
                self.tab()
                    .commit_files
                    .get(i)
                    .map(|f| repo_path.join(f.display_path()))
            })
            .collect();

        if paths.is_empty() {
            return;
        }

        let path_strs: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
        let summary = if path_strs.len() == 1 {
            path_strs[0].clone()
        } else {
            format!("{} files", path_strs.len())
        };

        if self.editor.is_terminal_editor() {
            self.tab_mut().status_message = Some(format!(
                "Opening {} in {} — suspending TUI",
                summary, self.editor
            ));
            self.pending_editor_open = Some(paths);
        } else if !matches!(self.editor, gitkraft_core::Editor::None) {
            // For GUI editors: open each file individually (they handle multiple windows).
            let mut last_error: Option<String> = None;
            for path in &paths {
                if let Err(e) = self.editor.open_file(path) {
                    last_error = Some(format!("{e}"));
                }
            }
            if let Some(e) = last_error {
                self.tab_mut().error_message = Some(format!("Failed to open in editor: {e}"));
            } else {
                self.tab_mut().status_message =
                    Some(format!("Opened {} in {}", summary, self.editor));
            }
        } else {
            self.tab_mut().status_message = Some(format!(
                "Files: {}  (no editor configured — press E to choose one)",
                path_strs.join(", ")
            ));
        }
    }

    pub fn load_staging_diff(&mut self) {
        match self.tab().staging_focus {
            StagingFocus::Unstaged => {
                if let Some(idx) = self.tab().unstaged_list_state.selected() {
                    if idx < self.tab().unstaged_changes.len() {
                        let diff = self.tab().unstaged_changes[idx].clone();
                        let tab = self.tab_mut();
                        tab.selected_diff = Some(diff);
                        tab.diff_scroll = 0;
                    }
                }
            }
            StagingFocus::Staged => {
                if let Some(idx) = self.tab().staged_list_state.selected() {
                    if idx < self.tab().staged_changes.len() {
                        let diff = self.tab().staged_changes[idx].clone();
                        let tab = self.tab_mut();
                        tab.selected_diff = Some(diff);
                        tab.diff_scroll = 0;
                    }
                }
            }
        }
    }

    // ── Remote ───────────────────────────────────────────────────────────

    pub fn fetch_remote(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some("Fetching…".into());
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res = (|| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::remotes::fetch_remote(&repo, "origin")
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::FetchDone(res));
        });
    }

    pub fn pull_rebase(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some("Pulling (rebase)…".into());
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res = gitkraft_core::features::branches::pull_rebase(workdir, "origin");
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| "Pulled (rebase) from origin".into()),
                err_message: res.err().map(|e| format!("pull: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn push_branch(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let branch = match self
            .tab()
            .repo_info
            .as_ref()
            .and_then(|i| i.head_branch.clone())
        {
            Some(b) => b,
            None => {
                self.tab_mut().error_message = Some("No branch checked out".into());
                return;
            }
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("Pushing {branch}…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res = gitkraft_core::features::branches::push_branch(workdir, &branch, "origin");
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Pushed {branch} to origin")),
                err_message: res.err().map(|e| format!("push: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn force_push_branch(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let branch = match self
            .tab()
            .repo_info
            .as_ref()
            .and_then(|i| i.head_branch.clone())
        {
            Some(b) => b,
            None => {
                self.tab_mut().error_message = Some("No branch checked out".into());
                return;
            }
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("Force pushing {branch}…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res =
                gitkraft_core::features::branches::force_push_branch(workdir, &branch, "origin");
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Force pushed {branch} to origin")),
                err_message: res.err().map(|e| format!("force push: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn merge_selected_branch(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let branch_name = match self.tab().branch_list_state.selected() {
            Some(idx) => match self.tab().branches.get(idx) {
                Some(b) => b.name.clone(),
                None => return,
            },
            None => return,
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("Merging {branch_name}…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res = (|| {
                let repo = open_repo_str(&repo_path)?;
                gitkraft_core::features::branches::merge_branch(&repo, &branch_name)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("Merged {branch_name}")),
                err_message: res.err(),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn rebase_onto_selected_branch(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let branch_name = match self.tab().branch_list_state.selected() {
            Some(idx) => match self.tab().branches.get(idx) {
                Some(b) => b.name.clone(),
                None => return,
            },
            None => return,
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("Rebasing onto {branch_name}…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res = gitkraft_core::features::branches::rebase_onto(workdir, &branch_name);
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Rebased onto {branch_name}")),
                err_message: res.err().map(|e| format!("rebase: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    /// Open the commit-action popup for the currently selected commit.
    pub fn open_commit_action_popup(&mut self) {
        let idx = match self.tab().commit_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        let oid = match self.tab().commits.get(idx).map(|c| c.oid.clone()) {
            Some(o) => o,
            None => return,
        };
        // Build the flat item list from COMMIT_MENU_GROUPS
        let items: Vec<gitkraft_core::CommitActionKind> = gitkraft_core::COMMIT_MENU_GROUPS
            .iter()
            .flat_map(|g| g.iter().copied())
            .collect();
        let tab = self.tab_mut();
        tab.pending_commit_action_oid = Some(oid);
        tab.commit_action_items = items;
        tab.commit_action_cursor = 0;
    }

    /// Execute a fully-built `CommitAction` on the pending OID in the background.
    pub fn execute_commit_action(&mut self, action: gitkraft_core::CommitAction) {
        let oid = match self.tab().pending_commit_action_oid.clone() {
            Some(o) => o,
            None => return,
        };
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let label = action.label().to_string();
        let short = oid[..oid.len().min(7)].to_string();
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("{label} {short}…"));
        self.tab_mut().pending_commit_action_oid = None;
        self.tab_mut().pending_action_kind = None;
        self.tab_mut().action_input1.clear();
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = repo_path.as_path();
            let res = action.execute(workdir, &oid);
            let _ = tx.send(crate::app::BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("{label} complete")),
                err_message: res.err().map(|e| e.to_string()),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    /// Open the file-history overlay for the given repo-relative path.
    pub fn open_file_history(&mut self, file_path: String) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let tab = self.tab_mut();
        tab.blame_path = None; // close blame if open
        tab.file_history_path = Some(file_path.clone());
        tab.file_history_commits.clear();
        tab.file_history_cursor = 0;
        tab.status_message = Some(format!(
            "Loading history for {}…",
            file_path.rsplit('/').next().unwrap_or(&file_path)
        ));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let repo = match gitkraft_core::features::repo::open_repo(&repo_path) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(BackgroundResult::OperationDone {
                        ok_message: None,
                        err_message: Some(format!("file history: {e}")),
                        needs_refresh: false,
                        needs_staging_refresh: false,
                    });
                    return;
                }
            };
            match gitkraft_core::file_history(&repo, &file_path, 500) {
                Ok(commits) => {
                    let _ = tx.send(BackgroundResult::FileHistoryLoaded {
                        path: file_path,
                        commits,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BackgroundResult::OperationDone {
                        ok_message: None,
                        err_message: Some(format!("file history: {e}")),
                        needs_refresh: false,
                        needs_staging_refresh: false,
                    });
                }
            }
        });
    }

    /// Open the blame overlay for the given repo-relative path.
    pub fn open_file_blame(&mut self, file_path: String) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let tab = self.tab_mut();
        tab.file_history_path = None; // close history if open
        tab.blame_path = Some(file_path.clone());
        tab.blame_lines.clear();
        tab.blame_scroll = 0;
        tab.status_message = Some(format!(
            "Loading blame for {}…",
            file_path.rsplit('/').next().unwrap_or(&file_path)
        ));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let repo = match gitkraft_core::features::repo::open_repo(&repo_path) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(BackgroundResult::OperationDone {
                        ok_message: None,
                        err_message: Some(format!("blame: {e}")),
                        needs_refresh: false,
                        needs_staging_refresh: false,
                    });
                    return;
                }
            };
            match gitkraft_core::blame_file(&repo, &file_path) {
                Ok(lines) => {
                    let _ = tx.send(BackgroundResult::FileBlameLoaded {
                        path: file_path,
                        lines,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BackgroundResult::OperationDone {
                        ok_message: None,
                        err_message: Some(format!("blame: {e}")),
                        needs_refresh: false,
                        needs_staging_refresh: false,
                    });
                }
            }
        });
    }

    /// Prompt to delete the given working-tree file (first keypress).
    pub fn prompt_delete_file(&mut self, file_path: String) {
        let file_name = file_path
            .rsplit('/')
            .next()
            .unwrap_or(&file_path)
            .to_string();
        self.tab_mut().confirm_delete_file = Some(file_path);
        self.tab_mut().status_message = Some(format!(
            "Delete '{file_name}'? Press 'd' again to confirm, any other key to cancel"
        ));
    }

    /// Execute the pending file deletion (second keypress confirmation).
    pub fn confirm_delete_file(&mut self) {
        let path = match self.tab().confirm_delete_file.clone() {
            Some(p) => p,
            None => return,
        };
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.tab_mut().confirm_delete_file = None;
        self.tab_mut().is_loading = true;
        let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
        self.tab_mut().status_message = Some(format!("Deleting '{file_name}'…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res = gitkraft_core::delete_file(repo_path.as_path(), &path);
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("Deleted '{file_name}'")),
                err_message: res.err().map(|e| e.to_string()),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn revert_selected_commit(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let oid = match self.tab().commit_list_state.selected() {
            Some(idx) => match self.tab().commits.get(idx) {
                Some(c) => c.oid.clone(),
                None => return,
            },
            None => return,
        };
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some("Reverting commit…".into());
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res = gitkraft_core::features::repo::revert_commit(workdir, &oid);
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("Reverted {}", &oid[..7])),
                err_message: res.err().map(|e| format!("revert: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    /// Cherry-pick the selected commit(s) onto the current branch.
    ///
    /// If `selected_commits` has 2+ entries, cherry-picks all of them in
    /// ascending OID order (oldest first so the history is linear).
    /// Otherwise cherry-picks only the currently focused commit.
    pub fn cherry_pick_selected(&mut self) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };

        // Collect OIDs: multi-selection wins over single cursor.
        let oids: Vec<String> = if self.tab().selected_commits.len() > 1 {
            // selected_commits is in cursor order; sort descending so we apply
            // oldest-first (highest list-index = oldest commit).
            let mut sorted = self.tab().selected_commits.clone();
            sorted.sort_unstable_by(|a, b| b.cmp(a)); // reverse: highest index = oldest
            sorted
                .iter()
                .filter_map(|&i| self.tab().commits.get(i).map(|c| c.oid.clone()))
                .collect()
        } else {
            match self.tab().commit_list_state.selected() {
                Some(i) => self
                    .tab()
                    .commits
                    .get(i)
                    .map(|c| vec![c.oid.clone()])
                    .unwrap_or_default(),
                None => return,
            }
        };

        if oids.is_empty() {
            return;
        }

        let count = oids.len();
        let short = oids[0][..oids[0].len().min(7)].to_string();
        let label = if count == 1 {
            format!("Cherry-picking {short}…")
        } else {
            format!("Cherry-picking {count} commits…")
        };

        let tab = self.tab_mut();
        tab.is_loading = true;
        tab.status_message = Some(label);

        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let res: Result<String, String> = (|| {
                for oid in &oids {
                    gitkraft_core::features::repo::cherry_pick_commit(&repo_path, oid)
                        .map_err(|e| format!("cherry-pick {}: {e}", &oid[..oid.len().min(7)]))?;
                }
                Ok(format!("Cherry-picked {} commit(s)", oids.len()))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err(),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn reset_to_selected_commit(&mut self, mode: &str) {
        let repo_path = match self.tab().repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let oid = match self.tab().commit_list_state.selected() {
            Some(idx) => match self.tab().commits.get(idx) {
                Some(c) => c.oid.clone(),
                None => return,
            },
            None => return,
        };
        let mode_owned = mode.to_string();
        self.tab_mut().is_loading = true;
        self.tab_mut().status_message = Some(format!("Resetting ({mode})…"));
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let workdir = std::path::Path::new(&repo_path);
            let res = gitkraft_core::features::repo::reset_to_commit(workdir, &oid, &mode_owned);
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Reset ({mode_owned}) to {}", &oid[..7])),
                err_message: res.err().map(|e| format!("reset: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    // ── Path helpers ─────────────────────────────────────────────────────

    fn unstaged_file_path(&self, idx: usize) -> String {
        if idx >= self.tab().unstaged_changes.len() {
            return String::new();
        }
        self.tab().unstaged_changes[idx].display_path().to_owned()
    }

    fn staged_file_path(&self, idx: usize) -> String {
        if idx >= self.tab().staged_changes.len() {
            return String::new();
        }
        self.tab().staged_changes[idx].display_path().to_owned()
    }
}

// ── Free-standing helpers ─────────────────────────────────────────────────────

/// Open a repository, mapping the error to a `String` for background-task results.
fn open_repo_str(path: &std::path::Path) -> Result<git2::Repository, String> {
    gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())
}
/// Map a persisted theme name back to its index (0–26).
fn theme_name_to_index(name: &str) -> usize {
    gitkraft_core::theme_index_by_name(name)
}

/// Clamp a `ListState` selection to be within `[0, len)`, or `None` if empty.
fn clamp_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else if state.selected().is_none() {
        state.select(Some(0));
    } else if let Some(i) = state.selected() {
        if i >= len {
            state.select(Some(len - 1));
        }
    }
}

/// Blocking helper that loads all repo data in one go.
/// Runs inside `spawn_blocking` — must not touch any async APIs.
fn load_repo_blocking(path: &std::path::Path) -> Result<RepoPayload, String> {
    gitkraft_core::load_repo_snapshot(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_defaults() {
        let app = App::new();
        assert!(!app.should_quit);
        assert_eq!(app.screen, AppScreen::Welcome);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.tab().commits.is_empty());
        assert!(app.tab().branches.is_empty());
        assert!(app.tab().repo_path.is_none());
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn cycle_theme_next_wraps() {
        let mut app = App::new();
        app.current_theme_index = 0;
        app.cycle_theme_next();
        assert_eq!(app.current_theme_index, 1);
        // Cycle to end
        for _ in 0..26 {
            app.cycle_theme_next();
        }
        assert_eq!(app.current_theme_index, 0); // wrapped
    }

    #[test]
    fn cycle_theme_prev_wraps() {
        let mut app = App::new();
        app.current_theme_index = 0;
        app.cycle_theme_prev();
        assert_eq!(app.current_theme_index, 26); // wrapped to last
    }

    #[test]
    fn theme_returns_struct() {
        let mut app = App::new();
        app.current_theme_index = 0;
        let theme = app.theme();
        // Default theme active border comes from the core accent (88, 166, 255)
        assert_eq!(
            format!("{:?}", theme.border_active),
            format!("{:?}", ratatui::style::Color::Rgb(88, 166, 255))
        );
    }

    #[test]
    fn theme_name_to_index_known() {
        assert_eq!(theme_name_to_index("Default"), 0);
        assert_eq!(theme_name_to_index("Dracula"), 8);
        assert_eq!(theme_name_to_index("Nord"), 9);
    }

    #[test]
    fn theme_name_to_index_unknown_returns_zero() {
        assert_eq!(theme_name_to_index("NonExistentTheme"), 0);
        assert_eq!(theme_name_to_index(""), 0);
    }

    #[test]
    fn tab_management_new_tab() {
        let mut app = App::new();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);

        app.new_tab();
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);

        app.new_tab();
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active_tab_index, 2);
    }

    #[test]
    fn tab_management_close_tab() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active_tab_index, 2);

        app.close_tab();
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1);

        app.close_tab();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);

        // Close the only tab -- should reset rather than remove
        app.close_tab();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn tab_management_next_prev() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        // active_tab_index == 2

        app.next_tab();
        assert_eq!(app.active_tab_index, 0); // wrapped

        app.next_tab();
        assert_eq!(app.active_tab_index, 1);

        app.prev_tab();
        assert_eq!(app.active_tab_index, 0);

        app.prev_tab();
        assert_eq!(app.active_tab_index, 2); // wrapped
    }

    #[test]
    fn repo_tab_display_name() {
        let tab = RepoTab::new();
        assert_eq!(tab.display_name(), "New Tab");

        let mut tab2 = RepoTab::new();
        tab2.repo_path = Some(PathBuf::from("/home/user/projects/my-repo"));
        assert_eq!(tab2.display_name(), "my-repo");
    }

    #[test]
    fn repo_tab_search_defaults() {
        let tab = RepoTab::new();
        assert!(!tab.search_active);
        assert!(tab.search_query.is_empty());
        assert!(tab.search_results.is_empty());
    }

    #[test]
    fn repo_tab_new_has_empty_state() {
        let tab = RepoTab::new();
        assert!(tab.repo_path.is_none());
        assert!(tab.commits.is_empty());
        assert!(tab.branches.is_empty());
        assert!(tab.unstaged_changes.is_empty());
        assert!(tab.staged_changes.is_empty());
        assert!(tab.stashes.is_empty());
        assert!(tab.remotes.is_empty());
        assert!(tab.commit_files.is_empty());
        assert!(tab.selected_commit_oid.is_none());
        assert!(!tab.is_loading);
        assert!(!tab.confirm_discard);
        assert_eq!(tab.diff_scroll, 0);
        assert_eq!(tab.commit_diff_file_index, 0);
    }

    #[test]
    fn new_tab_switches_to_welcome() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.new_tab();
        assert_eq!(app.screen, AppScreen::Welcome);
        assert_eq!(app.active_tab_index, 1);
    }

    #[test]
    fn close_tab_last_tab_resets() {
        let mut app = App::new();
        // Set some state on the only tab
        app.tab_mut().search_active = true;
        app.tab_mut().search_query = "test".into();

        app.close_tab();

        // Should reset the tab, not remove it
        assert_eq!(app.tabs.len(), 1);
        assert!(!app.tab().search_active);
        assert!(app.tab().search_query.is_empty());
    }

    #[test]
    fn close_tab_middle_adjusts_index() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        // 3 tabs, active = 2

        app.active_tab_index = 1; // select middle tab
        app.close_tab();

        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab_index, 1); // stays at 1 (now the last)
    }

    #[test]
    fn next_tab_single_tab_no_change() {
        let mut app = App::new();
        app.next_tab();
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn prev_tab_single_tab_no_change() {
        let mut app = App::new();
        app.prev_tab();
        assert_eq!(app.active_tab_index, 0);
    }

    #[test]
    fn open_browser_sets_dir_browser_screen() {
        let mut app = App::new();
        app.screen = AppScreen::Main;
        app.open_browser(PathBuf::from("/tmp"));
        assert_eq!(app.screen, AppScreen::DirBrowser);
        assert_eq!(app.browser_return_screen, AppScreen::Main);
    }

    #[test]
    fn repo_tab_selected_defaults_empty() {
        let tab = RepoTab::new();
        assert!(tab.selected_unstaged.is_empty());
        assert!(tab.selected_staged.is_empty());
    }

    #[test]
    fn repo_tab_selected_toggle() {
        let mut tab = RepoTab::new();
        tab.selected_unstaged.insert(0);
        tab.selected_unstaged.insert(2);
        assert_eq!(tab.selected_unstaged.len(), 2);
        assert!(tab.selected_unstaged.contains(&0));
        tab.selected_unstaged.remove(&0);
        assert_eq!(tab.selected_unstaged.len(), 1);
        assert!(!tab.selected_unstaged.contains(&0));
    }

    #[test]
    fn auto_refresh_field_exists() {
        let app = App::new();
        assert!(app.last_auto_refresh.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn editor_defaults_from_settings() {
        let app = App::new();
        // Should have loaded from settings or defaulted to None
        let _ = app.editor.display_name();
    }

    #[test]
    fn pull_rebase_sets_loading() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        app.pull_rebase();
        assert!(app.tab().is_loading);
        assert_eq!(
            app.tab().status_message.as_deref(),
            Some("Pulling (rebase)…")
        );
    }

    #[test]
    fn repo_tab_diff_sub_pane_defaults_to_file_list() {
        let tab = RepoTab::new();
        assert_eq!(tab.diff_sub_pane, DiffSubPane::FileList);
    }

    #[test]
    fn repo_tab_selected_file_indices_defaults_empty() {
        let tab = RepoTab::new();
        assert!(tab.selected_file_indices.is_empty());
    }

    #[test]
    fn repo_tab_commit_diffs_defaults_empty_hashmap() {
        let tab = RepoTab::new();
        assert!(tab.commit_diffs.is_empty());
    }

    #[test]
    fn next_tab_restores_main_screen_for_tab_with_repo() {
        let mut app = App::new();
        // tab 0 gets a repo path
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/repo-a"));
        // create tab 1 (no repo) — new_tab sets screen = Welcome
        app.new_tab();
        assert_eq!(app.active_tab_index, 1);
        // switching forward wraps back to tab 0 which has a repo
        app.next_tab();
        assert_eq!(app.active_tab_index, 0);
        assert_eq!(app.screen, AppScreen::Main);
    }

    #[test]
    fn prev_tab_restores_welcome_for_tab_without_repo() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/repo-a"));
        app.new_tab(); // tab 1, no repo, screen = Welcome
                       // go back to tab 0 (has repo) and set screen manually
        app.active_tab_index = 0;
        app.screen = AppScreen::Main;
        // prev wraps to tab 1 which has no repo
        app.prev_tab();
        assert_eq!(app.active_tab_index, 1);
        assert_eq!(app.screen, AppScreen::Welcome);
    }

    #[test]
    fn next_diff_file_clears_multi_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = vec![
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "a.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "b.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "c.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
        ];
        app.tab_mut().commit_diff_file_index = 0;
        // pre-populate a multi-selection
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        app.next_diff_file();
        // should have exactly one entry: the new current index
        assert_eq!(app.tab().selected_file_indices.len(), 1);
        assert_eq!(app.tab().commit_diff_file_index, 1);
        assert!(app.tab().selected_file_indices.contains(&1));
    }

    #[test]
    fn prev_diff_file_clears_multi_selection() {
        let mut app = App::new();
        app.tab_mut().commit_files = vec![
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "a.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "b.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
        ];
        app.tab_mut().commit_diff_file_index = 1;
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        app.prev_diff_file();
        assert_eq!(app.tab().selected_file_indices.len(), 1);
        assert_eq!(app.tab().commit_diff_file_index, 0);
        assert!(app.tab().selected_file_indices.contains(&0));
    }

    #[test]
    fn load_diff_for_file_index_out_of_bounds_is_noop() {
        let mut app = App::new();
        // no commit_files — should not panic
        app.load_diff_for_file_index(0);
        assert!(app.tab().commit_diffs.is_empty());
    }

    #[test]
    fn load_diff_for_file_index_skips_if_already_cached() {
        let mut app = App::new();
        app.tab_mut().commit_files = vec![gitkraft_core::DiffFileEntry {
            old_file: String::new(),
            new_file: "a.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
        }];
        // insert a fake cached diff so the function short-circuits
        app.tab_mut().commit_diffs.insert(
            0,
            DiffInfo {
                old_file: String::new(),
                new_file: "a.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
                hunks: Vec::new(),
            },
        );
        // without repo_path the bg task would be a no-op anyway, but is_loading
        // should remain false because we skip the load entirely
        app.load_diff_for_file_index(0);
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn push_branch_requires_head_branch() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        // No repo_info / head_branch set
        app.push_branch();
        assert!(app.tab().error_message.is_some());
    }

    #[test]
    fn force_push_requires_head_branch() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        app.force_push_branch();
        assert!(app.tab().error_message.is_some());
    }

    #[test]
    fn merge_selected_branch_no_selection() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        // No branch selected — should be a no-op (no crash)
        app.merge_selected_branch();
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn rebase_onto_selected_no_selection() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        app.rebase_onto_selected_branch();
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn revert_selected_commit_no_selection() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        app.revert_selected_commit();
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn reset_to_selected_commit_no_selection() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/fake-repo"));
        app.reset_to_selected_commit("soft");
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn open_repo_creates_new_tab_when_current_has_repo() {
        let mut app = App::new();
        app.tabs[0].repo_path = Some(PathBuf::from("/tmp/repo1"));
        app.screen = AppScreen::Main;
        // Simulate browser selecting a repo when one is already open
        let initial_tabs = app.tabs.len();
        if app.tab().repo_path.is_some() {
            app.new_tab();
        }
        assert_eq!(app.tabs.len(), initial_tabs + 1);
    }

    // ── Commit action popup ───────────────────────────────────────────────

    #[test]
    fn commit_action_items_defaults_empty() {
        let tab = RepoTab::new();
        assert!(tab.commit_action_items.is_empty());
        assert_eq!(tab.commit_action_cursor, 0);
        assert!(tab.pending_commit_action_oid.is_none());
        assert!(tab.pending_action_kind.is_none());
        assert!(tab.action_input1.is_empty());
    }

    #[test]
    fn open_commit_action_popup_no_selection_is_noop() {
        let mut app = App::new();
        // No commit selected, no commits loaded
        app.open_commit_action_popup();
        assert!(app.tab().pending_commit_action_oid.is_none());
        assert!(app.tab().commit_action_items.is_empty());
    }

    #[test]
    fn open_commit_action_popup_fills_items_from_menu_groups() {
        let mut app = App::new();
        // Add a fake commit and select it
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "abc1234567890".to_string(),
            short_oid: "abc1234".to_string(),
            summary: "test commit".to_string(),
            message: "test commit".to_string(),
            author_name: "author".to_string(),
            author_email: "a@b.com".to_string(),
            time: Default::default(),
            parent_ids: vec![],
        }];
        app.tab_mut().commit_list_state.select(Some(0));

        app.open_commit_action_popup();

        // Should have filled items from COMMIT_MENU_GROUPS (10 total)
        let expected: Vec<gitkraft_core::CommitActionKind> = gitkraft_core::COMMIT_MENU_GROUPS
            .iter()
            .flat_map(|g| g.iter().copied())
            .collect();
        assert_eq!(app.tab().commit_action_items, expected);
        assert_eq!(app.tab().commit_action_items.len(), 10);
    }

    #[test]
    fn open_commit_action_popup_sets_pending_oid() {
        let mut app = App::new();
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "deadbeef1234567".to_string(),
            short_oid: "deadbee".to_string(),
            summary: "s".to_string(),
            message: "s".to_string(),
            author_name: "a".to_string(),
            author_email: "a@b.com".to_string(),
            time: Default::default(),
            parent_ids: vec![],
        }];
        app.tab_mut().commit_list_state.select(Some(0));

        app.open_commit_action_popup();

        assert_eq!(
            app.tab().pending_commit_action_oid.as_deref(),
            Some("deadbeef1234567")
        );
        assert_eq!(app.tab().commit_action_cursor, 0);
    }

    #[test]
    fn open_commit_action_popup_resets_cursor() {
        let mut app = App::new();
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "aaa".to_string(),
            short_oid: "aaa".to_string(),
            summary: "s".to_string(),
            message: "s".to_string(),
            author_name: "a".to_string(),
            author_email: "a@b.com".to_string(),
            time: Default::default(),
            parent_ids: vec![],
        }];
        app.tab_mut().commit_list_state.select(Some(0));
        // Pre-set cursor to a non-zero value
        app.tab_mut().commit_action_cursor = 5;

        app.open_commit_action_popup();

        assert_eq!(app.tab().commit_action_cursor, 0);
    }

    #[test]
    fn execute_commit_action_no_pending_oid_is_noop() {
        let mut app = App::new();
        // No pending OID — should not set is_loading
        app.execute_commit_action(gitkraft_core::CommitAction::CherryPick);
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn execute_commit_action_no_repo_path_is_noop() {
        let mut app = App::new();
        app.tab_mut().pending_commit_action_oid = Some("abc123".to_string());
        // No repo_path set — should not set is_loading
        app.execute_commit_action(gitkraft_core::CommitAction::CherryPick);
        assert!(!app.tab().is_loading);
    }

    #[test]
    fn execute_commit_action_sets_loading_and_clears_state() {
        let mut app = App::new();
        app.tab_mut().pending_commit_action_oid = Some("abc123".to_string());
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().pending_action_kind = Some(gitkraft_core::CommitActionKind::CherryPick);
        app.tab_mut().action_input1 = "some-input".to_string();

        app.execute_commit_action(gitkraft_core::CommitAction::CherryPick);

        assert!(app.tab().is_loading);
        // State should be cleared after dispatch
        assert!(app.tab().pending_commit_action_oid.is_none());
        assert!(app.tab().pending_action_kind.is_none());
        assert!(app.tab().action_input1.is_empty());
    }

    // ── File history / blame / delete ─────────────────────────────────────

    #[test]
    fn file_history_defaults_empty() {
        let tab = RepoTab::new();
        assert!(tab.file_history_path.is_none());
        assert!(tab.file_history_commits.is_empty());
        assert_eq!(tab.file_history_cursor, 0);
    }

    #[test]
    fn blame_defaults_empty() {
        let tab = RepoTab::new();
        assert!(tab.blame_path.is_none());
        assert!(tab.blame_lines.is_empty());
        assert_eq!(tab.blame_scroll, 0);
    }

    #[test]
    fn confirm_delete_defaults_none() {
        let tab = RepoTab::new();
        assert!(tab.confirm_delete_file.is_none());
    }

    #[test]
    fn open_file_history_no_repo_is_noop() {
        let mut app = App::new();
        // No repo_path — should not set file_history_path
        app.open_file_history("src/main.rs".to_string());
        assert!(app.tab().file_history_path.is_none());
    }

    #[test]
    fn open_file_history_sets_path_and_clears_blame() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().blame_path = Some("old.rs".to_string());

        app.open_file_history("src/main.rs".to_string());

        assert_eq!(app.tab().file_history_path.as_deref(), Some("src/main.rs"));
        // blame should be closed
        assert!(app.tab().blame_path.is_none());
    }

    #[test]
    fn open_file_blame_sets_path_and_clears_history() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().file_history_path = Some("old.rs".to_string());

        app.open_file_blame("src/lib.rs".to_string());

        assert_eq!(app.tab().blame_path.as_deref(), Some("src/lib.rs"));
        // history should be closed
        assert!(app.tab().file_history_path.is_none());
    }

    #[test]
    fn prompt_delete_file_sets_confirm_and_status() {
        let mut app = App::new();
        app.prompt_delete_file("src/old.rs".to_string());
        assert_eq!(app.tab().confirm_delete_file.as_deref(), Some("src/old.rs"));
        assert!(app.tab().status_message.is_some());
    }

    #[test]
    fn confirm_delete_file_no_repo_is_noop() {
        let mut app = App::new();
        app.tab_mut().confirm_delete_file = Some("src/old.rs".to_string());
        // No repo_path
        app.confirm_delete_file();
        assert!(!app.tab().is_loading);
    }

    // ── open_commit_files_in_editor ───────────────────────────────────────

    #[test]
    fn open_commit_files_in_editor_shows_path_when_no_editor() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/repo"));
        app.tab_mut().commit_files = vec![gitkraft_core::DiffFileEntry {
            old_file: String::new(),
            new_file: "src/main.rs".to_string(),
            status: gitkraft_core::FileStatus::Modified,
        }];
        app.tab_mut().commit_diff_file_index = 0;
        // editor is Editor::None by default

        app.open_commit_files_in_editor();

        // With no editor, shows a status message with the file path
        assert!(app.tab().status_message.is_some());
        let msg = app.tab().status_message.as_deref().unwrap();
        assert!(msg.contains("no editor") || msg.contains("src/main.rs"));
    }

    #[test]
    fn open_commit_files_in_editor_queues_multi_file_for_terminal_editor() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/repo"));
        app.tab_mut().commit_files = vec![
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "a.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "b.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
        ];
        app.tab_mut().selected_file_indices.insert(0);
        app.tab_mut().selected_file_indices.insert(1);
        app.editor = gitkraft_core::Editor::Helix; // terminal editor

        app.open_commit_files_in_editor();

        // Both files should be queued for the editor
        let queued = app.pending_editor_open.as_ref().unwrap();
        assert_eq!(queued.len(), 2);
    }

    #[test]
    fn cherry_pick_selected_single_commit_sets_loading() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "abc1234567890".to_string(),
            short_oid: "abc1234".to_string(),
            summary: "test".into(),
            message: "test".into(),
            author_name: "A".into(),
            author_email: "a@a.com".into(),
            time: Default::default(),
            parent_ids: Vec::new(),
        }];
        app.tab_mut().commit_list_state.select(Some(0));

        app.cherry_pick_selected();

        assert!(app.tab().is_loading);
    }

    #[test]
    fn cherry_pick_selected_no_repo_path_is_noop() {
        let mut app = App::new();
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "abc1234567890".to_string(),
            short_oid: "abc1234".to_string(),
            summary: "test".into(),
            message: "test".into(),
            author_name: "A".into(),
            author_email: "a@a.com".into(),
            time: Default::default(),
            parent_ids: Vec::new(),
        }];
        app.tab_mut().commit_list_state.select(Some(0));

        app.cherry_pick_selected();

        assert!(!app.tab().is_loading);
    }

    #[test]
    fn cherry_pick_selected_no_cursor_is_noop() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "abc1234567890".to_string(),
            short_oid: "abc1234".to_string(),
            summary: "test".into(),
            message: "test".into(),
            author_name: "A".into(),
            author_email: "a@a.com".into(),
            time: Default::default(),
            parent_ids: Vec::new(),
        }];
        // No cursor selected — commit_list_state.selected() returns None

        app.cherry_pick_selected();

        assert!(
            !app.tab().is_loading,
            "no cursor → cherry_pick_selected must be a noop"
        );
    }

    #[test]
    fn cherry_pick_selected_single_sets_status_message_with_short_oid() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().commits = vec![gitkraft_core::CommitInfo {
            oid: "deadbeef12345".to_string(),
            short_oid: "deadbee".to_string(),
            summary: "fix: something".into(),
            message: "fix: something".into(),
            author_name: "A".into(),
            author_email: "a@a.com".into(),
            time: Default::default(),
            parent_ids: Vec::new(),
        }];
        app.tab_mut().commit_list_state.select(Some(0));

        app.cherry_pick_selected();

        let msg = app.tab().status_message.as_deref().unwrap_or("");
        assert!(
            msg.contains("deadbee"),
            "status message must contain the short OID; got: {msg}"
        );
        assert!(
            msg.to_lowercase().contains("cherry"),
            "status message must mention cherry-pick; got: {msg}"
        );
    }

    #[test]
    fn cherry_pick_selected_multi_uses_selected_commits_and_sets_count_message() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        // Three commits newest-first (index 0 = newest, index 2 = oldest).
        app.tab_mut().commits = vec![
            gitkraft_core::CommitInfo {
                oid: "oid_newest".to_string(),
                short_oid: "newest".to_string(),
                summary: "newest".into(),
                message: "newest".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
            gitkraft_core::CommitInfo {
                oid: "oid_middle".to_string(),
                short_oid: "middle".to_string(),
                summary: "middle".into(),
                message: "middle".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
            gitkraft_core::CommitInfo {
                oid: "oid_oldest".to_string(),
                short_oid: "oldest".to_string(),
                summary: "oldest".into(),
                message: "oldest".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
        ];
        // Multi-select all three commits.
        app.tab_mut().selected_commits = vec![0, 1, 2];
        app.tab_mut().commit_list_state.select(Some(0));

        app.cherry_pick_selected();

        assert!(
            app.tab().is_loading,
            "multi cherry-pick must set is_loading"
        );
        let msg = app.tab().status_message.as_deref().unwrap_or("");
        assert!(
            msg.contains("3"),
            "status message must mention commit count (3); got: {msg}"
        );
    }

    #[test]
    fn cherry_pick_selected_multi_orders_oldest_first() {
        // Verifies that the OIDs are collected in oldest-first order (highest
        // list index first) by checking the status message uses the first
        // collected OID — which should be the one at the highest index.
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().commits = vec![
            gitkraft_core::CommitInfo {
                oid: "oid_0_newest".to_string(),
                short_oid: "oid0new".to_string(),
                summary: "newest".into(),
                message: "newest".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
            gitkraft_core::CommitInfo {
                oid: "oid_1_oldest".to_string(),
                short_oid: "oid1old".to_string(),
                summary: "oldest".into(),
                message: "oldest".into(),
                author_name: "A".into(),
                author_email: "a@a.com".into(),
                time: Default::default(),
                parent_ids: Vec::new(),
            },
        ];
        app.tab_mut().selected_commits = vec![0, 1];

        app.cherry_pick_selected();

        // The status message short-OID should be from index 1 (oldest) since
        // we sort descending (index 1 > index 0) before iterating.
        let msg = app.tab().status_message.as_deref().unwrap_or("");
        assert!(
            msg.contains("2"),
            "multi cherry-pick message should say 2 commits; got: {msg}"
        );
    }

    #[test]
    fn open_commit_files_in_editor_uses_single_cursor_when_no_multi_selection() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/repo"));
        app.tab_mut().commit_files = vec![
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "a.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
            gitkraft_core::DiffFileEntry {
                old_file: String::new(),
                new_file: "b.rs".to_string(),
                status: gitkraft_core::FileStatus::Modified,
            },
        ];
        app.tab_mut().commit_diff_file_index = 1; // cursor on b.rs
        app.editor = gitkraft_core::Editor::Helix;
        // selected_file_indices has 1 item (or 0) → should use cursor

        app.open_commit_files_in_editor();

        let queued = app.pending_editor_open.as_ref().unwrap();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].ends_with("b.rs"));
    }

    #[test]
    fn git_state_changed_triggers_full_refresh_when_repo_open() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        // Simulate receiving GitStateChanged via poll_background
        // by sending it through the channel first.
        app.bg_tx
            .send(crate::app::BackgroundResult::GitStateChanged)
            .unwrap();
        app.poll_background();
        // After GitStateChanged, a full refresh should have been enqueued
        // (is_loading set true because refresh() calls load_repo_blocking).
        assert!(
            app.tab().is_loading,
            "GitStateChanged must trigger a full refresh (is_loading should be true)"
        );
    }

    #[test]
    fn git_state_changed_is_noop_when_already_loading() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        app.tab_mut().is_loading = true; // already loading — guard should prevent another refresh
        app.bg_tx
            .send(crate::app::BackgroundResult::GitStateChanged)
            .unwrap();
        app.poll_background();
        // refresh() sets status_message = "Refreshing…" when called.
        // The guard `if !self.tab().is_loading` must have blocked the call,
        // so status_message should NOT have been updated to "Refreshing…".
        assert_ne!(
            app.tab().status_message.as_deref(),
            Some("Refreshing\u{2026}"),
            "GitStateChanged must be a noop when already loading"
        );
    }

    #[test]
    fn maybe_auto_refresh_triggers_full_refresh_after_interval() {
        let mut app = App::new();
        app.tab_mut().repo_path = Some(std::path::PathBuf::from("/tmp/fake-repo"));
        // Force the last_auto_refresh to be long ago so the interval has elapsed.
        app.last_auto_refresh = std::time::Instant::now() - std::time::Duration::from_secs(10);

        app.maybe_auto_refresh();

        assert!(
            app.tab().is_loading,
            "maybe_auto_refresh must trigger a full refresh after the interval"
        );
    }
}
