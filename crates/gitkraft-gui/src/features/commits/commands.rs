//! Async command helpers for commit operations.
//!
//! Each function spawns blocking work on a background thread via the
//! `git_task!` macro, performs the git operation, and maps the result into a
//! [`Message`].

use std::path::PathBuf;

use iced::Task;

use crate::message::Message;

/// Load just the file list (paths + statuses) for a commit — no line parsing.
pub fn load_commit_file_list(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::CommitFileListLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::get_commit_file_list(&repo, &oid)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Load the full diff for a single file in a commit.
pub fn load_single_file_diff(path: PathBuf, oid: String, file_path: String) -> Task<Message> {
    git_task!(
        Message::SingleFileDiffLoaded,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::diff::get_single_file_diff(&repo, &oid, &file_path)
                .map_err(|e| e.to_string())
        })()
    )
}

/// Create a new commit with the currently staged changes.
pub fn create_commit(path: PathBuf, message: String) -> Task<Message> {
    git_task!(
        Message::CommitCreated,
        (|| {
            let repo = open_repo!(&path);
            gitkraft_core::features::commits::create_commit(&repo, &message)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })()
    )
}
