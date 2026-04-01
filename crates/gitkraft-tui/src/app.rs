use std::path::PathBuf;

use anyhow::Result;
use ratatui::widgets::ListState;

use gitkraft_core::*;

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

    pub stashes: Vec<StashEntry>,
    pub remotes: Vec<RemoteInfo>,

    pub input_buffer: String,

    pub status_message: Option<String>,
    pub error_message: Option<String>,

    /// When `true`, the next `d` press actually discards; otherwise the first
    /// `d` sets this flag and shows a confirmation prompt.
    pub confirm_discard: bool,
}

impl App {
    // ── Constructor ───────────────────────────────────────────────────────

    #[must_use]
    pub fn new() -> Self {
        Self {
            should_quit: false,
            screen: AppScreen::Welcome,
            active_pane: ActivePane::Branches,
            input_mode: InputMode::Normal,
            input_purpose: InputPurpose::None,
            tick_count: 0,

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

            stashes: Vec::new(),
            remotes: Vec::new(),

            input_buffer: String::new(),

            status_message: None,
            error_message: None,

            confirm_discard: false,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    // ── Repo helpers ─────────────────────────────────────────────────────

    /// Open the repository fresh from `self.repo_path`.  We never store
    /// `git2::Repository` on `App` because it is not `Send`.
    fn open_current_repo(&self) -> Result<git2::Repository> {
        let path = self
            .repo_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No repository open"))?;
        gitkraft_core::features::repo::open_repo(path)
    }

    /// Open the repository as `&mut` (needed for stash operations).
    fn open_current_repo_mut(&self) -> Result<git2::Repository> {
        self.open_current_repo()
    }

    // ── High-level operations ────────────────────────────────────────────

    pub fn open_repo(&mut self, path: PathBuf) {
        self.error_message = None;
        self.status_message = None;

        let repo = match gitkraft_core::features::repo::open_repo(&path) {
            Ok(r) => r,
            Err(e) => {
                self.error_message = Some(format!("Failed to open repo: {e}"));
                return;
            }
        };

        match gitkraft_core::features::repo::get_repo_info(&repo) {
            Ok(info) => {
                // Use the workdir as the canonical path if available
                self.repo_path = info.workdir.clone().or(Some(path));
                self.repo_info = Some(info);
            }
            Err(e) => {
                self.repo_path = Some(path);
                self.error_message = Some(format!("Failed to read repo info: {e}"));
                return;
            }
        }

        self.screen = AppScreen::Main;
        self.refresh();
        self.status_message = Some("Repository opened".into());
    }

    pub fn refresh(&mut self) {
        self.error_message = None;

        let repo = match self.open_current_repo() {
            Ok(r) => r,
            Err(e) => {
                self.error_message = Some(format!("{e}"));
                return;
            }
        };

        // Repo info
        match gitkraft_core::features::repo::get_repo_info(&repo) {
            Ok(info) => self.repo_info = Some(info),
            Err(e) => self.error_message = Some(format!("repo info: {e}")),
        }

        // Branches
        match gitkraft_core::features::branches::list_branches(&repo) {
            Ok(b) => {
                self.branches = b;
                // Clamp selection
                if self.branches.is_empty() {
                    self.branch_list_state.select(None);
                } else if self.branch_list_state.selected().is_none() {
                    self.branch_list_state.select(Some(0));
                } else if let Some(i) = self.branch_list_state.selected() {
                    if i >= self.branches.len() {
                        self.branch_list_state.select(Some(self.branches.len() - 1));
                    }
                }
            }
            Err(e) => self.error_message = Some(format!("branches: {e}")),
        }

        // Commits + graph
        match gitkraft_core::features::commits::list_commits(&repo, 200) {
            Ok(c) => {
                self.graph_rows = gitkraft_core::features::graph::build_graph(&c);
                self.commits = c;
                if self.commits.is_empty() {
                    self.commit_list_state.select(None);
                } else if self.commit_list_state.selected().is_none() {
                    self.commit_list_state.select(Some(0));
                } else if let Some(i) = self.commit_list_state.selected() {
                    if i >= self.commits.len() {
                        self.commit_list_state.select(Some(self.commits.len() - 1));
                    }
                }
            }
            Err(_e) => {
                // Empty repo has no commits — that's fine
                self.commits.clear();
                self.graph_rows.clear();
                self.commit_list_state.select(None);
            }
        }

        // Staging
        self.refresh_staging_inner(&repo);

        // Stashes (needs mut)
        drop(repo);
        match self.open_current_repo_mut() {
            Ok(mut r) => match gitkraft_core::features::stash::list_stashes(&mut r) {
                Ok(s) => self.stashes = s,
                Err(_) => self.stashes.clear(),
            },
            Err(_) => self.stashes.clear(),
        }

        // Remotes
        match self.open_current_repo() {
            Ok(r) => match gitkraft_core::features::remotes::list_remotes(&r) {
                Ok(rem) => self.remotes = rem,
                Err(_) => self.remotes.clear(),
            },
            Err(_) => self.remotes.clear(),
        }

        if self.error_message.is_none() {
            self.status_message = Some("Refreshed".into());
        }
    }

    /// Reload only the staging area (unstaged + staged diffs).
    pub fn refresh_staging(&mut self) {
        match self.open_current_repo() {
            Ok(repo) => self.refresh_staging_inner(&repo),
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    fn refresh_staging_inner(&mut self, repo: &git2::Repository) {
        // Unstaged
        match gitkraft_core::features::diff::get_working_dir_diff(repo) {
            Ok(d) => {
                self.unstaged_changes = d;
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
            }
            Err(e) => {
                self.error_message = Some(format!("unstaged diff: {e}"));
                self.unstaged_changes.clear();
            }
        }

        // Staged
        match gitkraft_core::features::diff::get_staged_diff(repo) {
            Ok(d) => {
                self.staged_changes = d;
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
            Err(e) => {
                self.error_message = Some(format!("staged diff: {e}"));
                self.staged_changes.clear();
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
        let path = self.unstaged_file_path(idx);
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::staging::stage_file(&repo, &path) {
                Ok(()) => {
                    self.status_message = Some(format!("Staged: {path}"));
                    self.refresh_staging();
                }
                Err(e) => self.error_message = Some(format!("stage: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn unstage_selected(&mut self) {
        let idx = match self.staged_list_state.selected() {
            Some(i) => i,
            None => {
                self.status_message = Some("No staged file selected".into());
                return;
            }
        };
        let path = self.staged_file_path(idx);
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::staging::unstage_file(&repo, &path) {
                Ok(()) => {
                    self.status_message = Some(format!("Unstaged: {path}"));
                    self.refresh_staging();
                }
                Err(e) => self.error_message = Some(format!("unstage: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn stage_all(&mut self) {
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::staging::stage_all(&repo) {
                Ok(()) => {
                    self.status_message = Some("Staged all files".into());
                    self.refresh_staging();
                }
                Err(e) => self.error_message = Some(format!("stage all: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn unstage_all(&mut self) {
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::staging::unstage_all(&repo) {
                Ok(()) => {
                    self.status_message = Some("Unstaged all files".into());
                    self.refresh_staging();
                }
                Err(e) => self.error_message = Some(format!("unstage all: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn discard_selected(&mut self) {
        let idx = match self.unstaged_list_state.selected() {
            Some(i) => i,
            None => {
                self.status_message = Some("No unstaged file selected".into());
                return;
            }
        };
        let path = self.unstaged_file_path(idx);
        match self.open_current_repo() {
            Ok(repo) => {
                match gitkraft_core::features::staging::discard_file_changes(&repo, &path) {
                    Ok(()) => {
                        self.status_message = Some(format!("Discarded changes: {path}"));
                        self.confirm_discard = false;
                        self.refresh_staging();
                    }
                    Err(e) => self.error_message = Some(format!("discard: {e}")),
                }
            }
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    // ── Commit ───────────────────────────────────────────────────────────

    pub fn create_commit(&mut self) {
        let msg = self.input_buffer.trim().to_string();
        if msg.is_empty() {
            self.error_message = Some("Commit message cannot be empty".into());
            return;
        }
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::commits::create_commit(&repo, &msg) {
                Ok(info) => {
                    self.status_message =
                        Some(format!("Committed: {} {}", info.short_oid, info.summary));
                    self.input_buffer.clear();
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("commit: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
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
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::branches::checkout_branch(&repo, &name) {
                Ok(()) => {
                    self.status_message = Some(format!("Checked out: {name}"));
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("checkout: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn create_branch(&mut self) {
        let name = self.input_buffer.trim().to_string();
        if name.is_empty() {
            self.error_message = Some("Branch name cannot be empty".into());
            return;
        }
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::branches::create_branch(&repo, &name) {
                Ok(_info) => {
                    self.status_message = Some(format!("Created branch: {name}"));
                    self.input_buffer.clear();
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("create branch: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
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
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::branches::delete_branch(&repo, &name) {
                Ok(()) => {
                    self.status_message = Some(format!("Deleted branch: {name}"));
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("delete branch: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    // ── Stash ────────────────────────────────────────────────────────────

    pub fn stash_save(&mut self) {
        match self.open_current_repo_mut() {
            Ok(mut repo) => match gitkraft_core::features::stash::stash_save(&mut repo, None) {
                Ok(entry) => {
                    self.status_message = Some(format!("Stashed: {}", entry.message));
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("stash save: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn stash_pop_selected(&mut self) {
        // Always pop the most recent stash (index 0)
        let index: usize = 0;
        match self.open_current_repo_mut() {
            Ok(mut repo) => match gitkraft_core::features::stash::stash_pop(&mut repo, index) {
                Ok(()) => {
                    self.status_message = Some("Stash popped".into());
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("stash pop: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
    }

    pub fn stash_drop_selected(&mut self) {
        let index = if self.stashes.is_empty() {
            self.error_message = Some("No stashes to drop".into());
            return;
        } else {
            0
        };
        match self.open_current_repo_mut() {
            Ok(mut repo) => match gitkraft_core::features::stash::stash_drop(&mut repo, index) {
                Ok(()) => {
                    self.status_message = Some("Stash dropped".into());
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("stash drop: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
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
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::diff::get_commit_diff(&repo, &oid) {
                Ok(diffs) => {
                    // Merge all file diffs into a single synthetic DiffInfo for
                    // display purposes.
                    if diffs.is_empty() {
                        self.selected_diff = None;
                        self.status_message = Some("No changes in this commit".into());
                    } else {
                        self.selected_diff = Some(merge_diffs(&diffs));
                        self.diff_scroll = 0;
                    }
                }
                Err(e) => self.error_message = Some(format!("commit diff: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
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
        match self.open_current_repo() {
            Ok(repo) => match gitkraft_core::features::remotes::fetch_remote(&repo, "origin") {
                Ok(()) => {
                    self.status_message = Some("Fetched from origin".into());
                    self.refresh();
                }
                Err(e) => self.error_message = Some(format!("fetch: {e}")),
            },
            Err(e) => self.error_message = Some(format!("{e}")),
        }
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

/// Merge multiple per-file `DiffInfo`s into one combined view for the diff pane.
fn merge_diffs(diffs: &[DiffInfo]) -> DiffInfo {
    use gitkraft_core::{DiffHunk, DiffLine, FileStatus};

    let mut hunks = Vec::new();
    for d in diffs {
        let file_name = if d.new_file.is_empty() {
            &d.old_file
        } else {
            &d.new_file
        };
        // Add a synthetic hunk header for the file
        hunks.push(DiffHunk {
            header: format!("── {} ({}) ──", file_name, d.status),
            lines: vec![DiffLine::HunkHeader(format!(
                "── {} ({}) ──",
                file_name, d.status
            ))],
        });
        for h in &d.hunks {
            hunks.push(h.clone());
        }
    }

    DiffInfo {
        old_file: String::new(),
        new_file: if diffs.len() == 1 {
            let d = &diffs[0];
            if d.new_file.is_empty() {
                d.old_file.clone()
            } else {
                d.new_file.clone()
            }
        } else {
            format!("{} files", diffs.len())
        },
        status: FileStatus::Modified,
        hunks,
    }
}
