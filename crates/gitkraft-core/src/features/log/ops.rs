//! Log browsing and commit search operations.

use anyhow::{Context, Result};
use git2::Repository;

use crate::features::commits::CommitInfo;

/// Retrieve the commit log, optionally filtered by author and/or message substring.
///
/// Walks from HEAD, returning at most `max_count` commits that match every
/// supplied filter (filters are AND-ed together).
pub fn get_log(
    repo: &Repository,
    max_count: usize,
    filter_author: Option<&str>,
    filter_message: Option<&str>,
) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("failed to push HEAD")?;
    revwalk
        .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .context("failed to set sorting")?;

    let author_lower = filter_author.map(|s| s.to_lowercase());
    let message_lower = filter_message.map(|s| s.to_lowercase());

    let mut results = Vec::with_capacity(max_count.min(256));

    for oid_result in revwalk {
        if results.len() >= max_count {
            break;
        }

        let oid = oid_result.context("revwalk iteration error")?;
        let commit = repo
            .find_commit(oid)
            .context("failed to find commit during log walk")?;

        // ── Apply author filter ───────────────────────────────────────────
        if let Some(ref needle) = author_lower {
            let author = commit.author();
            let name = author.name().unwrap_or("").to_lowercase();
            let email = author.email().unwrap_or("").to_lowercase();
            if !name.contains(needle.as_str()) && !email.contains(needle.as_str()) {
                continue;
            }
        }

        // ── Apply message filter ──────────────────────────────────────────
        if let Some(ref needle) = message_lower {
            let msg = commit.message().unwrap_or("").to_lowercase();
            if !msg.contains(needle.as_str()) {
                continue;
            }
        }

        results.push(CommitInfo::from_git2_commit(&commit));
    }

    Ok(results)
}

/// Free-text search across commit summary, full message, author name, author
/// email, and OID.
///
/// Returns at most `max_count` commits where `query` appears (case-insensitive)
/// in any of those fields.
pub fn search_commits(repo: &Repository, query: &str, max_count: usize) -> Result<Vec<CommitInfo>> {
    let needle = query.to_lowercase();

    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("failed to push HEAD")?;
    revwalk
        .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .context("failed to set sorting")?;

    let mut results = Vec::with_capacity(max_count.min(256));

    for oid_result in revwalk {
        if results.len() >= max_count {
            break;
        }

        let oid = oid_result.context("revwalk iteration error")?;
        let commit = repo
            .find_commit(oid)
            .context("failed to find commit during search")?;

        let summary = commit.summary().unwrap_or("").to_lowercase();
        let message = commit.message().unwrap_or("").to_lowercase();
        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_lowercase();
        let author_email = author.email().unwrap_or("").to_lowercase();
        let oid_str = oid.to_string();

        if summary.contains(&needle)
            || message.contains(&needle)
            || author_name.contains(&needle)
            || author_email.contains(&needle)
            || oid_str.contains(&needle)
        {
            results.push(CommitInfo::from_git2_commit(&commit));
        }
    }

    Ok(results)
}

/// Return commits that touched `file_path`, newest-first, up to `max_count`.
///
/// A commit "touches" a file if the file appears in its diff against the
/// first parent (or against the empty tree for root commits).
pub fn file_history(
    repo: &Repository,
    file_path: &str,
    max_count: usize,
) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("failed to push HEAD")?;
    revwalk
        .set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)
        .context("failed to set sorting")?;

    let mut results = Vec::new();

    for oid_result in revwalk {
        if results.len() >= max_count {
            break;
        }
        let oid = oid_result.context("revwalk iteration error")?;
        let commit = repo
            .find_commit(oid)
            .context("failed to find commit during file history walk")?;

        if commit_touches_file(repo, &commit, file_path) {
            results.push(CommitInfo::from_git2_commit(&commit));
        }
    }

    Ok(results)
}

