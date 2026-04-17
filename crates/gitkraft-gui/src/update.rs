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
            Message::SelectCommit(_) | Message::CommitFileListLoaded(_) | Message::SingleFileDiffLoaded(_) => {
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
            | Message::StagingUpdated(_) => crate::features::staging::update::update(self, message),

            // ── Stash ─────────────────────────────────────────────────────
            Message::StashSave
            | Message::StashPop(_)
            | Message::StashDrop(_)
            | Message::StashUpdated(_)
            | Message::StashMessageChanged(_) => {
                crate::features::stash::update::update(self, message)
            }

            // ── Remotes ───────────────────────────────────────────────────
            Message::Fetch | Message::FetchCompleted(_) => {
                crate::features::remotes::update::update(self, message)
            }

            // ── UI / misc ─────────────────────────────────────────────────
            Message::SelectDiff(_) | Message::SelectDiffByIndex(_) => crate::features::diff::update::update(self, message),

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

            Message::ToggleSidebar => {
                self.sidebar_expanded = !self.sidebar_expanded;
                crate::features::repo::commands::save_layout_async(self.current_layout())
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
                tab.context_menu = Some(crate::state::ContextMenu::RemoteBranch {
                    name: name.clone(),
                });
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

            // ── Commit context menu actions ───────────────────────────────────────────
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
            Message::CopyText(text) => iced::clipboard::write(text.clone()),

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

            Message::Noop => Task::none(),
        }
    }
}
