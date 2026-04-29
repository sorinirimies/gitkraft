//! Top-level update function for the GitKraft application.
//!
//! Matches on each [`Message`] variant and delegates to the appropriate
//! feature's update handler. Each feature handler receives `&mut GitKraft`
//! and the message, and returns a `Task<Message>` for any follow-up async work.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

impl GitKraft {
    /// The single entry-point for all application updates. Iced calls this
    /// whenever a [`Message`] is produced (by user interaction or an async
    /// task completing).
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match &message {
            // ── Tabs ──────────────────────────────────────────────────────
            Message::SwitchTab(index) => {
                let index = *index;
                if index < self.tabs.len() {
                    self.active_tab = index;
                }
                Task::none()
            }

            Message::NewTab => {
                self.tabs.push(crate::state::RepoTab::new_empty());
                self.active_tab = self.tabs.len() - 1;
                // Refresh recent repos so the welcome screen is up to date.
                crate::features::repo::commands::load_recent_repos_async()
            }

            Message::CloseTab(index) => {
                let index = *index;
                if self.tabs.len() > 1 && index < self.tabs.len() {
                    self.tabs.remove(index);
                    // Adjust active_tab if needed.
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    } else if self.active_tab > index {
                        self.active_tab -= 1;
                    }
                }
                let open_tabs = self.open_tab_paths();
                let active = self.active_tab;
                crate::features::repo::commands::save_session_async(open_tabs, active)
            }

            // ── Repository ────────────────────────────────────────────────
            Message::OpenRepo
            | Message::InitRepo
            | Message::RepoSelected(_)
            | Message::RepoInitSelected(_)
            | Message::RepoOpened(_)
            | Message::RefreshRepo
            | Message::RepoRefreshed(_)
            | Message::OpenRecentRepo(_)
            | Message::CloseRepo
            | Message::RepoRecorded(_)
            | Message::RepoRestoredAt(_, _)
            | Message::MoreCommitsLoaded(_)
            | Message::SettingsLoaded(_)
            | Message::GitOperationResult(_) => {
                crate::features::repo::update::update(self, message)
            }

            // ── Branches ──────────────────────────────────────────────────
            Message::CheckoutBranch(_)
            | Message::BranchCheckedOut(_)
            | Message::CreateBranch
            | Message::NewBranchNameChanged(_)
            | Message::BranchCreated(_)
            | Message::DeleteBranch(_)
            | Message::BranchDeleted(_)
            | Message::ToggleBranchCreate
            | Message::ToggleLocalBranches
            | Message::ToggleRemoteBranches => {
                crate::features::branches::update::update(self, message)
            }

            // ── Commits ───────────────────────────────────────────────────
            Message::SelectCommit(_)
            | Message::CommitFileListLoaded(_)
            | Message::SingleFileDiffLoaded(_)
            | Message::DiffFileWithWorkingTree(_, _)
            | Message::DiffWithWorkingTreeLoaded(_)
            | Message::DiffMultiWithWorkingTree(_, _)
            | Message::CheckoutFileAtCommit(_, _)
            | Message::CheckoutMultiFilesAtCommit(_, _)
            | Message::CommitRangeDiffLoaded(_) => {
                // Both the commits and diff features care about SelectCommit.
                // We delegate to the commits handler which also loads the diff.
                crate::features::commits::update::update(self, message)
            }

            Message::CommitMessageChanged(_)
            | Message::CreateCommit
            | Message::CommitCreated(_) => crate::features::commits::update::update(self, message),

            Message::CommitLogScrolled(abs_y, rel_y) => {
                // relative_y is 0.0 at the top and 1.0 at the very bottom of
                // the scrollable content.  Using it (rather than absolute_y)
                // avoids needing to know the viewport height.
                const COMMITS_PAGE_SIZE: usize = 200;
                // Trigger a load when the user is in the last 15 % of the
                // scrollable area — roughly 2–3 screen-heights from the end.
                const LOAD_TRIGGER_RELATIVE: f32 = 0.85;

                self.active_tab_mut().commit_scroll_offset = *abs_y;

                let tab = self.active_tab();
                if *rel_y >= LOAD_TRIGGER_RELATIVE
                    && tab.has_more_commits
                    && !tab.is_loading_more_commits
                {
                    if let Some(path) = tab.repo_path.clone() {
                        let current = tab.commits.len();
                        self.active_tab_mut().is_loading_more_commits = true;
                        return crate::features::repo::commands::load_more_commits(
                            path,
                            current,
                            COMMITS_PAGE_SIZE,
                        );
                    }
                }
                Task::none()
            }

            Message::DiffViewScrolled(abs_y) => {
                self.active_tab_mut().diff_scroll_offset = *abs_y;
                Task::none()
            }

