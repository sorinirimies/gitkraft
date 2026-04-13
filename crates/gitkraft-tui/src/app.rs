use std::path::PathBuf;

use ratatui::widgets::ListState;
use tokio::sync::mpsc;

use gitkraft_core::*;

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
    /// A repo open / refresh completed.
    RepoLoaded(Result<RepoPayload, String>),
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

// ── App State ─────────────────────────────────────────────────────────────────

pub struct App {
    pub should_quit: bool,
    pub screen: AppScreen,
    pub active_pane: ActivePane,
    pub input_mode: InputMode,
    pub input_purpose: InputPurpose,
    pub tick_count: u64,

    /// True while a background task is in flight.
    pub is_loading: bool,
    /// Receiver for results from background tasks.
    pub bg_rx: mpsc::UnboundedReceiver<BackgroundResult>,
    /// Sender cloned into each spawned task.
    bg_tx: mpsc::UnboundedSender<BackgroundResult>,

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
    /// Index of the currently selected file in `commit_diffs`.
    pub commit_diff_file_index: usize,

    pub stashes: Vec<StashEntry>,
    pub stash_list_state: ListState,
    pub remotes: Vec<RemoteInfo>,

    pub input_buffer: String,
    /// Optional stash message (set via input mode before saving).
    pub stash_message_buffer: String,

    pub status_message: Option<String>,
    pub error_message: Option<String>,

    /// When `true`, the next `d` press actually discards; otherwise the first
    /// `d` sets this flag and shows a confirmation prompt.
    pub confirm_discard: bool,

    /// Whether the theme selection panel is visible.
    pub show_theme_panel: bool,
    /// Whether the options panel is visible.
    pub show_options_panel: bool,
    /// Currently selected theme index (0-26).
    pub current_theme_index: usize,
    /// ListState for the theme list widget.
    pub theme_list_state: ListState,

    /// Recently opened repositories loaded from persistence.
    pub recent_repos: Vec<gitkraft_core::RepoHistoryEntry>,
}

impl App {
    // ── Constructor ───────────────────────────────────────────────────────

    #[must_use]
    pub fn new() -> Self {
        let settings = gitkraft_core::features::persistence::load_settings().unwrap_or_default();

        let theme_index = theme_name_to_index(settings.theme_name.as_deref().unwrap_or(""));

        let recent_repos = settings.recent_repos;

        let (bg_tx, bg_rx) = mpsc::unbounded_channel();

        Self {
            should_quit: false,
            screen: AppScreen::Welcome,
            active_pane: ActivePane::Branches,
            input_mode: InputMode::Normal,
            input_purpose: InputPurpose::None,
            tick_count: 0,

            is_loading: false,
            bg_rx,
            bg_tx,

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

            stashes: Vec::new(),
            stash_list_state: ListState::default(),
            remotes: Vec::new(),

            input_buffer: String::new(),
            stash_message_buffer: String::new(),

            status_message: None,
            error_message: None,

            confirm_discard: false,

            show_theme_panel: false,
            show_options_panel: false,
            current_theme_index: theme_index,
            theme_list_state: {
                let mut s = ListState::default();
                s.select(Some(theme_index));
                s
            },

            recent_repos,
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
        self.status_message = Some(format!("Theme: {}", self.current_theme_name()));
    }

    pub fn cycle_theme_prev(&mut self) {
        let count = 27;
        if self.current_theme_index == 0 {
            self.current_theme_index = count - 1;
        } else {
            self.current_theme_index -= 1;
        }
        self.theme_list_state.select(Some(self.current_theme_index));
        self.status_message = Some(format!("Theme: {}", self.current_theme_name()));
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

    // ── High-level operations ────────────────────────────────────────────

    pub fn open_repo(&mut self, path: PathBuf) {
        self.error_message = None;
        self.status_message = Some("Opening repository…".into());
        self.is_loading = true;
        self.repo_path = Some(path.clone());
        self.screen = AppScreen::Main;

        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = load_repo_blocking(&path);
            let _ = tx.send(BackgroundResult::RepoLoaded(result));
        });
    }

