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
            let path = state.active_tab().repo_path.clone();
            if let Some(path) = path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Checking out '{name}'…"));
                commands::checkout_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchCheckedOut(result) => match result {
            Ok(()) => {
                {
                    let tab = state.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = Some("Branch checked out.".into());
                }
                // Trigger a full refresh to update everything.
                let path = state.active_tab().repo_path.clone();
                if let Some(path) = path {
                    crate::features::repo::commands::refresh_repo(path)
                } else {
                    Task::none()
                }
            }
            Err(e) => {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.error_message = Some(format!("Checkout failed: {e}"));
                tab.status_message = None;
                Task::none()
            }
        },

        Message::ToggleBranchCreate => {
            let tab = state.active_tab_mut();
            tab.show_branch_create = !tab.show_branch_create;
            if !tab.show_branch_create {
                tab.new_branch_name.clear();
            }
            Task::none()
        }

        Message::NewBranchNameChanged(name) => {
            state.active_tab_mut().new_branch_name = name;
            Task::none()
        }

        Message::CreateBranch => {
            let name = state.active_tab().new_branch_name.trim().to_string();
            if name.is_empty() {
                return Task::none();
            }
            let path = state.active_tab().repo_path.clone();
            if let Some(path) = path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Creating branch '{name}'…"));
                tab.show_branch_create = false;
                tab.new_branch_name.clear();
                commands::create_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchCreated(result) => match result {
            Ok(()) => {
                {
                    let tab = state.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = Some("Branch created.".into());
                }
                let path = state.active_tab().repo_path.clone();
                if let Some(path) = path {
                    crate::features::repo::commands::refresh_repo(path)
                } else {
                    Task::none()
                }
            }
            Err(e) => {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.error_message = Some(format!("Branch creation failed: {e}"));
                tab.status_message = None;
                Task::none()
            }
        },

        Message::DeleteBranch(name) => {
            let path = state.active_tab().repo_path.clone();
            if let Some(path) = path {
                let tab = state.active_tab_mut();
                tab.is_loading = true;
                tab.status_message = Some(format!("Deleting branch '{name}'…"));
                commands::delete_branch(path, name)
            } else {
                Task::none()
            }
        }

        Message::BranchDeleted(result) => match result {
            Ok(()) => {
                {
                    let tab = state.active_tab_mut();
                    tab.is_loading = false;
                    tab.status_message = Some("Branch deleted.".into());
                }
                let path = state.active_tab().repo_path.clone();
                if let Some(path) = path {
                    crate::features::repo::commands::refresh_repo(path)
                } else {
                    Task::none()
                }
            }
            Err(e) => {
                let tab = state.active_tab_mut();
                tab.is_loading = false;
                tab.error_message = Some(format!("Branch deletion failed: {e}"));
                tab.status_message = None;
                Task::none()
            }
        },

        _ => Task::none(),
    }
}
