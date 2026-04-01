//! Async command helpers for diff operations.
//!
//! Each function spawns blocking work on a background thread via
//! `std::thread::spawn` + `futures::channel::oneshot`, then maps the result
//! into a [`Message`].

use std::path::PathBuf;

use futures::channel::oneshot;
use iced::Task;

use crate::message::Message;

/// Load the diff for a specific commit (compared to its first parent).
pub fn load_commit_diff(path: PathBuf, oid: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::diff::get_commit_diff(&repo, &oid)
                        .map_err(|e| e.to_string())
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::CommitDiffLoaded,
    )
}
