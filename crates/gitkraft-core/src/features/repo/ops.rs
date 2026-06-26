//! Repository-level operations — open, init, clone, and inspect.

use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;

use super::types::{RepoInfo, RepoState};

/// Open an existing repository at `path`.
///
/// Uses [`Repository::discover`] so it works when `path` is any directory
/// inside the work-tree (it will walk upwards to find `.git`).
pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path)
        .with_context(|| format!("failed to open repository at {}", path.display()))
}

/// Initialise a brand-new repository at `path`.
pub fn init_repo(path: &Path) -> Result<Repository> {
    Repository::init(path)
        .with_context(|| format!("failed to init repository at {}", path.display()))
}

/// Gather high-level information about an already-opened repository.
pub fn get_repo_info(repo: &Repository) -> Result<RepoInfo> {
    let path = repo.path().to_path_buf();
    let workdir = repo.workdir().map(|p| p.to_path_buf());
    let is_bare = repo.is_bare();
    let state: RepoState = repo.state().into();

    let head_branch = repo.head().ok().and_then(|reference| {
        if reference.is_branch() {
            reference.shorthand().map(String::from)
        } else {
            // Detached HEAD — show the short OID instead
            reference.target().map(|oid| {
                let s = oid.to_string();
                s[..7.min(s.len())].to_string()
            })
        }
    });

    Ok(RepoInfo {
        path,
        workdir,
        head_branch,
        is_bare,
        state,
    })
}

/// Checkout a specific commit by OID, leaving HEAD in detached state.
pub fn checkout_commit_detached(repo: &Repository, oid_str: &str) -> Result<()> {
    let oid = git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    repo.set_head_detached(oid)
        .context("failed to detach HEAD")?;
    repo.checkout_tree(
        commit.as_object(),
        Some(git2::build::CheckoutBuilder::new().force()),
    )
    .context("failed to checkout commit tree")?;
    Ok(())
}

/// Revert a commit by OID using `git revert --no-edit`.
pub fn revert_commit(workdir: &std::path::Path, oid_str: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(["revert", "--no-edit", oid_str])
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", stderr.trim());
    }
    Ok(())
}

/// Reset the current branch to a specific commit.
///
/// The `mode` parameter is type-safe — use [`ResetMode::Soft`],
/// [`ResetMode::Mixed`], or [`ResetMode::Hard`].
pub fn reset_to_commit(
    workdir: &std::path::Path,
    oid_str: &str,
    mode: super::types::ResetMode,
) -> Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(["reset", mode.as_flag(), oid_str])
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", stderr.trim());
    }
    Ok(())
}

/// Delete a file from the working directory.
///
/// `relative_path` is the repository-relative path (e.g. `src/main.rs`).
/// Returns an error if the file does not exist or cannot be removed.
pub fn delete_file(workdir: &std::path::Path, relative_path: &str) -> Result<()> {
    let full_path = workdir.join(relative_path);
    std::fs::remove_file(&full_path)
        .with_context(|| format!("failed to delete '{}'", full_path.display()))
}

/// Load a complete repository snapshot in one blocking call.
///
/// Opens the repository at `path`, runs all eight data-loading operations in
/// sequence, and returns a [`RepoSnapshot`] containing every field needed to
/// render the UI.  Both the GUI and TUI call this from their background
/// threads rather than duplicating the load sequence locally.
/// Default number of commits to load for a brand-new repo open.
const DEFAULT_COMMIT_COUNT: usize = 500;

pub fn load_repo_snapshot(path: &std::path::Path) -> anyhow::Result<super::types::RepoSnapshot> {
    load_repo_snapshot_with_depth(path, DEFAULT_COMMIT_COUNT)
}

/// Like [`load_repo_snapshot`] but loads at least `min_commits` commits.
///
/// Used for background refreshes so the existing scroll depth is preserved.
pub fn load_repo_snapshot_with_depth(
    path: &std::path::Path,
    min_commits: usize,
) -> anyhow::Result<super::types::RepoSnapshot> {
    let mut repo = open_repo(path)?;

    let info = get_repo_info(&repo)?;
    let branches = crate::features::branches::list_branches(&repo)?;
    let count = min_commits.max(DEFAULT_COMMIT_COUNT);
    let commits = crate::features::commits::list_commits(&repo, count)?;
    let graph_rows = crate::features::graph::build_graph(&commits);
    let unstaged = crate::features::diff::get_working_dir_file_list(&repo)?;
    let staged = crate::features::diff::get_staged_file_list(&repo)?;
    let remotes = crate::features::remotes::list_remotes(&repo)?;
    let stashes = crate::features::stash::list_stashes(&mut repo)?;

    Ok(super::types::RepoSnapshot {
        info,
        branches,
        commits,
        graph_rows,
        unstaged,
        staged,
        stashes,
        remotes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_and_open() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path()).unwrap();
        assert!(!repo.is_bare());

        let reopened = open_repo(tmp.path()).unwrap();
        assert_eq!(
            repo.path().canonicalize().unwrap(),
            reopened.path().canonicalize().unwrap(),
        );
    }

    #[test]
    fn repo_info_on_fresh_repo() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path()).unwrap();
        let info = get_repo_info(&repo).unwrap();

        assert!(!info.is_bare);
        assert_eq!(info.state, RepoState::Clean);
        // No commits yet, so HEAD is unborn — head_branch is None.
        assert!(info.head_branch.is_none());
        assert!(info.workdir.is_some());
    }

    #[test]
    fn repo_info_with_commit() {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path()).unwrap();

        // Create an initial commit so HEAD points to a branch.
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_oid = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let info = get_repo_info(&repo).unwrap();
        // git init creates branch "master" by default (unless configured otherwise).
        assert!(info.head_branch.is_some());
    }

    #[test]
    fn load_repo_snapshot_returns_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        {
            let repo = git2::Repository::init(dir.path()).unwrap();
            // Minimal setup: configure user so commit can be created
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
            drop(config);
            // Create initial commit
            let sig = repo.signature().unwrap();
            let tree_id = {
                let mut idx = repo.index().unwrap();
                idx.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
            // tree and repo both drop here, tree first (reverse declaration order)
        }

        let snapshot = load_repo_snapshot(dir.path()).unwrap();
        // At minimum the info should have workdir set
        assert!(snapshot.info.workdir.is_some());
        // graph_rows is computed from commits — both should have the same length
        assert_eq!(snapshot.commits.len(), snapshot.graph_rows.len());
    }
}
