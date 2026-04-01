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
