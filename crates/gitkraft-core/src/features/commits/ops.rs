//! Commit operations — list, create, and inspect commits.

use anyhow::{Context, Result};
use git2::Repository;
use std::collections::HashMap;

use super::types::{CommitInfo, RefKind, RefLabel};

// ── ref map ───────────────────────────────────────────────────────────────

/// Build a map of commit OID → ref labels for all refs in the repository.
///
/// Used by [`list_commits`] and [`crate::features::log::ops::get_log`] to
/// attach branch/tag information to each [`CommitInfo`].
pub(crate) fn build_ref_map(repo: &Repository) -> HashMap<git2::Oid, Vec<RefLabel>> {
    let mut map: HashMap<git2::Oid, Vec<RefLabel>> = HashMap::new();

    let head_branch: Option<String> = repo
        .head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    if let Ok(refs) = repo.references() {
        for rf in refs.flatten() {
            let full_name = match rf.name() {
                Some(n) => n.to_string(),
                None => continue,
            };
            let oid = match rf.peel_to_commit() {
                Ok(c) => c.id(),
                Err(_) => continue,
            };
            let label = if let Some(branch) = full_name.strip_prefix("refs/heads/") {
                let kind = if head_branch.as_deref() == Some(branch) {
                    RefKind::Head
                } else {
                    RefKind::LocalBranch
                };
                RefLabel {
                    name: branch.to_string(),
                    kind,
                }
            } else if let Some(rb) = full_name.strip_prefix("refs/remotes/") {
                if rb.ends_with("/HEAD") {
                    continue;
                }
                RefLabel {
                    name: rb.to_string(),
                    kind: RefKind::RemoteBranch,
                }
            } else if let Some(tag) = full_name.strip_prefix("refs/tags/") {
                RefLabel {
                    name: tag.to_string(),
                    kind: RefKind::Tag,
                }
            } else {
                continue;
            };
            map.entry(oid).or_default().push(label);
        }
    }

    // Detached HEAD: synthesise a label for the bare commit.
    if head_branch.is_none() {
        if let Ok(head) = repo.head() {
            if let Ok(commit) = head.peel_to_commit() {
                map.entry(commit.id()).or_default().push(RefLabel {
                    name: "HEAD".to_string(),
                    kind: RefKind::Head,
                });
            }
        }
    }

    // Sort each bucket: Head first, LocalBranch, RemoteBranch, Tag.
    for labels in map.values_mut() {
        labels.sort_by_key(|r| match r.kind {
            RefKind::Head => 0u8,
            RefKind::LocalBranch => 1,
            RefKind::RemoteBranch => 2,
            RefKind::Tag => 3,
        });
    }

    map
}

// ── cherry-pick ──────────────────────────────────────────────────────────
pub fn cherry_pick_commit(workdir: &std::path::Path, oid_str: &str) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .args(["cherry-pick", oid_str])
        .current_dir(workdir)
        .output()
        .context("failed to run git cherry-pick")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "cherry-pick failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Walk the history from HEAD and return up to `max_count` commits.
///
/// Commits are sorted topologically and by time (newest first).
/// Each commit's [`CommitInfo::refs`] is populated with any branch / tag /
/// HEAD labels that point directly at it.
pub fn list_commits(repo: &Repository, max_count: usize) -> Result<Vec<CommitInfo>> {
    let ref_map = build_ref_map(repo);

    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    // Push ALL refs so branches, tags, and remotes appear in the graph
    // (not just commits reachable from HEAD).
    revwalk
        .push_head()
        .context("failed to push HEAD to revwalk")?;
    if let Ok(refs) = repo.references() {
        for r in refs.flatten() {
            if let Some(oid) = r.target() {
                let _ = revwalk.push(oid);
            }
        }
    }
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
        let mut info = CommitInfo::from_git2_commit(&commit);
        // Don't store full multi-line commit bodies in the batch list —
        // only the summary (first line) is shown in the log view.
        // The full message is re-loaded on demand via get_commit_details().
        info.message = String::new();
        if let Some(refs) = ref_map.get(&oid) {
            info.refs = refs.clone();
        }
        commits.push(info);
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
    #[test]
    fn cherry_pick_on_nonexistent_repo_returns_error() {
        let result = super::cherry_pick_commit(std::path::Path::new("/nonexistent"), "abc1234");
        assert!(result.is_err());
    }

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

    #[test]
    fn list_commits_attaches_head_ref_to_tip() {
        let (_dir, repo) = setup_repo_with_commit();
        let commits = list_commits(&repo, 10).unwrap();
        // The single commit should carry the HEAD branch label.
        assert!(!commits[0].refs.is_empty(), "tip commit should have refs");
        assert!(
            commits[0]
                .refs
                .iter()
                .any(|r| r.kind == crate::features::commits::types::RefKind::Head),
            "tip commit should have a Head ref"
        );
    }

    #[test]
    fn list_commits_non_tip_commits_have_no_refs() {
        let (dir, repo) = setup_repo_with_commit();
        // Add a second commit — only the tip should have refs.
        std::fs::write(dir.path().join("second.txt"), "two\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("second.txt")).unwrap();
        index.write().unwrap();
        create_commit(&repo, "second commit").unwrap();

        let commits = list_commits(&repo, 100).unwrap();
        assert_eq!(commits.len(), 2);
        // Tip (newest) should have refs
        assert!(!commits[0].refs.is_empty());
        // Parent (older) should have no refs
        assert!(
            commits[1].refs.is_empty(),
            "non-tip commits should have empty refs"
        );
    }

    #[test]
    fn build_ref_map_includes_tags() {
        let (_dir, repo) = setup_repo_with_commit();
        // Create a lightweight tag on HEAD
        let head_oid = repo.head().unwrap().target().unwrap();
        let head_commit = repo.find_commit(head_oid).unwrap();
        repo.tag_lightweight("v1.0.0", head_commit.as_object(), false)
            .unwrap();

        let ref_map = build_ref_map(&repo);
        let labels = ref_map.get(&head_oid).expect("HEAD should have refs");
        assert!(
            labels
                .iter()
                .any(|r| r.name == "v1.0.0"
                    && r.kind == crate::features::commits::types::RefKind::Tag),
            "tag should appear in ref map"
        );
    }
}
