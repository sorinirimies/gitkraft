//! Planning the checkout of a pull request into a worktree: which remote, which ref, where.
//! The git calls live in `gitkraft-core`; this part is pure.

use std::path::{Path, PathBuf};

use gitkraft_platforms::{PlatformKind, RepoId, parse_remote_url};

/// What to fetch and where to put it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The local remote that points at the PR's repository.
    pub remote: String,
    /// The ref on that remote holding the PR head.
    pub remote_ref: String,
    /// Local branch name for the review branch.
    pub branch: String,
    /// The worktree directory.
    pub dest: PathBuf,
}

/// The ref a platform keeps a pull request's head under.
pub fn head_ref(kind: PlatformKind, number: u64) -> String {
    match kind {
        PlatformKind::GitLab => format!("refs/merge-requests/{number}/head"),
        PlatformKind::GitHub | PlatformKind::Gitea => format!("refs/pull/{number}/head"),
    }
}

/// The remote (by name) whose URL is `repo`; `origin` wins when several match.
pub fn remote_for(remotes: &[(String, String)], repo: &RepoId) -> Option<String> {
    let matching: Vec<&(String, String)> = remotes
        .iter()
        .filter(|(_, url)| parse_remote_url(url).is_some_and(|r| r == *repo))
        .collect();
    matching
        .iter()
        .find(|(n, _)| n == "origin")
        .or_else(|| matching.first())
        .map(|(n, _)| n.clone())
}

/// Plans the checkout of PR `number` of `repo`, next to the checkout at `workdir`
/// (`<parent>/<dir>-pr-<number>`).
pub fn plan(
    kind: PlatformKind,
    repo: &RepoId,
    number: u64,
    remotes: &[(String, String)],
    workdir: &Path,
) -> Option<Plan> {
    let remote = remote_for(remotes, repo)?;
    let name = workdir.file_name()?.to_string_lossy().to_string();
    let parent = workdir.parent()?;
    Some(Plan {
        remote,
        remote_ref: head_ref(kind, number),
        branch: format!("gitkraft/pr-{number}"),
        dest: parent.join(format!("{name}-pr-{number}")),
    })
}

/// Local names and URLs of the remotes of the checkout at `path`.
pub fn remotes_of(path: &Path) -> Result<Vec<(String, String)>, String> {
    let repo = git2::Repository::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let remotes =
        gitkraft_core::features::remotes::list_remotes(&repo).map_err(|e| e.to_string())?;
    Ok(remotes
        .into_iter()
        .filter_map(|r| Some((r.name, r.url?)))
        .collect())
}

/// Fetches PR `number` of `repo` and adds (or updates) its worktree; returns what to tell the user.
pub fn checkout_worktree(
    kind: PlatformKind,
    repo: &RepoId,
    number: u64,
    workdir: &Path,
) -> Result<String, String> {
    let remotes = remotes_of(workdir)?;
    let plan = plan(kind, repo, number, &remotes, workdir).ok_or_else(|| {
        format!(
            "{} has no remote that points at {}",
            workdir.display(),
            repo.key()
        )
    })?;
    gitkraft_core::features::remotes::worktree::add_from_remote_ref(
        workdir,
        &plan.remote,
        &plan.remote_ref,
        &plan.branch,
        &plan.dest,
    )
    .map_err(|e| format!("{e:#}"))?;
    Ok(format!(
        "#{number} is checked out in {}",
        plan.dest.display()
    ))
}

