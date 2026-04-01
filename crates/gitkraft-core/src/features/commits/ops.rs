//! Commit operations — list, create, and inspect commits.

use anyhow::{Context, Result};
use git2::Repository;

use super::types::CommitInfo;

/// Walk the history from HEAD and return up to `max_count` commits.
///
/// Commits are sorted topologically and by time (newest first).
pub fn list_commits(repo: &Repository, max_count: usize) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk
        .push_head()
        .context("failed to push HEAD to revwalk")?;
    revwalk
        .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .context("failed to set revwalk sorting")?;

    let mut commits = Vec::with_capacity(max_count.min(256));
    for oid_result in revwalk {
        if commits.len() >= max_count {
            break;
        }
        let oid = oid_result.context("revwalk iteration error")?;
        let commit = repo
            .find_commit(oid)
            .with_context(|| format!("failed to find commit {oid}"))?;
        commits.push(CommitInfo::from_git2_commit(&commit));
    }

    Ok(commits)
}

/// Commit the currently staged (index) changes with the given message.
///
/// Uses the repository's default signature (`user.name` / `user.email`).
/// Returns the newly created [`CommitInfo`].
pub fn create_commit(repo: &Repository, message: &str) -> Result<CommitInfo> {
    let sig = repo.signature().context(
        "failed to obtain default signature — set user.name and user.email in git config",
    )?;

    let mut index = repo.index().context("failed to read index")?;
    let tree_oid = index
        .write_tree()
        .context("failed to write index to tree — are there staged changes?")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find tree written from index")?;

    // Collect parent commits (HEAD if it exists).
    let parent_commit;
    let parents: Vec<&git2::Commit<'_>> = if let Ok(head_ref) = repo.head() {
        let head_oid = head_ref
            .target()
            .context("HEAD is not a direct reference")?;
        parent_commit = repo
            .find_commit(head_oid)
            .context("failed to find HEAD commit")?;
        vec![&parent_commit]
    } else {
        // Initial commit — no parents.
        vec![]
    };

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .context("failed to create commit")?;

    let commit = repo
        .find_commit(oid)
        .context("failed to look up newly created commit")?;

    Ok(CommitInfo::from_git2_commit(&commit))
}

/// Retrieve the full [`CommitInfo`] for a commit identified by its hex OID string.
pub fn get_commit_details(repo: &Repository, oid_str: &str) -> Result<CommitInfo> {
    let oid =
        git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID string: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;

    Ok(CommitInfo::from_git2_commit(&commit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: create a repo with one commit so HEAD exists.
    fn setup_repo_with_commit() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Configure signature.
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create a file, stage it, and commit.
        let file_path = dir.path().join("hello.txt");
        std::fs::write(&file_path, "hello world\n").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("hello.txt")).unwrap();
            index.write().unwrap();

            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn list_commits_returns_initial_commit() {
        let (_dir, repo) = setup_repo_with_commit();
        let commits = list_commits(&repo, 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "initial commit");
        assert!(!commits[0].oid.is_empty());
        assert_eq!(commits[0].short_oid.len(), 7);
        assert!(commits[0].parent_ids.is_empty());
    }

    #[test]
    fn create_commit_works() {
        let (dir, repo) = setup_repo_with_commit();

        // Make a change, stage it, then commit.
        std::fs::write(dir.path().join("hello.txt"), "updated\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let info = create_commit(&repo, "second commit").unwrap();
        assert_eq!(info.summary, "second commit");
        assert_eq!(info.parent_ids.len(), 1);
    }

    #[test]
    fn get_commit_details_works() {
        let (_dir, repo) = setup_repo_with_commit();
        let commits = list_commits(&repo, 1).unwrap();
        let oid_str = &commits[0].oid;
        let detail = get_commit_details(&repo, oid_str).unwrap();
        assert_eq!(detail.oid, *oid_str);
        assert_eq!(detail.summary, "initial commit");
    }

    #[test]
    fn get_commit_details_bad_oid() {
        let (_dir, repo) = setup_repo_with_commit();
        let result = get_commit_details(&repo, "not-a-valid-oid");
        assert!(result.is_err());
    }

    #[test]
    fn list_commits_respects_max_count() {
        let (dir, repo) = setup_repo_with_commit();

        // Add a second commit.
        std::fs::write(dir.path().join("second.txt"), "two\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("second.txt")).unwrap();
        index.write().unwrap();
        create_commit(&repo, "second commit").unwrap();

        let one = list_commits(&repo, 1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].summary, "second commit");

        let both = list_commits(&repo, 100).unwrap();
        assert_eq!(both.len(), 2);
    }
}
