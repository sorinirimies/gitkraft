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

/// Cherry-pick a commit by OID onto the current branch.
pub fn cherry_pick_commit(workdir: &std::path::Path, oid_str: &str) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(["cherry-pick", oid_str])
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
/// `mode` must be one of `"soft"`, `"mixed"`, or `"hard"`:
/// - **soft**  — moves HEAD; staged + working-directory changes are kept.
/// - **mixed** — moves HEAD and unstages changes; working directory is kept.
/// - **hard**  — moves HEAD and discards all uncommitted changes permanently.
pub fn reset_to_commit(workdir: &std::path::Path, oid_str: &str, mode: &str) -> Result<()> {
    let flag = format!("--{mode}");
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(["reset", &flag, oid_str])
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

/// Retrieve the content of a file at a specific commit.
///
/// Returns the file content as a UTF-8 string. Returns an error if the file
/// doesn't exist at that commit or isn't valid UTF-8.
pub fn get_file_at_commit(
    repo: &Repository,
    oid_str: &str,
    file_path: &str,
) -> anyhow::Result<String> {
    let oid = git2::Oid::from_str(oid_str).with_context(|| format!("invalid OID: {oid_str}"))?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    let tree = commit.tree().context("commit has no tree")?;

    let entry = tree
        .get_path(std::path::Path::new(file_path))
        .with_context(|| format!("file '{file_path}' not found at commit {oid_str}"))?;

    let blob = repo
        .find_blob(entry.id())
        .with_context(|| format!("could not read blob for '{file_path}'"))?;

    let content = std::str::from_utf8(blob.content())
        .with_context(|| format!("file '{file_path}' is not valid UTF-8"))?;

    Ok(content.to_string())
}

/// Load a complete repository snapshot in one blocking call.
///
/// Opens the repository at `path`, runs all eight data-loading operations in
/// sequence, and returns a [`RepoSnapshot`] containing every field needed to
/// render the UI.  Both the GUI and TUI call this from their background
/// threads rather than duplicating the load sequence locally.
pub fn load_repo_snapshot(path: &std::path::Path) -> anyhow::Result<super::types::RepoSnapshot> {
    let mut repo = open_repo(path)?;

    let info = get_repo_info(&repo)?;
    let branches = crate::features::branches::list_branches(&repo)?;
    let commits = crate::features::commits::list_commits(&repo, 500)?;
    let graph_rows = crate::features::graph::build_graph(&commits);
    let unstaged = crate::features::diff::get_working_dir_diff(&repo)?;
    let staged = crate::features::diff::get_staged_diff(&repo)?;
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

    fn setup_repo_with_commit() -> (TempDir, Repository) {
        let tmp = TempDir::new().unwrap();
        let repo = init_repo(tmp.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        std::fs::write(tmp.path().join("file.txt"), "hello\n").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        (tmp, repo)
    }

    #[test]
    fn get_file_at_commit_returns_content() {
        let (_dir, repo) = setup_repo_with_commit();
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let content = get_file_at_commit(&repo, &head_oid, "file.txt").unwrap();
        assert_eq!(content, "hello\n");
    }

    #[test]
    fn get_file_at_commit_not_found() {
        let (_dir, repo) = setup_repo_with_commit();
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let result = get_file_at_commit(&repo, &head_oid, "nonexistent.txt");
        assert!(result.is_err());
    }
}