            // ── Staging ───────────────────────────────────────────────────
            Message::StageFile(_)
            | Message::UnstageFile(_)
            | Message::StageAll
            | Message::UnstageAll
            | Message::DiscardFile(_)
            | Message::ConfirmDiscard(_)
            | Message::CancelDiscard
            | Message::StagingUpdated(_)
            | Message::ToggleSelectUnstaged(_)
            | Message::ToggleSelectStaged(_)
            | Message::StageSelected
            | Message::UnstageSelected
            | Message::DiscardSelected
            | Message::DiscardStagedFile(_) => {
                crate::features::staging::update::update(self, message)
            }

            // ── Stash ─────────────────────────────────────────────────────
            Message::StashSave
            | Message::StashPop(_)
            | Message::StashDrop(_)
            | Message::StashUpdated(_)
            | Message::StashMessageChanged(_)
            | Message::StashApply(_)
            | Message::ViewStashDiff(_)
            | Message::StashDiffLoaded(_) => crate::features::stash::update::update(self, message),

            // ── Remotes ───────────────────────────────────────────────────
            Message::Fetch | Message::FetchCompleted(_) => {
                crate::features::remotes::update::update(self, message)
            }

            // ── UI / misc ─────────────────────────────────────────────────
            Message::SelectDiff(_)
            | Message::SelectDiffByIndex(_)
            | Message::CommitMultiDiffLoaded(_) => {
                crate::features::diff::update::update(self, message)
            }

            Message::ModifiersChanged(mods) => {
                self.keyboard_modifiers = *mods;
                Task::none()
            }

            Message::DismissError => {
                self.active_tab_mut().error_message = None;
                Task::none()
            }