/// Which open checkouts belong to which project: `(path, remote URLs)` per open tab.
pub fn local_checkouts(tabs: &[(Option<PathBuf>, Vec<String>)]) -> Vec<(RepoId, PathBuf)> {
    let mut out: Vec<(RepoId, PathBuf)> = Vec::new();
    for (path, urls) in tabs {
        let Some(path) = path else { continue };
        for repo in urls.iter().filter_map(|u| parse_remote_url(u)) {
            if !out.iter().any(|(r, _)| *r == repo) {
                out.push((repo, path.clone()));
            }
        }
    }
    out
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

    #[test]
    fn remotes_are_listed_and_a_repo_without_a_matching_remote_is_an_error() {
        let root = tempfile::tempdir().expect("tmp");
        let main = root.path().join("main");
        std::fs::create_dir_all(&main).expect("mk");
        git(&main, &["init", "-q"]);
        git(
            &main,
            &["remote", "add", "origin", "git@github.com:o/r.git"],
        );
        git(&main, &["remote", "add", "local", "/srv/git/r"]);
        assert_eq!(
            remotes_of(&main).expect("remotes"),
            [
                ("local".to_string(), "/srv/git/r".to_string()),
                ("origin".to_string(), "git@github.com:o/r.git".to_string())
            ]
        );
        let other = RepoId::new("github.com", "x", "y");
        let err = checkout_worktree(PlatformKind::GitHub, &other, 3, &main).expect_err("no remote");
        assert!(
            err.contains("no remote that points at github.com/x/y"),
            "{err}"
        );
        assert!(remotes_of(&root.path().join("nope")).is_err());
    }

    #[test]
    fn open_tabs_map_projects_to_their_first_checkout() {
        let tabs = vec![
            (
                Some(PathBuf::from("/a")),
                vec!["git@github.com:o/r.git".to_string(), "/srv/x".to_string()],
            ),
            (None, vec!["git@github.com:o/none.git".to_string()]),
            (
                Some(PathBuf::from("/b")),
                vec![
                    "https://github.com/o/r".to_string(),
                    "git@gitlab.com:g/p.git".to_string(),
                ],
            ),
        ];
        let got = local_checkouts(&tabs);
        assert_eq!(
            got,
            [
                (RepoId::new("github.com", "o", "r"), PathBuf::from("/a")),
                (RepoId::new("gitlab.com", "g", "p"), PathBuf::from("/b")),
            ]
        );
        assert!(local_checkouts(&[]).is_empty());
    }

    fn remotes() -> Vec<(String, String)> {
        vec![
            ("fork".into(), "git@github.com:me/gitkraft.git".into()),
            (
                "up".into(),
                "https://github.com/sorinirimies/gitkraft.git".into(),
            ),
            (
                "origin".into(),
                "git@github.com:sorinirimies/gitkraft.git".into(),
            ),
            ("local".into(), "/srv/git/gitkraft".into()),
        ]
    }

    #[test]
    fn refs_differ_per_platform() {
        assert_eq!(head_ref(PlatformKind::GitHub, 7), "refs/pull/7/head");
        assert_eq!(head_ref(PlatformKind::Gitea, 7), "refs/pull/7/head");
        assert_eq!(
            head_ref(PlatformKind::GitLab, 7),
            "refs/merge-requests/7/head"
        );
    }

    #[test]
    fn the_remote_of_the_repo_is_found_and_origin_wins_over_other_urls_of_it() {
        let id = RepoId::new("github.com", "sorinirimies", "gitkraft");
        assert_eq!(remote_for(&remotes(), &id).as_deref(), Some("origin"));
        assert_eq!(remote_for(&remotes()[..2], &id).as_deref(), Some("up"));
        assert_eq!(
            remote_for(&remotes(), &RepoId::new("github.com", "me", "gitkraft")).as_deref(),
            Some("fork")
        );
        assert_eq!(
            remote_for(&remotes(), &RepoId::new("gitlab.com", "x", "y")),
            None
        );
    }

    #[test]
    fn the_plan_puts_the_worktree_next_to_the_checkout() {
        let id = RepoId::new("github.com", "sorinirimies", "gitkraft");
        let p = plan(
            PlatformKind::GitHub,
            &id,
            42,
            &remotes(),
            Path::new("/home/me/Projects/gitkraft"),
        )
        .expect("plan");
        assert_eq!(
            p,
            Plan {
                remote: "origin".into(),
                remote_ref: "refs/pull/42/head".into(),
                branch: "gitkraft/pr-42".into(),
                dest: PathBuf::from("/home/me/Projects/gitkraft-pr-42")
            }
        );
        assert!(
            plan(
                PlatformKind::GitHub,
                &RepoId::new("a.com", "b", "c"),
                1,
                &remotes(),
                Path::new("/x/y")
            )
            .is_none()
        );
        assert!(plan(PlatformKind::GitHub, &id, 1, &remotes(), Path::new("/")).is_none());
    }
}
