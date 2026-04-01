//! Async command helpers for remote operations.
//!
//! Each function clones the `PathBuf`, spawns blocking work that opens the repo
//! inside, performs the remote operation, and maps the result into a [`Message`].

use std::path::PathBuf;

use futures::channel::oneshot;
use iced::Task;

use crate::message::Message;

/// Fetch from a named remote.
///
/// If the remote doesn't exist or authentication is required, the task resolves
/// to an error message.
pub fn fetch_remote(path: PathBuf, remote_name: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::remotes::fetch_remote(&repo, &remote_name)
                        .map_err(|e| e.to_string())
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::FetchCompleted,
    )
}
