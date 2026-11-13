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

#[cfg(test)]
mod tests {
    use super::*;

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
