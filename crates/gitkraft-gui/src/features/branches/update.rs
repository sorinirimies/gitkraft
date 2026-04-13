//! Update logic for branch-related messages.

use iced::Task;

use crate::message::Message;
use crate::state::GitKraft;

use super::commands;

/// Handle all branch-related messages, returning a [`Task`] for any follow-up
/// async work.
pub fn update(state: &mut GitKraft, message: Message) -> Task<Message> {
    match message {
        Message::CheckoutBranch(name) => with_repo!(
            state,
            loading,
            format!("Checking out '{name}'…"),
            |repo_path| commands::checkout_branch(repo_path, name)
        ),

        Message::BranchCheckedOut(result) => {
            state.on_ok_refresh(result, "Branch checked out.", "Checkout failed")
        }

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
            with_repo!(
                state,
                loading,
                format!("Creating branch '{name}'…"),
                |repo_path| {
                    let tab = state.active_tab_mut();
                    tab.show_branch_create = false;
                    tab.new_branch_name.clear();
                    commands::create_branch(repo_path, name)
                }
            )
        }

        Message::BranchCreated(result) => {
            state.on_ok_refresh(result, "Branch created.", "Branch creation failed")
        }

        Message::DeleteBranch(name) => with_repo!(
            state,
            loading,
            format!("Deleting branch '{name}'…"),
            |repo_path| commands::delete_branch(repo_path, name)
        ),

        Message::BranchDeleted(result) => {
            state.on_ok_refresh(result, "Branch deleted.", "Branch deletion failed")
        }

        _ => Task::none(),
    }
}
