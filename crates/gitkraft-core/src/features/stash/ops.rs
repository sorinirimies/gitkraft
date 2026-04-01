//! Stash operations — list, save, pop, and drop stash entries.

use anyhow::{Context, Result};
use git2::Repository;
use tracing::debug;

use super::types::StashEntry;

/// List all stash entries in the repository.
///
/// Note: `stash_foreach` requires `&mut Repository` in git2, but only reads
/// stash state. We accept `&mut Repository` here to be safe and correct.
pub fn list_stashes(repo: &mut Repository) -> Result<Vec<StashEntry>> {
    let mut stashes = Vec::new();

    repo.stash_foreach(|index, message, oid| {
        stashes.push(StashEntry {
            index,
            message: message.to_string(),
            oid: oid.to_string(),
        });
        true // continue iterating
    })
    .context("Failed to iterate stashes")?;

    debug!("Found {} stash entries", stashes.len());
    Ok(stashes)
}

/// Save the current working directory and index state as a new stash entry.
///
/// If `message` is `None`, a default "WIP" message is used.
/// Returns the newly created `StashEntry`.
pub fn stash_save(repo: &mut Repository, message: Option<&str>) -> Result<StashEntry> {
    let signature = repo.signature().context(
        "Failed to determine default signature for stash — set user.name and user.email",
    )?;

    let msg = message.unwrap_or("WIP");

    let oid = repo
        .stash_save(&signature, msg, None)
        .context("Failed to save stash (are there any changes to stash?)")?;

    debug!("Stash saved: {} — {}", oid, msg);

    Ok(StashEntry {
        index: 0, // newly saved stash is always at index 0
        message: msg.to_string(),
        oid: oid.to_string(),
    })
}

/// Pop (apply + drop) a stash entry by its zero-based index.
pub fn stash_pop(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_pop(index, None)
        .with_context(|| format!("Failed to pop stash at index {index}"))?;

    debug!("Stash at index {} popped", index);
    Ok(())
}

/// Drop (delete) a stash entry by its zero-based index without applying it.
pub fn stash_drop(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_drop(index)
        .with_context(|| format!("Failed to drop stash at index {index}"))?;

    debug!("Stash at index {} dropped", index);
    Ok(())
}