/// Return `true` if `commit` introduces any change to `file_path` relative to
/// its first parent (or the empty tree for a root commit).
fn commit_touches_file(repo: &Repository, commit: &git2::Commit<'_>, file_path: &str) -> bool {
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let parent_tree = commit.parents().next().and_then(|p| p.tree().ok());

    let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
        Ok(d) => d,
        Err(_) => return false,
    };

    diff.deltas().any(|d| {
        d.new_file()
            .path()
            .and_then(|p| p.to_str())
            .map(|s| s == file_path)
            .unwrap_or(false)
            || d.old_file()
                .path()
                .and_then(|p| p.to_str())
                .map(|s| s == file_path)
                .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn setup_repo_with_commits() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Alice Test").unwrap();
            config.set_str("user.email", "alice@example.com").unwrap();
        }

        // First commit
        let c1 = {
            std::fs::write(dir.path().join("a.txt"), "aaa\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("a.txt")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "first commit", &tree, &[])
                .unwrap()
        };

        // Second commit by a different "author"
        {
            let commit1 = repo.find_commit(c1).unwrap();
            std::fs::write(dir.path().join("b.txt"), "bbb\n").unwrap();
            let mut index2 = repo.index().unwrap();
            index2.add_path(Path::new("b.txt")).unwrap();
            index2.write().unwrap();
            let tree_oid2 = index2.write_tree().unwrap();
            let tree2 = repo.find_tree(tree_oid2).unwrap();
            let sig2 = git2::Signature::now("Bob Builder", "bob@example.com").unwrap();
            repo.commit(
                Some("HEAD"),
                &sig2,
                &sig2,
                "second commit by Bob",
                &tree2,
                &[&commit1],
            )
            .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn get_log_no_filters() {
        let (_dir, repo) = setup_repo_with_commits();
        let log = get_log(&repo, 100, None, None).unwrap();
        assert_eq!(log.len(), 2);
        // Newest first
        assert_eq!(log[0].summary, "second commit by Bob");
        assert_eq!(log[1].summary, "first commit");
    }

    #[test]
    fn get_log_filter_author() {
        let (_dir, repo) = setup_repo_with_commits();
        let log = get_log(&repo, 100, Some("bob"), None).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].summary, "second commit by Bob");
    }

    #[test]
    fn get_log_filter_message() {
        let (_dir, repo) = setup_repo_with_commits();
        let log = get_log(&repo, 100, None, Some("first")).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].summary, "first commit");
    }

    #[test]
    fn get_log_respects_max_count() {
        let (_dir, repo) = setup_repo_with_commits();
        let log = get_log(&repo, 1, None, None).unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn search_commits_by_author() {
        let (_dir, repo) = setup_repo_with_commits();
        let results = search_commits(&repo, "alice", 100).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].summary, "first commit");
    }

    #[test]
    fn search_commits_by_message() {
        let (_dir, repo) = setup_repo_with_commits();
        let results = search_commits(&repo, "second", 100).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].author_name, "Bob Builder");
    }

    #[test]
    fn search_commits_case_insensitive() {
        let (_dir, repo) = setup_repo_with_commits();
        let results = search_commits(&repo, "BOB", 100).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_commits_empty_query_returns_all() {
        let (_dir, repo) = setup_repo_with_commits();
        let results = search_commits(&repo, "", 100).unwrap();
        // Empty string matches everything
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_commits_no_match() {
        let (_dir, repo) = setup_repo_with_commits();
        let results = search_commits(&repo, "zzzznonexistent", 100).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn file_history_returns_only_touching_commits() {
        let (dir, repo) = setup_repo_with_commits();

        // Add a third commit that only touches a.txt
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            std::fs::write(dir.path().join("a.txt"), "updated\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("a.txt")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "update a.txt only",
                &tree,
                &[&head],
            )
            .unwrap();
        }

        // a.txt was touched in commit 1 ("first commit") and commit 3
        let hist = file_history(&repo, "a.txt", 100).unwrap();
        assert_eq!(hist.len(), 2, "expected 2 commits touching a.txt");
        assert_eq!(hist[0].summary, "update a.txt only");
        assert_eq!(hist[1].summary, "first commit");

        // b.txt was only touched in commit 2
        let hist_b = file_history(&repo, "b.txt", 100).unwrap();
        assert_eq!(hist_b.len(), 1);
        assert_eq!(hist_b[0].summary, "second commit by Bob");
    }

    #[test]
    fn file_history_respects_max_count() {
        let (dir, repo) = setup_repo_with_commits();

        // Add second touch of a.txt
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            std::fs::write(dir.path().join("a.txt"), "v2\n").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("a.txt")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "touch a again", &tree, &[&head])
                .unwrap();
        }

        let hist = file_history(&repo, "a.txt", 1).unwrap();
        assert_eq!(hist.len(), 1);
    }

    #[test]
    fn file_history_nonexistent_file_returns_empty() {
        let (_dir, repo) = setup_repo_with_commits();
        let hist = file_history(&repo, "nonexistent.txt", 100).unwrap();
        assert!(hist.is_empty());
    }
}
