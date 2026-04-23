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
            let mut repo = open_repo!(&path);
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
            let mut repo = open_repo!(&path);
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
            let mut repo = open_repo!(&path);
            gitkraft_core::features::stash::stash_drop(&mut repo, index)
                .map_err(|e| e.to_string())?;
            refresh_stash_list(&path)
        })()
    )
}

/// Load the diff for a stash entry.
pub fn load_stash_diff(path: PathBuf, index: usize) -> Task<Message> {
    git_task!(
        Message::StashDiffLoaded,
        (|| {
            let mut repo = open_repo!(&path);
            // Get the stash commit OID
            let mut stash_oid = None;
            repo.stash_foreach(|i, _msg, oid| {
                if i == index {
                    stash_oid = Some(oid.to_string());
                    false // stop iterating
                } else {
                    true
                }
            })
            .map_err(|e| e.to_string())?;

            let oid = stash_oid.ok_or_else(|| format!("stash@{{{index}}} not found"))?;
            let repo = open_repo!(&path); // reopen as immutable
            gitkraft_core::features::diff::get_commit_diff(&repo, &oid).map_err(|e| e.to_string())
        })()
    )
}

/// Apply a stash entry (like pop but keeps it in the stash list).
pub fn stash_apply(path: PathBuf, index: usize) -> Task<Message> {
    git_task!(
        Message::StashUpdated,
        (|| {
            let mut repo = open_repo!(&path);
            repo.stash_apply(index, None).map_err(|e| e.to_string())?;
            refresh_stash_list(&path)
        })()
    )
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Re-read the stash list so the caller can update the UI in one shot.
fn refresh_stash_list(path: &std::path::Path) -> Result<Vec<gitkraft_core::StashEntry>, String> {
    let mut repo = open_repo!(path);
    gitkraft_core::features::stash::list_stashes(&mut repo).map_err(|e| e.to_string())
}