            Message::ZoomIn => {
                self.ui_scale = (self.ui_scale + 0.1).min(2.0);
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::ZoomOut => {
                self.ui_scale = (self.ui_scale - 0.1).max(0.5);
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::ZoomReset => {
                self.ui_scale = 1.0;
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::AnimationTick => {
                self.animation_tick = self.animation_tick.wrapping_add(1);
                Task::none()
            }

            Message::ShiftArrowDown => {
                // Priority 1: file list (if commit files are loaded and a file is selected).
                let file_info: Option<(usize, usize)> = {
                    let tab = self.active_tab();
                    if !tab.commit_files.is_empty() {
                        tab.selected_file_index
                            .map(|cur| (cur, tab.commit_files.len()))
                    } else {
                        None
                    }
                };
                if let Some((current, files_len)) = file_info {
                    let new_idx = (current + 1).min(files_len.saturating_sub(1));
                    if new_idx != current {
                        return crate::features::diff::update::update(
                            self,
                            Message::SelectDiffByIndex(new_idx),
                        );
                    }
                    return Task::none();
                }
                // Priority 2: commit log.
                let commit_info: Option<(usize, usize)> = {
                    let tab = self.active_tab();
                    if !tab.commits.is_empty() {
                        Some((tab.selected_commit.unwrap_or(0), tab.commits.len()))
                    } else {
                        None
                    }
                };
                if let Some((current, commits_len)) = commit_info {
                    let new_idx = (current + 1).min(commits_len.saturating_sub(1));
                    if new_idx != current {
                        return crate::features::commits::update::update(
                            self,
                            Message::SelectCommit(new_idx),
                        );
                    }
                }
                Task::none()
            }

            Message::ShiftArrowUp => {
                // Priority 1: file list (if commit files are loaded and a file is selected).
                let file_info: Option<(usize, usize)> = {
                    let tab = self.active_tab();
                    if !tab.commit_files.is_empty() {
                        tab.selected_file_index
                            .map(|cur| (cur, tab.commit_files.len()))
                    } else {
                        None
                    }
                };
                if let Some((current, _files_len)) = file_info {
                    let new_idx = current.saturating_sub(1);
                    if new_idx != current {
                        return crate::features::diff::update::update(
                            self,
                            Message::SelectDiffByIndex(new_idx),
                        );
                    }
                    return Task::none();
                }
                // Priority 2: commit log.
                let commit_info: Option<(usize, usize)> = {
                    let tab = self.active_tab();
                    if !tab.commits.is_empty() {
                        Some((tab.selected_commit.unwrap_or(0), tab.commits.len()))
                    } else {
                        None
                    }
                };
                if let Some((current, _commits_len)) = commit_info {
                    let new_idx = current.saturating_sub(1);
                    if new_idx != current {
                        return crate::features::commits::update::update(
                            self,
                            Message::SelectCommit(new_idx),
                        );
                    }
                }
                Task::none()
            }

            Message::ToggleSidebar => {
                self.sidebar_expanded = !self.sidebar_expanded;
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::WindowResized(w, h) => {
                let w = *w;
                let h = *h;
                self.window_width = w;
                self.window_height = h;
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::WindowMoved(x, y) => {
                let x = *x;
                let y = *y;
                self.window_x = x;
                self.window_y = y;
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            Message::OpenSettingsFile => {
                // Resolve the settings file path.
                let path = match gitkraft_core::features::persistence::ops::settings_json_path() {
                    Ok(p) => p,
                    Err(e) => {
                        let msg = format!("Cannot determine settings path: {e}");
                        self.active_tab_mut().error_message = Some(msg);
                        return Task::none();
                    }
                };

                // Ensure the file exists so the editor can open it immediately.
                if !path.exists() {
                    let snap = gitkraft_core::features::persistence::ops::load_settings()
                        .unwrap_or_default();
                    if let Err(e) = gitkraft_core::features::persistence::ops::save_settings(&snap)
                    {
                        let msg = format!("Could not create settings file: {e}");
                        self.active_tab_mut().error_message = Some(msg);
                        return Task::none();
                    }
                }

                let path_str = path.display().to_string();

                // open_file_or_default tries the configured editor first, then
                // falls back to the system default opener (xdg-open / open /
                // start).  On macOS, GUI editors are activated via `open -a`
                // so the existing window is brought to the front.
                match self.editor.open_file_or_default(&path) {
                    Ok(method) => {
                        let msg = format!("Settings opened in {method} — {path_str}");
                        self.active_tab_mut().status_message = Some(msg);
                    }
                    Err(e) => {
                        // Opening failed entirely — show the path so the user
                        // can find and open the file manually.
                        let msg = format!("Could not open settings ({e}) — file is at: {path_str}");
                        self.active_tab_mut().error_message = Some(msg);
                    }
                }
                Task::none()
            }

            // ── Pane resize ───────────────────────────────────────────────
            Message::PaneDragStart(target, _x) => {
                self.dragging = Some(*target);
                // Position is 0.0 because `on_press` doesn't provide coords.
                // We set drag_initialized to false so the first `PaneDragMove`
                // captures the real position instead of computing a bogus delta.
                self.drag_initialized = false;
                Task::none()
            }

            Message::PaneDragStartH(target, _y) => {
                self.dragging_h = Some(*target);
                self.drag_initialized_h = false;
                Task::none()
            }

            Message::PaneDragMove(x, y) => {
                use crate::state::{DragTarget, DragTargetH};

                // Always record cursor position so context menus open at the pointer.
                self.cursor_pos = iced::Point::new(*x, *y);

                if let Some(target) = self.dragging {
                    if !self.drag_initialized {
                        // First move after press — just record the position.
                        self.drag_start_x = *x;
                        self.drag_initialized = true;
                    } else {
                        let dx = *x - self.drag_start_x;
                        self.drag_start_x = *x;

                        match target {
                            DragTarget::SidebarRight => {
                                self.sidebar_width = (self.sidebar_width + dx).clamp(120.0, 500.0);
                            }
                            DragTarget::CommitLogRight => {
                                self.commit_log_width =
                                    (self.commit_log_width + dx).clamp(200.0, 1200.0);
                            }
                            DragTarget::DiffFileListRight => {
                                self.diff_file_list_width =
                                    (self.diff_file_list_width + dx).clamp(100.0, 400.0);
                            }
                        }
                    }
                }

                if let Some(target_h) = self.dragging_h {
                    if !self.drag_initialized_h {
                        self.drag_start_y = *y;
                        self.drag_initialized_h = true;
                    } else {
                        let dy = *y - self.drag_start_y;
                        self.drag_start_y = *y;

                        match target_h {
                            DragTargetH::StagingTop => {
                                // Dragging up → larger staging area (subtract dy).
                                self.staging_height =
                                    (self.staging_height - dy).clamp(100.0, 600.0);
                            }
                        }
                    }
                }

                Task::none()
            }

            Message::PaneDragEnd => {
                self.dragging = None;
                self.dragging_h = None;
                self.drag_initialized = false;
                self.drag_initialized_h = false;
                crate::features::repo::commands::save_layout_async(self.current_layout())
            }

            // ── Context menu lifecycle ────────────────────────────────────────────────
            Message::OpenBranchContextMenu(name, local_index, is_current) => {
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu = Some(crate::state::ContextMenu::Branch {
                    name: name.clone(),
                    is_current: *is_current,
                    local_index: *local_index,
                });
                Task::none()
            }

            Message::OpenRemoteBranchContextMenu(name) => {
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu =
                    Some(crate::state::ContextMenu::RemoteBranch { name: name.clone() });
                Task::none()
            }

            Message::OpenCommitFileContextMenu(oid, file_path) => {
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu = Some(crate::state::ContextMenu::CommitFile {
                    oid: oid.clone(),
                    file_path: file_path.clone(),
                });
                Task::none()
            }

            Message::OpenStashContextMenu(index) => {
                let index = *index;
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu = Some(crate::state::ContextMenu::Stash { index });
                Task::none()
            }

            Message::OpenUnstagedFileContextMenu(path) => {
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu =
                    Some(crate::state::ContextMenu::UnstagedFile { path: path.clone() });
                Task::none()
            }

            Message::OpenStagedFileContextMenu(path) => {
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                let tab = self.active_tab_mut();
                tab.context_menu_pos = pos;
                tab.context_menu =
                    Some(crate::state::ContextMenu::StagedFile { path: path.clone() });
                Task::none()
            }

            Message::OpenCommitContextMenu(idx) => {
                let oid = self.active_tab().commits.get(*idx).map(|c| c.oid.clone());
                let pos = (self.cursor_pos.x, self.cursor_pos.y);
                if let Some(oid) = oid {
                    let tab = self.active_tab_mut();
                    tab.context_menu_pos = pos;
                    tab.context_menu = Some(crate::state::ContextMenu::Commit { index: *idx, oid });
                }
                Task::none()
            }

            Message::OpenSearchResultContextMenu(idx) => {
                if let Some(commit) = self.search_results.get(*idx) {
                    let oid = commit.oid.clone();
                    let pos = (self.cursor_pos.x, self.cursor_pos.y);
                    let tab = self.active_tab_mut();
                    tab.context_menu_pos = pos;
                    tab.context_menu = Some(crate::state::ContextMenu::Commit { index: *idx, oid });
                }
                Task::none()
            }

            Message::CloseContextMenu => {
                self.active_tab_mut().context_menu = None;
                Task::none()
            }

            // ── Inline branch rename ──────────────────────────────────────────────────
            Message::BeginRenameBranch(name) => {
                let tab = self.active_tab_mut();
                tab.context_menu = None;
                tab.rename_branch_input = name.clone();
                tab.rename_branch_target = Some(name.clone());
                Task::none()
            }

            Message::RenameBranchInputChanged(s) => {
                self.active_tab_mut().rename_branch_input = s.clone();
                Task::none()
            }

            Message::CancelRename => {
                let tab = self.active_tab_mut();
                tab.rename_branch_target = None;
                tab.rename_branch_input.clear();
                Task::none()
            }

            Message::ConfirmRenameBranch => {
                let (original, new_name, path) = {
                    let tab = self.active_tab();
                    (
                        tab.rename_branch_target.clone(),
                        tab.rename_branch_input.trim().to_string(),
                        tab.repo_path.clone(),
                    )
                };
                if let (Some(orig), false) = (&original, new_name.is_empty()) {
                    if *orig != new_name {
                        if let Some(path) = path {
                            let orig = orig.clone();
                            {
                                let tab = self.active_tab_mut();
                                tab.rename_branch_target = None;
                                tab.rename_branch_input.clear();
                                tab.is_loading = true;
                                tab.status_message =
                                    Some(format!("Renaming '{orig}' → '{new_name}'…"));
                            }
                            return crate::features::repo::commands::rename_branch_async(
                                path, orig, new_name,
                            );
                        }
                    }
                }
                self.active_tab_mut().rename_branch_target = None;
                Task::none()
            }

            // ── Branch context menu actions ───────────────────────────────────────────
            Message::PushBranch(name) => {
                let name = name.clone();
                let remote = self
                    .active_tab()
                    .remotes
                    .first()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "origin".to_string());
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Pushing '{name}' to {remote}…"),
                    |path| crate::features::repo::commands::push_branch_async(path, name, remote)
                )
            }

            Message::PullBranch(_name) => {
                let remote = self
                    .active_tab()
                    .remotes
                    .first()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "origin".to_string());
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Pulling from {remote} (rebase)…"),
                    |path| crate::features::repo::commands::pull_rebase_async(path, remote)
                )
            }

            Message::RebaseOnto(target) => {
                let target = target.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Rebasing onto '{target}'…"),
                    |path| crate::features::repo::commands::rebase_onto_async(path, target)
                )
            }

            Message::MergeBranch(name) => {
                let name = name.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Merging '{name}' into current branch…"),
                    |path| crate::features::repo::commands::merge_branch_async(path, name)
                )
            }

