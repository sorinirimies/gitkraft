//! Branch operations — list, create, delete, checkout, and merge branches.

use anyhow::{bail, Context, Result};
use git2::{BranchType as Git2BranchType, Repository};
use tracing::debug;

use super::types::{BranchInfo, BranchType};

/// List all branches (local and remote) in the repository.
pub fn list_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let mut branches = Vec::new();

    let head_ref = repo.head().ok();
    let head_name = head_ref
        .as_ref()
        .and_then(|r| r.shorthand().map(String::from));

    for branch_result in repo.branches(None)? {
        let (branch, bt) = branch_result?;

        let name = branch.name()?.unwrap_or("<invalid utf-8>").to_string();

        let branch_type = match bt {
            Git2BranchType::Local => BranchType::Local,
            Git2BranchType::Remote => BranchType::Remote,
        };

        let is_head = match branch_type {
            BranchType::Local => head_name.as_deref() == Some(name.as_str()),
            BranchType::Remote => false,
        };

        let target_oid = branch.get().target().map(|oid| oid.to_string());

        branches.push(BranchInfo {
            name,
            branch_type,
            is_head,
            target_oid,
        });
    }

    debug!("listed {} branches", branches.len());
    Ok(branches)
}

/// Create a new local branch at HEAD with the given name.
///
/// Returns the newly created [`BranchInfo`].
pub fn create_branch(repo: &Repository, name: &str) -> Result<BranchInfo> {
    let head_ref = repo
        .head()
        .context("HEAD not found — is this an empty repository?")?;
    let commit = head_ref
        .peel_to_commit()
        .context("HEAD does not point to a commit")?;

    let branch = repo
        .branch(name, &commit, false)
        .with_context(|| format!("failed to create branch '{name}'"))?;

    let target_oid = branch.get().target().map(|oid| oid.to_string());

    debug!(name, "created branch");
    Ok(BranchInfo {
        name: name.to_string(),
        branch_type: BranchType::Local,
        is_head: false,
        target_oid,
    })
}

/// Delete a local branch by name.
///
/// Refuses to delete the currently checked-out branch.
pub fn delete_branch(repo: &Repository, name: &str) -> Result<()> {
    let mut branch = repo
        .find_branch(name, Git2BranchType::Local)
        .with_context(|| format!("local branch '{name}' not found"))?;

    if branch.is_head() {
        bail!("cannot delete the currently checked-out branch '{name}'");
    }

    branch
        .delete()
        .with_context(|| format!("failed to delete branch '{name}'"))?;
    debug!(name, "deleted branch");
    Ok(())
}

/// Checkout an existing local branch by name.
///
/// Sets HEAD to the branch reference and updates the working directory.
pub fn checkout_branch(repo: &Repository, name: &str) -> Result<()> {
    let refname = format!("refs/heads/{name}");

    // Make sure the branch actually exists
    repo.find_branch(name, Git2BranchType::Local)
        .with_context(|| format!("local branch '{name}' not found"))?;

    repo.set_head(&refname)
        .with_context(|| format!("failed to set HEAD to '{refname}'"))?;

    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .with_context(|| format!("failed to checkout branch '{name}'"))?;

    debug!(name, "checked out branch");
    Ok(())
}

