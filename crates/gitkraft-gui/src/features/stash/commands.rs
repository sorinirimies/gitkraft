//! Async command helpers for stash operations.
//!
//! Stash operations in `gitkraft_core` require `&mut Repository`, so each
//! command opens its own mutable repository handle inside the blocking task.

use std::path::PathBuf;

use iced::Task;

use crate::message::Message;

/// Save the current working state as a new stash entry, then return the
/// refreshed stash list.
pub fn stash_save(path: PathBuf, stash_message: Option<String>) -> Task<Message> {
    git_task!(
        Message::StashUpdated,
        (|| {
            let mut repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            let msg_ref = stash_message.as_deref();
            gitkraft_core::features::stash::stash_save(&mut repo, msg_ref)
                .map_err(|e| e.to_string())?;
            refresh_stash_list(&path)
        })()
    )
}

/// Pop (apply + drop) a stash entry by index, then return the refreshed stash
/// list.
pub fn stash_pop(path: PathBuf, index: usize) -> Task<Message> {
    git_task!(
        Message::StashUpdated,
        (|| {
            let mut repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::stash::stash_pop(&mut repo, index)
                .map_err(|e| e.to_string())?;
            refresh_stash_list(&path)
        })()
    )
}

/// Drop (delete without applying) a stash entry by index, then return the
/// refreshed stash list.
pub fn stash_drop(path: PathBuf, index: usize) -> Task<Message> {
    git_task!(
        Message::StashUpdated,
        (|| {
            let mut repo =
                gitkraft_core::features::repo::open_repo(&path).map_err(|e| e.to_string())?;
            gitkraft_core::features::stash::stash_drop(&mut repo, index)
                .map_err(|e| e.to_string())?;
            refresh_stash_list(&path)
        })()
    )
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Re-read the stash list so the caller can update the UI in one shot.
fn refresh_stash_list(path: &std::path::Path) -> Result<Vec<gitkraft_core::StashEntry>, String> {
    let mut repo = gitkraft_core::features::repo::open_repo(path).map_err(|e| e.to_string())?;
    gitkraft_core::features::stash::list_stashes(&mut repo).map_err(|e| e.to_string())
}
