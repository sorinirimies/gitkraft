//! Async command helpers for commit operations.
//!
//! Each function spawns blocking work on a background thread via
//! `std::thread::spawn` + `futures::channel::oneshot`, performs the git
//! operation, and maps the result into a [`Message`].

use std::path::PathBuf;

use futures::channel::oneshot;
use iced::Task;

use crate::message::Message;
use gitkraft_core::DiffInfo;

/// Load the diff introduced by a specific commit (by OID string).
pub fn load_commit_diff(path: PathBuf, oid: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    let diffs: Vec<DiffInfo> =
                        gitkraft_core::features::diff::get_commit_diff(&repo, &oid)
                            .map_err(|e| e.to_string())?;
                    Ok(diffs)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::CommitDiffLoaded,
    )
}

/// Create a new commit with the currently staged changes.
pub fn create_commit(path: PathBuf, message: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::commits::create_commit(&repo, &message)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::CommitCreated,
    )
}
