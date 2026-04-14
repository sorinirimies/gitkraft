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

/// Clone a remote repository from `url` into `path`.
///
/// This performs a plain HTTPS/SSH clone.  Authentication is **not** configured
/// here — it will work for public repos and fail for private ones that require
/// credentials.
pub fn clone_repo(url: &str, path: &Path) -> Result<Repository> {
    Repository::clone(url, path)
        .with_context(|| format!("failed to clone '{url}' into {}", path.display()))
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
}
