//! Async command helpers for branch operations.
//!
//! Each function clones the `PathBuf`, spawns blocking work that opens the repo
//! inside, performs the git operation, and maps the result into a [`Message`].

use std::path::PathBuf;

use iced::Task;

use crate::message::Message;

/// Checkout an existing local branch by name.
pub fn checkout_branch(path: PathBuf, branch_name: String) -> Task<Message> {
    git_task!(
        Message::BranchCheckedOut,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::branches::checkout_branch(&repo, &branch_name)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Create a new local branch at HEAD with the given name.
pub fn create_branch(path: PathBuf, branch_name: String) -> Task<Message> {
    git_task!(
        Message::BranchCreated,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::branches::create_branch(&repo, &branch_name)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })()
    )
}

/// Delete a local branch by name.
pub fn delete_branch(path: PathBuf, branch_name: String) -> Task<Message> {
    git_task!(
        Message::BranchDeleted,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::branches::delete_branch(&repo, &branch_name)
                .map_err(|e| e.to_string())
        })()
    )
}