    pub fn refresh(&mut self) {
        self.error_message = None;
        self.is_loading = true;
        self.status_message = Some("Refreshing…".into());

        let path = match self.repo_path.clone() {
            Some(p) => p,
            None => {
                self.error_message = Some("No repository open".into());
                self.is_loading = false;
                return;
            }
        };

        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = load_repo_blocking(&path);
            let _ = tx.send(BackgroundResult::RepoLoaded(result));
        });
    }

    /// Process any pending results from background tasks.
    /// Call this once per tick in the event loop.
    pub fn poll_background(&mut self) {
        while let Ok(result) = self.bg_rx.try_recv() {
            match result {
                BackgroundResult::RepoLoaded(res) => {
                    self.is_loading = false;
                    match res {
                        Ok(payload) => {
                            let canonical = payload
                                .info
                                .workdir
                                .clone()
                                .unwrap_or_else(|| self.repo_path.clone().unwrap_or_default());
                            self.repo_path = Some(canonical.clone());

                            // Persist
                            let _ = gitkraft_core::features::persistence::record_repo_opened(
                                &canonical,
                            );
                            if let Ok(settings) =
                                gitkraft_core::features::persistence::load_settings()
                            {
                                self.recent_repos = settings.recent_repos;
                            }

                            self.repo_info = Some(payload.info);
                            self.branches = payload.branches;
                            clamp_list_state(&mut self.branch_list_state, self.branches.len());
                            self.graph_rows = payload.graph_rows;
                            self.commits = payload.commits;
                            clamp_list_state(&mut self.commit_list_state, self.commits.len());
                            self.unstaged_changes = payload.unstaged;
                            clamp_list_state(
                                &mut self.unstaged_list_state,
                                self.unstaged_changes.len(),
                            );
                            self.staged_changes = payload.staged;
                            clamp_list_state(
                                &mut self.staged_list_state,
                                self.staged_changes.len(),
                            );
                            self.stashes = payload.stashes;
                            clamp_list_state(&mut self.stash_list_state, self.stashes.len());
                            self.remotes = payload.remotes;
                            self.screen = AppScreen::Main;
                            self.status_message = Some("Repository loaded".into());
                        }
                        Err(e) => {
                            self.error_message = Some(e);
                            self.status_message = None;
                        }
                    }
                }
                BackgroundResult::FetchDone(res) => {
                    self.is_loading = false;
                    match res {
                        Ok(()) => {
                            self.status_message = Some("Fetched from origin".into());
                            self.refresh();
                        }
                        Err(e) => self.error_message = Some(format!("fetch: {e}")),
                    }
                }
                BackgroundResult::CommitDiffLoaded(res) => {
                    self.is_loading = false;
                    match res {
                        Ok(diffs) => {
                            if diffs.is_empty() {
                                self.selected_diff = None;
                                self.commit_diffs.clear();
                                self.commit_diff_file_index = 0;
                                self.status_message = Some("No changes in this commit".into());
                            } else {
                                self.commit_diffs = diffs.clone();
                                self.commit_diff_file_index = 0;
                                self.selected_diff = Some(diffs[0].clone());
                                self.diff_scroll = 0;
                                if diffs.len() > 1 {
                                    self.status_message = Some(format!(
                                        "Showing file 1/{} — use h/l to switch files",
                                        diffs.len()
                                    ));
                                }
                            }
                        }
                        Err(e) => self.error_message = Some(format!("commit diff: {e}")),
                    }
                }
                BackgroundResult::StagingRefreshed(res) => {
                    self.is_loading = false;
                    match res {
                        Ok(payload) => self.apply_staging_payload(payload),
                        Err(e) => self.error_message = Some(format!("staging refresh: {e}")),
                    }
                }
                BackgroundResult::OperationDone {
                    ok_message,
                    err_message,
                    needs_refresh,
                    needs_staging_refresh,
                } => {
                    self.is_loading = false;
                    if let Some(msg) = err_message {
                        self.error_message = Some(msg);
                    } else if let Some(msg) = ok_message {
                        self.status_message = Some(msg);
                    }
                    if needs_refresh {
                        self.refresh();
                    } else if needs_staging_refresh {
                        self.refresh_staging();
                    }
                }
            }
        }
    }

    /// Reload only the staging area (unstaged + staged diffs).
    pub fn refresh_staging(&mut self) {
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => {
                self.error_message = Some("No repository open".into());
                return;
            }
        };
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
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
        self.unstaged_changes = payload.unstaged;
        if self.unstaged_changes.is_empty() {
            self.unstaged_list_state.select(None);
        } else if self.unstaged_list_state.selected().is_none() {
            self.unstaged_list_state.select(Some(0));
        } else if let Some(i) = self.unstaged_list_state.selected() {
            if i >= self.unstaged_changes.len() {
                self.unstaged_list_state
                    .select(Some(self.unstaged_changes.len() - 1));
            }
        }

        self.staged_changes = payload.staged;
        if self.staged_changes.is_empty() {
            self.staged_list_state.select(None);
        } else if self.staged_list_state.selected().is_none() {
            self.staged_list_state.select(Some(0));
        } else if let Some(i) = self.staged_list_state.selected() {
            if i >= self.staged_changes.len() {
                self.staged_list_state
                    .select(Some(self.staged_changes.len() - 1));
            }
        }
    }

    // ── Staging operations ───────────────────────────────────────────────

    pub fn stage_selected(&mut self) {
        let idx = match self.unstaged_list_state.selected() {
            Some(i) => i,
            None => {
                self.status_message = Some("No unstaged file selected".into());
                return;
            }
        };
        let file_path = self.unstaged_file_path(idx);
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::staging::stage_file(&repo, &file_path)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("Staged: {file_path}")),
                err_message: res.err().map(|e| format!("stage: {e}")),
                needs_refresh: false,
                needs_staging_refresh: true,
            });
        });
    }

    pub fn unstage_selected(&mut self) {
        let idx = match self.staged_list_state.selected() {
            Some(i) => i,
            None => {
                self.status_message = Some("No staged file selected".into());
                return;
            }
        };
        let file_path = self.staged_file_path(idx);
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::staging::unstage_file(&repo, &file_path)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| format!("Unstaged: {file_path}")),
                err_message: res.err().map(|e| format!("unstage: {e}")),
                needs_refresh: false,
                needs_staging_refresh: true,
            });
        });
    }

    pub fn stage_all(&mut self) {
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::staging::stage_all(&repo).map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| "Staged all files".into()),
                err_message: res.err().map(|e| format!("stage all: {e}")),
                needs_refresh: false,
                needs_staging_refresh: true,
            });
        });
    }

    pub fn unstage_all(&mut self) {
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::staging::unstage_all(&repo).map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().map(|_| "Unstaged all files".into()),
                err_message: res.err().map(|e| format!("unstage all: {e}")),
                needs_refresh: false,
                needs_staging_refresh: true,
            });
        });
    }

    pub fn discard_selected(&mut self) {
        let idx = match self.unstaged_list_state.selected() {
            Some(i) => i,
            None => {
                self.status_message = Some("No unstaged file selected".into());
                return;
            }
        };
        let file_path = self.unstaged_file_path(idx);
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.confirm_discard = false;
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::staging::discard_file_changes(&repo, &file_path)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Discarded changes: {file_path}")),
                err_message: res.err().map(|e| format!("discard: {e}")),
                needs_refresh: false,
                needs_staging_refresh: true,
            });
        });
    }

    // ── Commit ───────────────────────────────────────────────────────────

    pub fn create_commit(&mut self) {
        let msg = self.input_buffer.trim().to_string();
        if msg.is_empty() {
            self.error_message = Some("Commit message cannot be empty".into());
            return;
        }
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.input_buffer.clear();
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                let info = gitkraft_core::features::commits::create_commit(&repo, &msg)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(format!("Committed: {} {}", info.short_oid, info.summary))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err().map(|e| format!("commit: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    // ── Branches ─────────────────────────────────────────────────────────

    pub fn checkout_selected_branch(&mut self) {
        let idx = match self.branch_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.branches.len() {
            return;
        }
        let name = self.branches[idx].name.clone();
        if self.branches[idx].is_head {
            self.status_message = Some(format!("Already on '{name}'"));
            return;
        }
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::branches::checkout_branch(&repo, &name)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(format!("Checked out: {name}"))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err().map(|e| format!("checkout: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn create_branch(&mut self) {
        let name = self.input_buffer.trim().to_string();
        if name.is_empty() {
            self.error_message = Some("Branch name cannot be empty".into());
            return;
        }
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.input_buffer.clear();
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::branches::create_branch(&repo, &name)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(format!("Created branch: {name}"))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err().map(|e| format!("create branch: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn delete_selected_branch(&mut self) {
        let idx = match self.branch_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.branches.len() {
            return;
        }
        let branch = &self.branches[idx];
        if branch.is_head {
            self.error_message = Some("Cannot delete the current branch".into());
            return;
        }
        let name = branch.name.clone();
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::branches::delete_branch(&repo, &name)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(format!("Deleted branch: {name}"))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err().map(|e| format!("delete branch: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    // ── Stash ────────────────────────────────────────────────────────────

    pub fn stash_save(&mut self) {
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        let msg = if self.stash_message_buffer.trim().is_empty() {
            None
        } else {
            Some(self.stash_message_buffer.trim().to_string())
        };
        self.stash_message_buffer.clear();
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let mut repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                let entry = gitkraft_core::features::stash::stash_save(&mut repo, msg.as_deref())
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(format!("Stashed: {}", entry.message))
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res.as_ref().ok().cloned(),
                err_message: res.err().map(|e| format!("stash save: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn stash_pop_selected(&mut self) {
        let idx = self.stash_list_state.selected().unwrap_or(0);
        if idx >= self.stashes.len() {
            self.error_message = Some("No stash selected".into());
            return;
        }
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let mut repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::stash::stash_pop(&mut repo, idx).map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Stash @{{{idx}}} popped")),
                err_message: res.err().map(|e| format!("stash pop: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    pub fn stash_drop_selected(&mut self) {
        let idx = self.stash_list_state.selected().unwrap_or(0);
        if idx >= self.stashes.len() {
            self.error_message = Some("No stash to drop".into());
            return;
        }
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let mut repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::stash::stash_drop(&mut repo, idx)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::OperationDone {
                ok_message: res
                    .as_ref()
                    .ok()
                    .map(|_| format!("Stash @{{{idx}}} dropped")),
                err_message: res.err().map(|e| format!("stash drop: {e}")),
                needs_refresh: true,
                needs_staging_refresh: false,
            });
        });
    }

    // ── Diff ─────────────────────────────────────────────────────────────

    /// Load the diff for the currently selected commit into the diff pane.
    pub fn load_commit_diff(&mut self) {
        let idx = match self.commit_list_state.selected() {
            Some(i) => i,
            None => return,
        };
        if idx >= self.commits.len() {
            return;
        }
        let oid = self.commits[idx].oid.clone();
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        self.status_message = Some("Loading diff…".into());
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::diff::get_commit_diff(&repo, &oid)
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::CommitDiffLoaded(res));
        });
    }

    /// Switch to the next file in the commit diff list.
    pub fn next_diff_file(&mut self) {
        if self.commit_diffs.is_empty() {
            return;
        }
        self.commit_diff_file_index = (self.commit_diff_file_index + 1) % self.commit_diffs.len();
        self.selected_diff = Some(self.commit_diffs[self.commit_diff_file_index].clone());
        self.diff_scroll = 0;
        self.status_message = Some(format!(
            "File {}/{}",
            self.commit_diff_file_index + 1,
            self.commit_diffs.len()
        ));
    }

    /// Switch to the previous file in the commit diff list.
    pub fn prev_diff_file(&mut self) {
        if self.commit_diffs.is_empty() {
            return;
        }
        if self.commit_diff_file_index == 0 {
            self.commit_diff_file_index = self.commit_diffs.len() - 1;
        } else {
            self.commit_diff_file_index -= 1;
        }
        self.selected_diff = Some(self.commit_diffs[self.commit_diff_file_index].clone());
        self.diff_scroll = 0;
        self.status_message = Some(format!(
            "File {}/{}",
            self.commit_diff_file_index + 1,
            self.commit_diffs.len()
        ));
    }

    /// Close the current repository and return to the welcome screen.
    pub fn close_repo(&mut self) {
        self.repo_path = None;
        self.repo_info = None;
        self.branches.clear();
        self.branch_list_state = ListState::default();
        self.commits.clear();
        self.graph_rows.clear();
        self.commit_list_state = ListState::default();
        self.unstaged_changes.clear();
        self.staged_changes.clear();
        self.unstaged_list_state = ListState::default();
        self.staged_list_state = ListState::default();
        self.staging_focus = StagingFocus::Unstaged;
        self.selected_diff = None;
        self.commit_diffs.clear();
        self.commit_diff_file_index = 0;
        self.diff_scroll = 0;
        self.stashes.clear();
        self.stash_list_state = ListState::default();
        self.remotes.clear();
        self.input_buffer.clear();
        self.stash_message_buffer.clear();
        self.status_message = None;
        self.error_message = None;
        self.confirm_discard = false;
        self.show_theme_panel = false;
        self.show_options_panel = false;
        self.screen = AppScreen::Welcome;
        // Reload recent repos
        if let Ok(settings) = gitkraft_core::features::persistence::load_settings() {
            self.recent_repos = settings.recent_repos;
        }
    }

    /// Load the diff for a selected staging file into the diff pane.
    pub fn load_staging_diff(&mut self) {
        match self.staging_focus {
            StagingFocus::Unstaged => {
                if let Some(idx) = self.unstaged_list_state.selected() {
                    if idx < self.unstaged_changes.len() {
                        self.selected_diff = Some(self.unstaged_changes[idx].clone());
                        self.diff_scroll = 0;
                    }
                }
            }
            StagingFocus::Staged => {
                if let Some(idx) = self.staged_list_state.selected() {
                    if idx < self.staged_changes.len() {
                        self.selected_diff = Some(self.staged_changes[idx].clone());
                        self.diff_scroll = 0;
                    }
                }
            }
        }
    }

    // ── Remote ───────────────────────────────────────────────────────────

    pub fn fetch_remote(&mut self) {
        let repo_path = match self.repo_path.clone() {
            Some(p) => p,
            None => return,
        };
        self.is_loading = true;
        self.status_message = Some("Fetching…".into());
        let tx = self.bg_tx.clone();
        tokio::task::spawn_blocking(move || {
            let res = (|| {
                let repo = gitkraft_core::features::repo::open_repo(&repo_path)
                    .map_err(|e| e.to_string())?;
                gitkraft_core::features::remotes::fetch_remote(&repo, "origin")
                    .map_err(|e| e.to_string())
            })();
            let _ = tx.send(BackgroundResult::FetchDone(res));
        });
    }

    // ── Path helpers ─────────────────────────────────────────────────────

    fn unstaged_file_path(&self, idx: usize) -> String {
        if idx >= self.unstaged_changes.len() {
            return String::new();
        }
        let d = &self.unstaged_changes[idx];
        if d.new_file.is_empty() {
            d.old_file.clone()
        } else {
            d.new_file.clone()
        }
    }

    fn staged_file_path(&self, idx: usize) -> String {
        if idx >= self.staged_changes.len() {
            return String::new();
        }
        let d = &self.staged_changes[idx];
        if d.new_file.is_empty() {
            d.old_file.clone()
        } else {
            d.new_file.clone()
        }
    }
}

// ── Free-standing helpers ─────────────────────────────────────────────────────

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
    let mut repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;

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
        assert!(app.commits.is_empty());
        assert!(app.branches.is_empty());
        assert!(app.repo_path.is_none());
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
        // Default theme's active border comes from the core accent (88, 166, 255)
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
}
