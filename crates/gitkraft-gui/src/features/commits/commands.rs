//! Async command helpers for commit operations.
//!
//! Each function spawns blocking work on a background thread via the
//! [`git_task!`] macro, performs the git operation, and maps the result into a
//! [`Message`].

use std::path::PathBuf;

use iced::Task;

use crate::message::Message;
use gitkraft_core::DiffInfo;

/// Load the diff introduced by a specific commit (by OID string).
pub fn load_commit_diff(path: PathBuf, oid: String) -> Task<Message> {
    git_task!(
        Message::CommitDiffLoaded,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            let diffs: Vec<DiffInfo> = gitkraft_core::features::diff::get_commit_diff(&repo, &oid)
                .map_err(|e| e.to_string())?;
            Ok(diffs)
        })()
    )
}

/// Create a new commit with the currently staged changes.
pub fn create_commit(path: PathBuf, message: String) -> Task<Message> {
    git_task!(
        Message::CommitCreated,
        (|| {
            let repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::commits::create_commit(&repo, &message)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })()
    )
}