            Message::CheckoutRemoteBranch(name) => {
                let name = name.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("Checking out '{name}'…"), |path| {
                    crate::features::repo::commands::checkout_remote_branch_async(path, name)
                })
            }

            Message::DeleteRemoteBranch(name) => {
                let name = name.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Deleting remote branch '{name}'…"),
                    |path| crate::features::repo::commands::delete_remote_branch_async(path, name)
                )
            }

            Message::BeginCreateTag(oid, annotated) => {
                let tab = self.active_tab_mut();
                tab.context_menu = None;
                tab.create_tag_target_oid = Some(oid.clone());
                tab.create_tag_annotated = *annotated;
                tab.create_tag_name.clear();
                tab.create_tag_message.clear();
                Task::none()
            }

            Message::TagNameChanged(s) => {
                self.active_tab_mut().create_tag_name = s.clone();
                Task::none()
            }

            Message::TagMessageChanged(s) => {
                self.active_tab_mut().create_tag_message = s.clone();
                Task::none()
            }

            Message::ConfirmCreateTag => {
                let (oid, name, message, annotated, path) = {
                    let tab = self.active_tab();
                    (
                        tab.create_tag_target_oid.clone(),
                        tab.create_tag_name.trim().to_string(),
                        tab.create_tag_message.trim().to_string(),
                        tab.create_tag_annotated,
                        tab.repo_path.clone(),
                    )
                };
                if let (Some(oid), false) = (&oid, name.is_empty()) {
                    if let Some(path) = path {
                        let oid = oid.clone();
                        {
                            let tab = self.active_tab_mut();
                            tab.create_tag_target_oid = None;
                            tab.create_tag_name.clear();
                            tab.create_tag_message.clear();
                            tab.is_loading = true;
                            tab.status_message = Some(format!("Creating tag '{name}'…"));
                        }
                        return if annotated {
                            crate::features::repo::commands::create_annotated_tag_async(
                                path, name, message, oid,
                            )
                        } else {
                            crate::features::repo::commands::create_tag_async(path, name, oid)
                        };
                    }
                }
                Task::none()
            }

            Message::CancelCreateTag => {
                let tab = self.active_tab_mut();
                tab.create_tag_target_oid = None;
                tab.create_tag_name.clear();
                tab.create_tag_message.clear();
                Task::none()
            }

            Message::CherryPickCommit(oid) => {
                let oid = oid.clone();
                let short = gitkraft_core::utils::short_oid_str(&oid).to_string();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("Cherry-picking {short}…"), |path| {
                    crate::features::repo::commands::cherry_pick_async(path, oid)
                })
            }

            Message::BeginCreateBranchAtCommit(oid) => {
                let tab = self.active_tab_mut();
                tab.context_menu = None;
                tab.create_branch_at_oid = Some(oid.clone());
                tab.new_branch_name.clear();
                Task::none()
            }

            Message::ConfirmCreateBranchAtCommit => {
                let (oid, name, path) = {
                    let tab = self.active_tab();
                    (
                        tab.create_branch_at_oid.clone(),
                        tab.new_branch_name.trim().to_string(),
                        tab.repo_path.clone(),
                    )
                };
                if let (Some(oid), false) = (&oid, name.is_empty()) {
                    if let Some(path) = path {
                        let oid = oid.clone();
                        {
                            let tab = self.active_tab_mut();
                            tab.create_branch_at_oid = None;
                            tab.new_branch_name.clear();
                            tab.is_loading = true;
                            tab.status_message = Some(format!("Creating branch '{name}'…"));
                        }
                        return crate::features::repo::commands::create_branch_at_commit_async(
                            path, name, oid,
                        );
                    }
                }
                Task::none()
            }

            Message::CancelCreateBranchAtCommit => {
                let tab = self.active_tab_mut();
                tab.create_branch_at_oid = None;
                tab.new_branch_name.clear();
                Task::none()
            }

            // ── Commit context menu actions ───────────────────────────────────────
            Message::ExecuteCommitAction(oid, action) => {
                let oid = oid.clone();
                let action = action.clone();
                let label = action.label();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("{label}…"), |path| {
                    crate::features::repo::commands::execute_commit_action_async(path, oid, action)
                })
            }

            Message::CheckoutCommitDetached(oid) => {
                let oid = oid.clone();
                let short = gitkraft_core::utils::short_oid_str(&oid).to_string();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("Checking out {short}…"), |path| {
                    crate::features::repo::commands::checkout_commit_async(path, oid)
                })
            }

            Message::RebaseOntoCommit(oid) => {
                let oid = oid.clone();
                let short = gitkraft_core::utils::short_oid_str(&oid).to_string();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("Rebasing onto {short}…"), |path| {
                    crate::features::repo::commands::rebase_onto_async(path, oid)
                })
            }

            Message::RevertCommit(oid) => {
                let oid = oid.clone();
                let short = gitkraft_core::utils::short_oid_str(&oid).to_string();
                self.active_tab_mut().context_menu = None;
                with_repo!(self, loading, format!("Reverting {short}…"), |path| {
                    crate::features::repo::commands::revert_commit_async(path, oid)
                })
            }

            Message::ResetSoft(ref oid)
            | Message::ResetMixed(ref oid)
            | Message::ResetHard(ref oid) => {
                let mode = match &message {
                    Message::ResetSoft(_) => "soft",
                    Message::ResetMixed(_) => "mixed",
                    Message::ResetHard(_) => "hard",
                    _ => unreachable!(),
                };
                let oid = oid.clone();
                let short = gitkraft_core::utils::short_oid_str(&oid).to_string();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Resetting ({mode}) to {short}…"),
                    |path| crate::features::repo::commands::reset_to_commit_async(
                        path,
                        oid,
                        mode.to_string()
                    )
                )
            }

            // ── Shared ───────────────────────────────────────────────────────────────
            Message::CopyText(text) => {
                self.active_tab_mut().context_menu = None;
                iced::clipboard::write(text.clone())
            }

            // ── Persistence / misc ────────────────────────────────────────
            Message::ThemeChanged(index) => {
                self.current_theme_index = *index;
                // Persist the selected theme name on a background thread.
                let name = gitkraft_core::THEME_NAMES
                    .get(*index)
                    .copied()
                    .unwrap_or("Default");
                crate::features::repo::commands::save_theme_async(name.to_string())
            }

            Message::ThemeSaved(_result) => {
                // Fire-and-forget — errors are silently ignored.
                Task::none()
            }

            Message::EditorChanged(editor) => {
                self.editor = editor.clone();
                self.active_tab_mut().status_message =
                    Some(format!("Editor set to {}", self.editor));
                // Persist the editor choice
                let name = self.editor.display_name().to_string();
                crate::features::repo::commands::save_editor_async(name)
            }

            Message::EditorSaved(_result) => {
                // Fire-and-forget — errors are silently ignored.
                Task::none()
            }

            Message::LayoutSaved(_result) => {
                // Fire-and-forget — errors are silently ignored.
                Task::none()
            }

            Message::SessionSaved(_) => {
                // Fire-and-forget — errors are silently ignored.
                Task::none()
            }

            Message::LayoutLoaded(result) => {
                if let Ok(Some(layout)) = result {
                    if let Some(w) = layout.sidebar_width {
                        self.sidebar_width = w;
                    }
                    if let Some(w) = layout.commit_log_width {
                        self.commit_log_width = w;
                    }
                    if let Some(h) = layout.staging_height {
                        self.staging_height = h;
                    }
                    if let Some(w) = layout.diff_file_list_width {
                        self.diff_file_list_width = w;
                    }
                    if let Some(expanded) = layout.sidebar_expanded {
                        self.sidebar_expanded = expanded;
                    }
                    if let Some(scale) = layout.ui_scale {
                        self.ui_scale = scale.clamp(0.5, 2.0);
                    }
                }
                Task::none()
            }

            // ── Search ────────────────────────────────────────────────────
            Message::ToggleSearch => {
                self.search_visible = !self.search_visible;
                if !self.search_visible {
                    self.search_query.clear();
                    self.search_results.clear();
                    self.search_selected = None;
                    self.search_diff_files.clear();
                    self.search_diff_selected.clear();
                    self.search_diff_content.clear();
                    self.search_diff_oid = None;
                    Task::none()
                } else {
                    iced::widget::operation::focus_next()
                }
            }

            Message::SearchQueryChanged(query) => {
                let query = query.clone();
                self.search_query = query.clone();
                if query.trim().len() >= 2 {
                    if let Some(path) = self.active_tab().repo_path.clone() {
                        return crate::features::commits::commands::search_commits(path, query);
                    }
                } else {
                    self.search_results.clear();
                    self.search_selected = None;
                }
                Task::none()
            }

            Message::SearchResultsLoaded(result) => {
                match result {
                    Ok(results) => {
                        self.search_results = results.clone();
                        self.search_selected = if self.search_results.is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                    }
                    Err(e) => {
                        self.search_results.clear();
                        self.active_tab_mut().error_message = Some(format!("Search failed: {e}"));
                    }
                }
                Task::none()
            }

            Message::SelectSearchResult(index) => {
                let index = *index;
                if index < self.search_results.len() {
                    self.search_selected = Some(index);
                }
                Task::none()
            }

            Message::ConfirmSearchResult => {
                if let Some(idx) = self.search_selected {
                    if let Some(commit) = self.search_results.get(idx).cloned() {
                        let oid = commit.oid.clone();
                        // Keep search open — load the file list for commit vs working tree
                        self.search_diff_oid = Some(oid.clone());
                        self.search_diff_files.clear();
                        self.search_diff_selected.clear();
                        self.search_diff_content.clear();

                        if let Some(path) = self.active_tab().repo_path.clone() {
                            return crate::features::commits::commands::search_diff_file_list(
                                path, oid,
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::SearchDiffFilesLoaded(result) => {
                match result {
                    Ok(files) => {
                        self.search_diff_files = files.clone();
                        self.search_diff_selected.clear();
                        self.search_diff_content.clear();
                    }
                    Err(e) => {
                        self.active_tab_mut().error_message =
                            Some(format!("Failed to load diff files: {e}"));
                    }
                }
                Task::none()
            }

            Message::ToggleSearchDiffFile(index) => {
                let index = *index;
                if self.search_diff_selected.contains(&index) {
                    self.search_diff_selected.remove(&index);
                } else {
                    self.search_diff_selected.insert(index);
                }
                Task::none()
            }

            Message::ToggleSearchDiffSelectAll => {
                if self.search_diff_selected.len() == self.search_diff_files.len() {
                    self.search_diff_selected.clear();
                } else {
                    self.search_diff_selected = (0..self.search_diff_files.len()).collect();
                }
                Task::none()
            }

            Message::ViewSearchDiffFile(index) => {
                let index = *index;
                if let Some(file) = self.search_diff_files.get(index) {
                    let file_path = file.display_path().to_string();
                    if let (Some(oid), Some(repo_path)) = (
                        self.search_diff_oid.clone(),
                        self.active_tab().repo_path.clone(),
                    ) {
                        return crate::features::commits::commands::search_diff_file(
                            repo_path, oid, file_path,
                        );
                    }
                }
                Task::none()
            }

            Message::SearchFileDiffLoaded(result) => {
                match result {
                    Ok(diff) => {
                        self.search_diff_content = vec![diff.clone()];
                    }
                    Err(e) => {
                        self.active_tab_mut().error_message =
                            Some(format!("Failed to load file diff: {e}"));
                    }
                }
                Task::none()
            }

            Message::DiffSelectedFiles => {
                if self.search_diff_selected.is_empty() {
                    return Task::none();
                }
                let file_paths: Vec<String> = self
                    .search_diff_selected
                    .iter()
                    .filter_map(|&i| self.search_diff_files.get(i))
                    .map(|f| f.display_path().to_string())
                    .collect();
                if let (Some(oid), Some(repo_path)) = (
                    self.search_diff_oid.clone(),
                    self.active_tab().repo_path.clone(),
                ) {
                    return crate::features::commits::commands::search_diff_multi_files(
                        repo_path, oid, file_paths,
                    );
                }
                Task::none()
            }

            Message::SearchMultiDiffLoaded(result) => {
                match result {
                    Ok(diffs) => {
                        self.search_diff_content = diffs.clone();
                    }
                    Err(e) => {
                        self.active_tab_mut().error_message =
                            Some(format!("Failed to load diffs: {e}"));
                    }
                }
                Task::none()
            }

            Message::SearchDiffBack => {
                self.search_diff_content.clear();
                Task::none()
            }

            Message::FileSystemChanged => {
                if self.has_repo() && !self.active_tab().is_loading {
                    return self.refresh_active_tab();
                }
                Task::none()
            }

            Message::OpenInEditor(path) => {
                self.active_tab_mut().context_menu = None;
                if matches!(self.editor, gitkraft_core::Editor::None) {
                    self.active_tab_mut().status_message = Some(
                        "No editor configured — select one from the editor dropdown in the toolbar"
                            .into(),
                    );
                    return Task::none();
                }
                if let Some(repo_path) = self.active_tab().repo_path.as_ref() {
                    let full_path = repo_path.join(path);
                    match self.editor.open_file(&full_path) {
                        Ok(()) => {
                            self.active_tab_mut().status_message =
                                Some(format!("Opened in {}", self.editor));
                        }
                        Err(e) => {
                            self.active_tab_mut().error_message =
                                Some(format!("Failed to open editor: {e}"));
                        }
                    }
                }
                Task::none()
            }

            Message::OpenInDefaultProgram(path) => {
                self.active_tab_mut().context_menu = None;
                if let Some(repo_path) = self.active_tab().repo_path.as_ref() {
                    let full_path = repo_path.join(path);
                    if let Err(e) = gitkraft_core::open_file_default(&full_path) {
                        self.active_tab_mut().error_message =
                            Some(format!("Failed to open file: {e}"));
                    }
                }
                Task::none()
            }

            Message::ShowInFolder(path) => {
                self.active_tab_mut().context_menu = None;
                if let Some(repo_path) = self.active_tab().repo_path.as_ref() {
                    let full_path = repo_path.join(path);
                    if let Err(e) = gitkraft_core::show_in_folder(&full_path) {
                        self.active_tab_mut().error_message =
                            Some(format!("Failed to show in folder: {e}"));
                    }
                }
                Task::none()
            }

            // ── File history overlay ──────────────────────────────────────────────
            Message::ViewFileHistory(path) => {
                let path = path.clone();
                if let Some(repo_path) = self.active_tab().repo_path.clone() {
                    let tab = self.active_tab_mut();
                    tab.blame_path = None; // close blame if open
                    tab.file_history_path = Some(path.clone());
                    tab.file_history_commits.clear();
                    tab.file_history_scroll = 0.0;
                    tab.context_menu = None;
                    tab.status_message = Some(format!(
                        "Loading history for {}…",
                        path.rsplit('/').next().unwrap_or(&path)
                    ));
                    crate::features::repo::commands::file_history_async(repo_path, path)
                } else {
                    Task::none()
                }
            }

            Message::FileHistoryLoaded(result) => {
                match result {
                    Ok((path, commits)) => {
                        let tab = self.active_tab_mut();
                        tab.file_history_path = Some(path.clone());
                        tab.file_history_commits = commits.clone();
                        tab.status_message = Some(format!(
                            "{} commits touch {}",
                            commits.len(),
                            path.rsplit('/').next().unwrap_or(path)
                        ));
                    }
                    Err(e) => {
                        let tab = self.active_tab_mut();
                        tab.file_history_path = None;
                        tab.error_message = Some(format!("File history failed: {e}"));
                    }
                }
                Task::none()
            }

            Message::CloseFileHistory => {
                let tab = self.active_tab_mut();
                tab.file_history_path = None;
                tab.file_history_commits.clear();
                tab.file_history_scroll = 0.0;
                Task::none()
            }

            Message::FileHistoryScrolled(y) => {
                self.active_tab_mut().file_history_scroll = *y;
                Task::none()
            }

            Message::SelectFileHistoryCommit(oid) => {
                let oid = oid.clone();
                let repo_path = self.active_tab().repo_path.clone();
                {
                    let tab = self.active_tab_mut();
                    tab.file_history_path = None;
                    tab.file_history_commits.clear();
                    tab.selected_commit_oid = Some(oid.clone());
                    tab.commit_files.clear();
                    tab.selected_diff = None;
                    tab.show_commit_detail = true;
                }
                if let Some(path) = repo_path {
                    crate::features::commits::commands::load_commit_file_list(path, oid)
                } else {
                    Task::none()
                }
            }

            // ── Blame overlay ─────────────────────────────────────────────────────
            Message::ViewFileBlame(path) => {
                let path = path.clone();
                if let Some(repo_path) = self.active_tab().repo_path.clone() {
                    let tab = self.active_tab_mut();
                    tab.file_history_path = None; // close history if open
                    tab.blame_path = Some(path.clone());
                    tab.blame_lines.clear();
                    tab.blame_scroll = 0.0;
                    tab.context_menu = None;
                    tab.status_message = Some(format!(
                        "Loading blame for {}…",
                        path.rsplit('/').next().unwrap_or(&path)
                    ));
                    crate::features::repo::commands::blame_file_async(repo_path, path)
                } else {
                    Task::none()
                }
            }

            Message::FileBlameLoaded(result) => {
                match result {
                    Ok((path, lines)) => {
                        let tab = self.active_tab_mut();
                        tab.blame_path = Some(path.clone());
                        tab.blame_lines = lines.clone();
                        tab.status_message = Some(format!(
                            "Blame: {} ({} lines)",
                            path.rsplit('/').next().unwrap_or(path),
                            lines.len()
                        ));
                    }
                    Err(e) => {
                        let tab = self.active_tab_mut();
                        tab.blame_path = None;
                        tab.error_message = Some(format!("Blame failed: {e}"));
                    }
                }
                Task::none()
            }

            Message::CloseFileBlame => {
                let tab = self.active_tab_mut();
                tab.blame_path = None;
                tab.blame_lines.clear();
                tab.blame_scroll = 0.0;
                Task::none()
            }

            Message::BlameScrolled(y) => {
                self.active_tab_mut().blame_scroll = *y;
                Task::none()
            }

            // ── File deletion ─────────────────────────────────────────────────────
            Message::DeleteFile(path) => {
                let tab = self.active_tab_mut();
                tab.context_menu = None;
                tab.pending_delete_file = Some(path.clone());
                tab.status_message = Some(format!(
                    "Delete '{}' — press Confirm to delete permanently",
                    path.rsplit('/').next().unwrap_or(path)
                ));
                Task::none()
            }

            Message::ConfirmDeleteFile => {
                let path = self.active_tab().pending_delete_file.clone();
                let repo_path = self.active_tab().repo_path.clone();
                if let (Some(file_path), Some(repo_path)) = (path, repo_path) {
                    let tab = self.active_tab_mut();
                    tab.pending_delete_file = None;
                    tab.is_loading = true;
                    tab.status_message = Some(format!(
                        "Deleting '{}'…",
                        file_path.rsplit('/').next().unwrap_or(&file_path)
                    ));
                    crate::features::repo::commands::delete_file_async(repo_path, file_path)
                } else {
                    Task::none()
                }
            }

            Message::CancelDeleteFile => {
                let tab = self.active_tab_mut();
                tab.pending_delete_file = None;
                tab.status_message = None;
                Task::none()
            }

            Message::Noop => Task::none(),

            Message::CherryPickCommits(oids) => {
                let oids = oids.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Cherry-picking {} commit(s)…", oids.len()),
                    |path| crate::features::repo::commands::cherry_pick_commits_async(path, oids)
                )
            }

            Message::RevertCommits(oids) => {
                let oids = oids.clone();
                self.active_tab_mut().context_menu = None;
                with_repo!(
                    self,
                    loading,
                    format!("Reverting {} commit(s)…", oids.len()),
                    |path| crate::features::repo::commands::revert_commits_async(path, oids)
                )
            }
        }
    }
}