/// Merge a source branch into the current HEAD branch.
///
/// If the merge can be fast-forwarded, it does so. If it results in a normal
/// merge (no conflicts), an automatic merge commit is created. If there are
/// conflicts, an error is returned and the repository is left in a merging
/// state so the user can resolve conflicts manually.
pub fn merge_branch(repo: &Repository, source_branch: &str) -> Result<()> {
    // Look up the source branch reference and its annotated commit.
    let branch = repo
        .find_branch(source_branch, Git2BranchType::Local)
        .with_context(|| format!("local branch '{source_branch}' not found"))?;

    let source_ref = branch.get();
    let source_oid = source_ref
        .target()
        .with_context(|| format!("branch '{source_branch}' has no target OID"))?;

    let annotated_commit = repo
        .find_annotated_commit(source_oid)
        .context("failed to find annotated commit for source branch")?;

    // Perform merge analysis.
    let (analysis, _preference) = repo
        .merge_analysis(&[&annotated_commit])
        .context("merge analysis failed")?;

    if analysis.is_up_to_date() {
        debug!(source_branch, "already up to date");
        return Ok(());
    }

    if analysis.is_fast_forward() {
        debug!(source_branch, "fast-forwarding");
        // Fast-forward: just move the current branch reference.
        let refname = format!("refs/heads/{}", head_branch_name(repo)?);
        let msg = format!("Fast-forward merge of '{source_branch}'");
        repo.reference(&refname, source_oid, true, &msg)?;
        repo.set_head(&refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        return Ok(());
    }

    if analysis.is_normal() {
        debug!(source_branch, "performing normal merge");

        // Perform the actual merge (writes conflicts to index if any).
        repo.merge(&[&annotated_commit], None, None)
            .context("merge failed")?;

        // Check for conflicts.
        let index = repo.index().context("failed to read index after merge")?;
        if index.has_conflicts() {
            bail!(
                "merge of '{source_branch}' resulted in conflicts — resolve them and commit manually"
            );
        }

        // No conflicts — create the merge commit automatically.
        let sig = repo
            .signature()
            .or_else(|_| git2::Signature::now("GitKraft User", "user@gitkraft.local"))
            .context("failed to obtain signature")?;

        let mut index = repo.index().context("failed to read index")?;
        let tree_oid = index.write_tree().context("failed to write merged tree")?;
        let tree = repo
            .find_tree(tree_oid)
            .context("failed to find merged tree")?;

        let head_commit = repo
            .head()?
            .peel_to_commit()
            .context("HEAD does not point to a commit")?;

        let source_commit = repo
            .find_commit(source_oid)
            .context("failed to find source commit")?;

        let message = format!("Merge branch '{source_branch}'");
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[&head_commit, &source_commit],
        )
        .context("failed to create merge commit")?;

        // Clean up merge state.
        repo.cleanup_state()
            .context("failed to clean up merge state")?;

        debug!(source_branch, "merge commit created");
        return Ok(());
    }

    bail!("merge analysis returned an unexpected result for branch '{source_branch}'");
}

/// Helper: get the short name of the branch HEAD points to.
fn head_branch_name(repo: &Repository) -> Result<String> {
    let head = repo.head().context("HEAD not found")?;
    let name = head
        .shorthand()
        .context("HEAD is not a symbolic reference (detached HEAD?)")?
        .to_string();
    Ok(name)
}

// ── subprocess helper ─────────────────────────────────────────────────────────

fn run_git(workdir: &std::path::Path, args: &[&str]) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(workdir)
        .args(args)
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}", stderr.trim());
    }
    Ok(())
}

// ── new public functions ──────────────────────────────────────────────────────

/// Rename a local branch.
pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<()> {
    let mut branch = repo
        .find_branch(old_name, Git2BranchType::Local)
        .with_context(|| format!("branch '{old_name}' not found"))?;
    branch
        .rename(new_name, false)
        .with_context(|| format!("failed to rename '{old_name}' → '{new_name}'"))?;
    debug!(old_name, new_name, "renamed branch");
    Ok(())
}

/// Create a new local branch pointing at a specific commit OID.
pub fn create_branch_at_commit(repo: &Repository, name: &str, oid_str: &str) -> Result<()> {
    let oid = git2::Oid::from_str(oid_str).context("invalid OID")?;
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid_str} not found"))?;
    repo.branch(name, &commit, false)
        .with_context(|| format!("failed to create branch '{name}' at {oid_str}"))?;
    debug!(name, oid_str, "created branch at commit");
    Ok(())
}

/// Push a local branch to a remote using `git push`.
///
/// Uses the system `git` binary so that the user's configured credential
/// helpers (SSH agent, git-credential-manager, etc.) are respected.
pub fn push_branch(workdir: &std::path::Path, branch: &str, remote: &str) -> Result<()> {
    run_git(workdir, &["push", remote, branch])
}

