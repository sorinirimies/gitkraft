//! Staging-area (index) operations — stage, unstage, and discard changes.

use std::path::Path;

use anyhow::{Context, Result};
use git2::{build::CheckoutBuilder, IndexAddOption, Repository};
use tracing::debug;

/// Stage a single file (add it to the index).
///
/// Works for both tracked-modified and untracked files.
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    debug!("staging file: {}", path);
    let mut index = repo.index().context("failed to read index")?;
    index
        .add_path(Path::new(path))
        .with_context(|| format!("failed to stage '{path}'"))?;
    index.write().context("failed to write index")?;
    Ok(())
}

/// Remove a single file from the staging area (unstage).
///
/// Resets the index entry to match HEAD. If HEAD doesn't exist yet (initial
/// commit scenario), the entry is removed from the index entirely.
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    debug!("unstaging file: {}", path);

    match repo.head() {
        Ok(head_ref) => {
            let head_obj = head_ref
                .peel(git2::ObjectType::Commit)
                .context("HEAD does not point to a commit")?;
            repo.reset_default(Some(&head_obj), [path])
                .with_context(|| format!("failed to unstage '{path}'"))?;
        }
        Err(_) => {
            // No HEAD yet (empty repo / initial commit). Remove from index.
            let mut index = repo.index().context("failed to read index")?;
            index
                .remove_path(Path::new(path))
                .with_context(|| format!("failed to remove '{path}' from index"))?;
            index.write().context("failed to write index")?;
        }
    }

    Ok(())
}

/// Stage all changes in the working directory (tracked and untracked).
pub fn stage_all(repo: &Repository) -> Result<()> {
    debug!("staging all files");
    let mut index = repo.index().context("failed to read index")?;
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .context("failed to stage all files")?;
    index.write().context("failed to write index")?;
    Ok(())
}

/// Unstage all currently staged changes, resetting the index back to HEAD.
///
/// If HEAD doesn't exist yet (initial commit), the index is cleared entirely.
pub fn unstage_all(repo: &Repository) -> Result<()> {
    debug!("unstaging all files");

    match repo.head() {
        Ok(head_ref) => {
            let head_obj = head_ref
                .peel(git2::ObjectType::Commit)
                .context("HEAD does not point to a commit")?;
            let head_commit = repo
                .find_commit(head_obj.id())
                .context("failed to find HEAD commit")?;
            let head_tree = head_commit.tree().context("failed to read HEAD tree")?;

            repo.reset(head_commit.as_object(), git2::ResetType::Mixed, None)
                .or_else(|_| {
                    // Fallback: manually reset the index to the HEAD tree
                    let mut index = repo.index().context("failed to read index")?;
                    index
                        .read_tree(&head_tree)
                        .context("failed to read HEAD tree into index")?;
                    index.write().context("failed to write index")
                })
                .context("failed to unstage all files")?;
        }
        Err(_) => {
            // No HEAD yet — clear the index entirely
            let mut index = repo.index().context("failed to read index")?;
            index.clear().context("failed to clear index")?;
            index.write().context("failed to write index")?;
        }
    }

    Ok(())
}

/// Discard working-directory changes to a single file, restoring it to the
/// version currently in the index (or HEAD if not staged).
///
/// **Warning:** this is destructive — uncommitted changes to the file are lost.
pub fn discard_file_changes(repo: &Repository, path: &str) -> Result<()> {
    debug!("discarding changes for: {}", path);

    let mut cb = CheckoutBuilder::new();
    cb.path(path);
    cb.force();

    repo.checkout_head(Some(&mut cb))
        .with_context(|| format!("failed to discard changes for '{path}'"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_repo_with_commit(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();

        // Write a file, stage it, and commit
        let file_path = dir.join("hello.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("hello.txt")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();

        {
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        repo
    }

    #[test]
    fn stage_and_unstage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        // Modify the file
        fs::write(tmp.path().join("hello.txt"), "modified\n").unwrap();

        // Stage it
        stage_file(&repo, "hello.txt").unwrap();

        // Verify it is staged (diff HEAD→index should have the file)
        let head = repo.head().unwrap().peel_to_tree().unwrap();
        let diff = repo.diff_tree_to_index(Some(&head), None, None).unwrap();
        assert_eq!(diff.deltas().len(), 1);

        // Unstage it
        unstage_file(&repo, "hello.txt").unwrap();

        // Now HEAD→index diff should be empty
        let head2 = repo.head().unwrap().peel_to_tree().unwrap();
        let diff2 = repo.diff_tree_to_index(Some(&head2), None, None).unwrap();
        assert_eq!(diff2.deltas().len(), 0);
    }

    #[test]
    fn stage_all_and_unstage_all() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        // Modify existing file and add a new one
        fs::write(tmp.path().join("hello.txt"), "changed\n").unwrap();
        fs::write(tmp.path().join("new.txt"), "new file\n").unwrap();

        stage_all(&repo).unwrap();

        let head = repo.head().unwrap().peel_to_tree().unwrap();
        let diff = repo.diff_tree_to_index(Some(&head), None, None).unwrap();
        assert_eq!(diff.deltas().len(), 2);

        unstage_all(&repo).unwrap();

        let head2 = repo.head().unwrap().peel_to_tree().unwrap();
        let diff2 = repo.diff_tree_to_index(Some(&head2), None, None).unwrap();
        assert_eq!(diff2.deltas().len(), 0);
    }

    #[test]
    fn discard_restores_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_repo_with_commit(tmp.path());

        let file_path = tmp.path().join("hello.txt");
        fs::write(&file_path, "totally different content\n").unwrap();

        discard_file_changes(&repo, "hello.txt").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world\n");
    }

    #[test]
    fn stage_and_unstage_on_empty_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        // Create a new file in an empty repo (no HEAD yet)
        fs::write(tmp.path().join("new.txt"), "brand new\n").unwrap();

        stage_file(&repo, "new.txt").unwrap();

        // Verify it's in the index
        let index = repo.index().unwrap();
        assert!(index.get_path(Path::new("new.txt"), 0).is_some());

        // Unstage it (no HEAD path)
        unstage_file(&repo, "new.txt").unwrap();

        let index2 = repo.index().unwrap();
        assert!(index2.get_path(Path::new("new.txt"), 0).is_none());
    }
}
