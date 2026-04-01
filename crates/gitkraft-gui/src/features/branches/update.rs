//! Update logic for branch-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all branch-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::CheckoutBranch(name) => {
            if let Some(path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some(format!("Checking out '{name}'…"));
                commands::checkout_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchCheckedOut(result) => {
            state.is_loading = false;
            match result {
                Ok(()) => {
                    state.status_message = Some("Branch checked out.".into());
                    // Trigger a full refresh to update everything.
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Checkout failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        Message::ToggleBranchCreate => {
            state.show_branch_create = !state.show_branch_create;
            if !state.show_branch_create {
                state.new_branch_name.clear();
            }
            Task::none()
        }

        Message::NewBranchNameChanged(name) => {
            state.new_branch_name = name;
            Task::none()
        }

        Message::CreateBranch => {
            let name = state.new_branch_name.trim().to_string();
            if name.is_empty() {
                return Task::none();
            }
            if let Some(path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some(format!("Creating branch '{name}'…"));
                state.show_branch_create = false;
                state.new_branch_name.clear();
                commands::create_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchCreated(result) => {
            state.is_loading = false;
            match result {
                Ok(()) => {
                    state.status_message = Some("Branch created.".into());
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Branch creation failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        Message::DeleteBranch(name) => {
            if let Some(path) = state.repo_path.clone() {
                state.is_loading = true;
                state.status_message = Some(format!("Deleting branch '{name}'…"));
                commands::delete_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchDeleted(result) => {
            state.is_loading = false;
            match result {
                Ok(()) => {
                    state.status_message = Some("Branch deleted.".into());
                    if let Some(path) = state.repo_path.clone() {
                        crate::features::repo::commands::refresh_repo(path)
                    } else {
                        Task::none()
                    }
                }
                Err(e) => {
                    state.error_message = Some(format!("Branch deletion failed: {e}"));
                    state.status_message = None;
                    Task::none()
                }
            }
        }

        _ => Task::none(),
    }
}
