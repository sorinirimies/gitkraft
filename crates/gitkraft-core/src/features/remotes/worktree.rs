//! Checking a pull request out into its own worktree, so the main checkout is left alone.

use std::path::Path;

use anyhow::{Context, Result};

use super::transport::run_git;

/// Fetches `remote_ref` of `remote` into the local branch `branch` (overwriting it: it is a
/// review branch, not yours) and adds a worktree for it at `dest`.
///
/// `workdir` is the main checkout. If `dest` already holds that worktree, it is updated in place.
pub fn add_from_remote_ref(
    workdir: &Path,
    remote: &str,
    remote_ref: &str,
    branch: &str,
    dest: &Path,
) -> Result<()> {
    let dest_str = dest
        .to_str()
        .context("the worktree path is not valid UTF-8")?;
    if dest.join(".git").exists() {
        // The worktree exists: move its branch to the new tip.
        run_git(
            workdir,
            &[
                "fetch",
                remote,
                &format!("+{remote_ref}:refs/remotes/{remote}/{branch}"),
            ],
        )?;
        return run_git(
            dest,
            &[
                "reset",
                "--hard",
                &format!("refs/remotes/{remote}/{branch}"),
            ],
        );
    }
    run_git(
        workdir,
        &[
            "fetch",
            remote,
            &format!("+{remote_ref}:refs/heads/{branch}"),
        ],
    )?;
    run_git(workdir, &["worktree", "add", dest_str, branch])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn commit_file(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).expect("write");
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", name]);
    }

    #[test]
    fn a_pull_request_head_becomes_a_worktree_and_updates_in_place() {
        let root = tempfile::tempdir().expect("tmp");
        let origin = root.path().join("origin");
        let main = root.path().join("main");
        std::fs::create_dir_all(&origin).expect("mk");
        git(&origin, &["init", "-q"]);
        commit_file(&origin, "a.txt", "one");
        // a "pull request" head ref in the origin
        git(&origin, &["checkout", "-qb", "feature"]);
        commit_file(&origin, "pr.txt", "pr one");
        git(&origin, &["update-ref", "refs/pull/7/head", "HEAD"]);
        git(&origin, &["checkout", "-q", "-"]);
        git(
            root.path(),
            &[
                "clone",
                "-q",
                origin.to_str().expect("p"),
                main.to_str().expect("p"),
            ],
        );

        let dest = root.path().join("main-pr-7");
        add_from_remote_ref(&main, "origin", "refs/pull/7/head", "gitkraft/pr-7", &dest)
            .expect("worktree");
        assert_eq!(
            std::fs::read_to_string(dest.join("pr.txt")).expect("pr file"),
            "pr one"
        );
        assert!(
            !main.join("pr.txt").exists(),
            "the main checkout is untouched"
        );

        // the PR gets a new commit
        git(&origin, &["checkout", "-q", "feature"]);
        commit_file(&origin, "pr.txt", "pr two");
        git(&origin, &["update-ref", "refs/pull/7/head", "HEAD"]);
        add_from_remote_ref(&main, "origin", "refs/pull/7/head", "gitkraft/pr-7", &dest)
            .expect("update");
        assert_eq!(
            std::fs::read_to_string(dest.join("pr.txt")).expect("pr file"),
            "pr two"
        );
    }

    #[test]
    fn a_missing_ref_or_remote_is_an_error() {
        let root = tempfile::tempdir().expect("tmp");
        git(root.path(), &["init", "-q"]);
        let err = add_from_remote_ref(
            root.path(),
            "origin",
            "refs/pull/1/head",
            "b",
            &root.path().join("w"),
        )
        .expect_err("no remote");
        assert!(!err.to_string().is_empty());
    }
}
