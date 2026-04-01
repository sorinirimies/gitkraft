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
            // ── Repository ────────────────────────────────────────────────
            Message::OpenRepo
            | Message::InitRepo
            | Message::RepoSelected(_)
            | Message::RepoInitSelected(_)
            | Message::RepoOpened(_)
            | Message::RefreshRepo
            | Message::RepoRefreshed(_) => crate::features::repo::update::update(self, message),

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

            // ── Staging ───────────────────────────────────────────────────
            Message::StageFile(_)
            | Message::UnstageFile(_)
            | Message::StageAll
            | Message::UnstageAll
            | Message::DiscardFile(_)
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
                self.error_message = None;
                Task::none()
            }

            Message::ToggleSidebar => {
                self.sidebar_expanded = !self.sidebar_expanded;
                Task::none()
            }

            Message::ThemeChanged(theme) => {
                self.theme = theme.clone();
                Task::none()
            }

            Message::Noop => Task::none(),
        }
    }
}
