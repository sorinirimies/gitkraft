//! Remote operations — list, fetch, pull, and push.

use anyhow::{Context, Result};
use git2::Repository;
use tracing::debug;

use super::types::RemoteInfo;

/// List all configured remotes in the repository.
pub fn list_remotes(repo: &Repository) -> Result<Vec<RemoteInfo>> {
    let remote_names = repo.remotes().context("failed to list remotes")?;
    let mut remotes = Vec::with_capacity(remote_names.len());

    for name in remote_names.iter() {
        let name = name.unwrap_or("<invalid utf-8>");
        let remote = repo
            .find_remote(name)
            .with_context(|| format!("failed to find remote '{name}'"))?;

        let url = remote.url().map(String::from);

        let mut fetch_refspecs = Vec::new();
        let refspecs = remote.refspecs();
        for refspec in refspecs {
            if refspec.direction() == git2::Direction::Fetch {
                if let Some(s) = refspec.str() {
                    fetch_refspecs.push(s.to_string());
                }
            }
        }

        remotes.push(RemoteInfo {
            name: name.to_string(),
            url,
            fetch_refspecs,
        });
    }

    debug!("found {} remotes", remotes.len());
    Ok(remotes)
}

/// Fetch from a remote by name.
///
/// This works for public repositories. Repositories requiring authentication
/// will return an error — callers should handle auth setup via `git2` callbacks
/// or credential helpers.
pub fn fetch_remote(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("remote '{remote_name}' not found"))?;

    debug!("fetching from remote '{}'", remote_name);

    remote
        .fetch(&[] as &[&str], None, None)
        .with_context(|| format!("failed to fetch from remote '{remote_name}'"))?;

    Ok(())
}

/// Pull from a remote: fetch + fast-forward merge of the given branch.
///
/// If a fast-forward is not possible (diverged histories), this returns an error
/// rather than creating a merge commit — callers can handle that case with
/// [`crate::features::branches::merge_branch`].
pub fn pull(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    // Step 1: fetch
    fetch_remote(repo, remote_name)?;

    // Step 2: look up FETCH_HEAD
    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .context("FETCH_HEAD not found after fetch")?;
    let fetch_commit_oid = fetch_head
        .target()
        .context("FETCH_HEAD is not a direct reference")?;
    let fetch_commit = repo
        .find_commit(fetch_commit_oid)
        .context("failed to find FETCH_HEAD commit")?;

    // Step 3: try to fast-forward the local branch
    let refname = format!("refs/heads/{branch}");
    match repo.find_reference(&refname) {
        Ok(mut local_ref) => {
            let local_oid = local_ref
                .target()
                .context("local branch ref is not direct")?;

            // Check if fast-forward is possible
            let (ahead, behind) = repo
                .graph_ahead_behind(local_oid, fetch_commit_oid)
                .context("failed to compute ahead/behind")?;

            if behind == 0 {
                debug!(
                    "local branch '{}' is already up to date (ahead by {})",
                    branch, ahead
                );
                return Ok(());
            }

            if ahead > 0 {
                anyhow::bail!(
                    "cannot fast-forward: local branch '{branch}' has diverged \
                     ({ahead} ahead, {behind} behind). Use merge instead."
                );
            }

            // Fast-forward: update the reference
            debug!(
                "fast-forwarding '{}' from {} to {}",
                branch, local_oid, fetch_commit_oid
            );
            local_ref
                .set_target(
                    fetch_commit_oid,
                    &format!("pull: fast-forward {branch} to {fetch_commit_oid}"),
                )
                .context("failed to fast-forward branch reference")?;

            // Update the working directory
            repo.set_head(&refname)
                .context("failed to set HEAD after fast-forward")?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .context("failed to checkout HEAD after fast-forward")?;
        }
        Err(_) => {
            // Local branch doesn't exist — create it pointing at the fetched commit
            debug!(
                "local branch '{}' not found, creating at {}",
                branch, fetch_commit_oid
            );
            repo.branch(branch, &fetch_commit, false)
                .with_context(|| format!("failed to create branch '{branch}'"))?;
        }
    }

    Ok(())
}

/// Push a local branch to a remote.
///
/// This will fail without authentication for non-local remotes — that is
/// expected. Callers should configure credential helpers or push callbacks.
pub fn push(repo: &Repository, remote_name: &str, branch: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("remote '{remote_name}' not found"))?;

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");

    debug!(
        "pushing '{}' to remote '{}' with refspec '{}'",
        branch, remote_name, refspec
    );

    remote
        .push(&[&refspec], None)
        .with_context(|| format!("failed to push '{branch}' to remote '{remote_name}'"))?;

    Ok(())
}
