//! Async command helpers for staging-area operations.
//!
//! Each function clones the `PathBuf`, spawns blocking work that opens the repo
//! inside, performs the staging operation, then re-reads both the working-dir
//! and staged diffs so the UI can update in one shot.

use std::path::PathBuf;

use futures::channel::oneshot;
use iced::Task;

use crate::message::{Message, StagingPayload};

/// Stage a single file, then return the refreshed staging state.
pub fn stage_file(path: PathBuf, file_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::staging::stage_file(&repo, &file_path)
                        .map_err(|e| e.to_string())?;
                    refresh_staging_state(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::StagingUpdated,
    )
}

/// Unstage a single file, then return the refreshed staging state.
pub fn unstage_file(path: PathBuf, file_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::staging::unstage_file(&repo, &file_path)
                        .map_err(|e| e.to_string())?;
                    refresh_staging_state(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::StagingUpdated,
    )
}

/// Stage all unstaged files, then return the refreshed staging state.
pub fn stage_all(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::staging::stage_all(&repo)
                        .map_err(|e| e.to_string())?;
                    refresh_staging_state(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::StagingUpdated,
    )
}

/// Unstage all staged files, then return the refreshed staging state.
pub fn unstage_all(path: PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::staging::unstage_all(&repo)
                        .map_err(|e| e.to_string())?;
                    refresh_staging_state(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::StagingUpdated,
    )
}

/// Discard working-directory changes for a single file, then return the
/// refreshed staging state.
pub fn discard_file(path: PathBuf, file_path: String) -> Task<Message> {
    Task::perform(
        async move {
            let (tx, rx) = oneshot::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let repo = gitkraft_core::features::repo::open_repo(&path)
                        .map_err(|e| e.to_string())?;
                    gitkraft_core::features::staging::discard_file_changes(&repo, &file_path)
                        .map_err(|e| e.to_string())?;
                    refresh_staging_state(&path)
                })();
                let _ = tx.send(result);
            });
            rx.await.map_err(|_| "Task cancelled".to_string())?
        },
        Message::StagingUpdated,
    )
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Re-read both the working-directory diff and the staged diff so the caller
/// can update the UI in one shot.
fn refresh_staging_state(path: &std::path::Path) -> Result<StagingPayload, String> {
    let repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;
    let unstaged =
        gitkraft_core::features::diff::get_working_dir_diff(&repo).map_err(|e| e.to_string())?;
    let staged =
        gitkraft_core::features::diff::get_staged_diff(&repo).map_err(|e| e.to_string())?;
    Ok(StagingPayload { unstaged, staged })
}
