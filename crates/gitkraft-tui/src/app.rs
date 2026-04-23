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

/// Payload produced by a background `refresh` / `open_repo` task.
#[derive(Debug)]
pub struct RepoPayload {
    pub info: RepoInfo,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
    pub graph_rows: Vec<gitkraft_core::GraphRow>,
    pub unstaged: Vec<DiffInfo>,
    pub staged: Vec<DiffInfo>,
    pub stashes: Vec<StashEntry>,
    pub remotes: Vec<RemoteInfo>,
}

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
    /// A single file's diff was loaded.
    SingleFileDiffLoaded(Result<gitkraft_core::DiffInfo, String>),
    /// Commit search results loaded.
    SearchResults(Result<Vec<gitkraft_core::CommitInfo>, String>),
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
}

/// Which sub-list within the staging pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingFocus {
    Unstaged,
    Staged,
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
    /// All file diffs for the currently viewed commit (for per-file navigation).
    pub commit_diffs: Vec<DiffInfo>,
    /// Index of the currently selected file in commit_diffs.
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
            commit_diffs: Vec::new(),
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

    /// Timestamp of the last auto-refresh.
    pub last_auto_refresh: std::time::Instant,
}

impl App {
    // ── Constructor ──────────────────────────────────────────────────────────

    #[must_use]
    pub fn new() -> Self {
        let settings = gitkraft_core::features::persistence::load_settings().unwrap_or_default();

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
        if let Ok(settings) = gitkraft_core::features::persistence::load_settings() {
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
        let _ = gitkraft_core::features::persistence::save_theme(self.current_theme_name());
    }

    /// Persist the paths of all open tabs for session restore.
    pub fn save_session(&self) {
        let paths: Vec<std::path::PathBuf> = self
            .tabs
            .iter()
            .filter_map(|t| t.repo_path.clone())
            .collect();
        let active = self.active_tab_index;
        let _ = gitkraft_core::features::persistence::save_session(&paths, active);
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
                            let _ = gitkraft_core::features::persistence::record_repo_opened(
                                &canonical,
                            );
                            if let Ok(settings) =
                                gitkraft_core::features::persistence::load_settings()
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
                                tab.commit_diffs = diffs.clone();
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

                            if count == 0 {
                                tab.status_message = Some("No changes in this commit".into());
                            } else {
                                tab.status_message = Some(format!("{count} file(s) changed"));
                                // Auto-load the first file's diff
                                let first_path = tab.commit_files[0].display_path().to_string();
                                self.load_single_file_diff(first_path);
                            }
                        }
                        Err(e) => self.tab_mut().error_message = Some(format!("file list: {e}")),
                    }
                }
                BackgroundResult::SingleFileDiffLoaded(res) => {
                    self.tab_mut().is_loading = false;
                    match res {
                        Ok(diff) => {
                            let tab = self.tab_mut();
                            // Store in commit_diffs for the file list sidebar.
                            if tab.commit_diffs.len() <= tab.commit_diff_file_index {
                                tab.commit_diffs.push(diff.clone());
                            } else {
                                tab.commit_diffs[tab.commit_diff_file_index] = diff.clone();
                            }
                            tab.selected_diff = Some(diff);
                            tab.diff_scroll = 0;
                            if tab.commit_files.len() > 1 {
                                tab.status_message = Some(format!(
                                    "File {}/{} — use h/l to switch files",
                                    tab.commit_diff_file_index + 1,
                                    tab.commit_files.len()
                                ));
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
            }
        }
    }

    /// Reload only the staging area (unstaged + staged diffs).
    /// Check if enough time has passed and trigger a staging refresh.
    pub fn maybe_auto_refresh(&mut self) {
        if self.tab().repo_path.is_some()
            && !self.tab().is_loading
            && self.last_auto_refresh.elapsed() >= std::time::Duration::from_secs(3)
        {
            self.last_auto_refresh = std::time::Instant::now();
            self.refresh_staging();
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
    pub fn load_single_file_diff(&mut self, file_path: String) {
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
                gitkraft_core::features::diff::get_single_file_diff(&repo, &oid, &file_path)
                    .map_err(|e| e.to_string())
            }
        );
    }

    /// Switch to the next file in the commit diff list.
    pub fn next_diff_file(&mut self) {
        if self.tab().commit_files.is_empty() {
            return;
        }
        let new_index = (self.tab().commit_diff_file_index + 1) % self.tab().commit_files.len();
        self.tab_mut().commit_diff_file_index = new_index;
        let file_path = self.tab().commit_files[self.tab().commit_diff_file_index]
            .display_path()
            .to_string();
        self.tab_mut().diff_scroll = 0;
        self.tab_mut().status_message = Some(format!(
            "File {}/{}",
            self.tab().commit_diff_file_index + 1,
            self.tab().commit_files.len()
        ));
        self.load_single_file_diff(file_path);
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
        self.tab_mut().commit_diff_file_index = new_index;
        let file_path = self.tab().commit_files[self.tab().commit_diff_file_index]
            .display_path()
            .to_string();
        self.tab_mut().diff_scroll = 0;
        self.tab_mut().status_message = Some(format!(
            "File {}/{}",
            self.tab().commit_diff_file_index + 1,
            self.tab().commit_files.len()
        ));
        self.load_single_file_diff(file_path);
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

    pub fn close_repo(&mut self) {
        self.tabs[self.active_tab_index] = RepoTab::new();
        self.input_buffer.clear();
        self.show_theme_panel = false;
        self.show_options_panel = false;
        self.screen = AppScreen::Welcome;
        // Reload recent repos
        if let Ok(settings) = gitkraft_core::features::persistence::load_settings() {
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
            match self.editor.open_file(&full_path) {
                Ok(()) => {
                    self.tab_mut().status_message =
                        Some(format!("Opened {} in {}", fp, self.editor));
                }
                Err(e) => {
                    self.tab_mut().error_message = Some(format!("Failed to open editor: {e}"));
                }
            }
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
    let mut repo = open_repo_str(path)?;

    let info = gitkraft_core::features::repo::get_repo_info(&repo).map_err(|e| e.to_string())?;
    let branches =
        gitkraft_core::features::branches::list_branches(&repo).map_err(|e| e.to_string())?;
    let commits =
        gitkraft_core::features::commits::list_commits(&repo, 500).map_err(|e| e.to_string())?;
    let graph_rows = gitkraft_core::features::graph::build_graph(&commits);
    let unstaged =
        gitkraft_core::features::diff::get_working_dir_diff(&repo).map_err(|e| e.to_string())?;
    let staged =
        gitkraft_core::features::diff::get_staged_diff(&repo).map_err(|e| e.to_string())?;
    let remotes =
        gitkraft_core::features::remotes::list_remotes(&repo).map_err(|e| e.to_string())?;
    let stashes =
        gitkraft_core::features::stash::list_stashes(&mut repo).map_err(|e| e.to_string())?;

    Ok(RepoPayload {
        info,
        branches,
        commits,
        graph_rows,
        unstaged,
        staged,
        stashes,
        remotes,
    })
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
}
