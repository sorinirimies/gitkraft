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
            | Message::SettingsLoaded(_) => crate::features::repo::update::update(self, message),

            // ── Branches ──────────────────────────────────────────────────
            Message::CheckoutBranch(_)
            | Message::BranchCheckedOut(_)
            | Message::CreateBranch
            | Message::NewBranchNameChanged(_)
            | Message::BranchCreated(_)
            | Message::DeleteBranch(_)
            | Message::BranchDeleted(_)
            | Message::ToggleBranchCreate => {
                crate::features::branches::update::update(self, message)
            }

            // ── Commits ───────────────────────────────────────────────────
            Message::SelectCommit(_) | Message::CommitDiffLoaded(_) => {
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
            Message::SelectDiff(_) => crate::features::diff::update::update(self, message),

            Message::DismissError => {
                self.active_tab_mut().error_message = None;
                Task::none()
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
                }
                Task::none()
            }

            Message::Noop => Task::none(),
        }
    }
}