/// Delete a remote branch using `git push <remote> --delete <branch>`.
///
/// `full_name` is the remote-tracking branch name (e.g. `origin/feature-x`).
/// The function extracts the remote and branch parts automatically.
pub fn delete_remote_branch(workdir: &std::path::Path, full_name: &str) -> Result<()> {
    let (remote, branch) = full_name.split_once('/').with_context(|| {
        format!("invalid remote branch name '{full_name}' — expected 'remote/branch'")
    })?;
    run_git(workdir, &["push", remote, "--delete", branch])
}

/// Checkout a remote branch by creating a local tracking branch.
///
/// `full_name` is the remote-tracking branch name (e.g. `origin/feature-x`).
/// Creates a local branch named `feature-x` that tracks `origin/feature-x`.
pub fn checkout_remote_branch(workdir: &std::path::Path, full_name: &str) -> Result<()> {
    let (_remote, branch) = full_name.split_once('/').with_context(|| {
        format!("invalid remote branch name '{full_name}' — expected 'remote/branch'")
    })?;
    run_git(workdir, &["checkout", "-b", branch, "--track", full_name])
}

/// Pull the current branch from a remote with `--rebase`.
pub fn pull_rebase(workdir: &std::path::Path, remote: &str) -> Result<()> {
    run_git(workdir, &["pull", "--rebase", remote])
}

/// Rebase the current HEAD onto `target` (branch name or OID).
pub fn rebase_onto(workdir: &std::path::Path, target: &str) -> Result<()> {
    run_git(workdir, &["rebase", target])
}

/// Create a lightweight Git tag pointing at the given OID.
pub fn create_tag(repo: &Repository, name: &str, oid_str: &str) -> Result<()> {
    let oid = git2::Oid::from_str(oid_str).context("invalid OID")?;
    let object = repo
        .find_object(oid, None)
        .with_context(|| format!("object {oid_str} not found"))?;
    repo.tag_lightweight(name, &object, false)
        .with_context(|| format!("failed to create lightweight tag '{name}'"))?;
    debug!(name, oid_str, "created lightweight tag");
    Ok(())
}

/// Create an annotated Git tag with a tagger signature and message pointing at the given OID.
pub fn create_annotated_tag(
    repo: &Repository,
    name: &str,
    message: &str,
    oid_str: &str,
) -> Result<()> {
    let oid = git2::Oid::from_str(oid_str).context("invalid OID")?;
    let object = repo
        .find_object(oid, None)
        .with_context(|| format!("object {oid_str} not found"))?;
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("GitKraft User", "user@gitkraft.local"))
        .context("failed to obtain signature")?;
    repo.tag(name, &object, &sig, message, false)
        .with_context(|| format!("failed to create annotated tag '{name}'"))?;
    debug!(name, oid_str, "created annotated tag");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_repo_with_commit() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
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
    fn list_branches_shows_main() {
        let (_dir, repo) = setup_repo_with_commit();
        let branches = list_branches(&repo).unwrap();
        assert!(!branches.is_empty());
        assert!(branches.iter().any(|b| b.is_head));
    }

    #[test]
    fn create_and_delete_branch() {
        let (_dir, repo) = setup_repo_with_commit();
        let branch = create_branch(&repo, "feature-test").unwrap();
        assert_eq!(branch.name, "feature-test");
        assert!(!branch.is_head);

        delete_branch(&repo, "feature-test").unwrap();
        let branches = list_branches(&repo).unwrap();
        assert!(!branches.iter().any(|b| b.name == "feature-test"));
    }

    #[test]
    fn checkout_branch_switches_head() {
        let (_dir, repo) = setup_repo_with_commit();
        create_branch(&repo, "new-branch").unwrap();
        checkout_branch(&repo, "new-branch").unwrap();
        let branches = list_branches(&repo).unwrap();
        let head = branches.iter().find(|b| b.is_head).unwrap();
        assert_eq!(head.name, "new-branch");
    }

    #[test]
    fn delete_head_branch_fails() {
        let (_dir, repo) = setup_repo_with_commit();
        let branches = list_branches(&repo).unwrap();
        let head = branches.iter().find(|b| b.is_head).unwrap();
        let result = delete_branch(&repo, &head.name);
        assert!(result.is_err());
    }
}
