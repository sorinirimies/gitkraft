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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo_with_commit() -> (TempDir, Repository) {
        // same helper as branches
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        std::fs::write(dir.path().join("file.txt"), "hello\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        (dir, repo)
    }

    #[test]
    fn list_stashes_empty() {
        let (_dir, mut repo) = setup_repo_with_commit();
        let stashes = list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }

    #[test]
    fn stash_save_and_list() {
        let (dir, mut repo) = setup_repo_with_commit();
        // Make a change
        std::fs::write(dir.path().join("file.txt"), "changed\n").unwrap();
        let entry = stash_save(&mut repo, Some("test stash")).unwrap();
        assert_eq!(entry.index, 0);
        assert!(entry.message.contains("test stash"));

        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);
    }

    #[test]
    fn stash_pop_restores_changes() {
        let (dir, mut repo) = setup_repo_with_commit();
        std::fs::write(dir.path().join("file.txt"), "changed\n").unwrap();
        stash_save(&mut repo, Some("test")).unwrap();

        // File should be restored to committed state after stash
        let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
        assert_eq!(content, "hello\n");

        stash_pop(&mut repo, 0).unwrap();
        let content = std::fs::read_to_string(dir.path().join("file.txt")).unwrap();
        assert_eq!(content, "changed\n");
    }

    fn setup_repo_with_stash() -> (TempDir, Repository) {
        let (dir, mut repo) = setup_repo_with_commit();
        std::fs::write(dir.path().join("file.txt"), "changed\n").unwrap();
        stash_save(&mut repo, Some("test stash")).unwrap();
        (dir, repo)
    }

    #[test]
    fn stash_drop_removes_entry() {
        let (_dir, mut repo) = setup_repo_with_stash();
        let stashes = list_stashes(&mut repo).unwrap();
        assert_eq!(stashes.len(), 1);

        stash_drop(&mut repo, 0).unwrap();

        let stashes = list_stashes(&mut repo).unwrap();
        assert!(stashes.is_empty());
    }
}
