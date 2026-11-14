//! One project, several forges: a checkout whose remotes point at GitHub, Codeberg, a Gitea,
//! GitLab and so on. This module turns the remotes into a [`Project`], lets a front-end narrow its
//! lists to one of them ([`Scope`]), shows whether each remote has the local branch
//! ([`sync_status`]) and folds CI runs of the same commit on several forges into one row
//! ([`group_mirrors`]).

use std::path::Path;

use gitkraft_ci::{Run, Status};
use gitkraft_platforms::RepoId;
use gitkraft_platforms::platform::parse_remote_url;

use crate::SectionData;

/// One remote of a checkout that points at a known forge repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRemote {
    /// Git's name for it (`origin`, `codeberg`, …).
    pub name: String,
    /// The URL as configured.
    pub url: String,
    /// The repository it is.
    pub repo: RepoId,
}

/// The remotes of a checkout, `origin` first, each forge repository once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Project {
    /// Remotes in display order.
    pub remotes: Vec<ProjectRemote>,
    /// Remotes that point at the same repository as an earlier one (name, repo).
    pub aliases: Vec<(String, RepoId)>,
}

impl Project {
    /// From `(name, url)` pairs. Remotes that are not a forge URL are skipped; a second remote for
    /// the same repository (an ssh and an https name, say) is kept as an alias, not a row.
    #[must_use]
    pub fn from_remotes(pairs: &[(String, String)]) -> Self {
        let mut sorted: Vec<&(String, String)> = pairs.iter().collect();
        sorted.sort_by_key(|(n, _)| n != "origin");
        let mut project = Self::default();
        for (name, url) in sorted {
            let Some(repo) = parse_remote_url(url) else {
                continue;
            };
            if project.remotes.iter().any(|r| r.repo == repo) {
                project.aliases.push((name.clone(), repo));
            } else {
                project.remotes.push(ProjectRemote {
                    name: name.clone(),
                    url: url.clone(),
                    repo,
                });
            }
        }
        project
    }

    /// The remotes of the checkout containing `dir`; empty outside a checkout.
    #[must_use]
    pub fn of_dir(dir: &Path) -> Self {
        let Ok(repo) = git2::Repository::discover(dir) else {
            return Self::default();
        };
        Self::from_remotes(&remote_pairs(&repo))
    }

    /// The first remote's repository (`origin` when there is one).
    #[must_use]
    pub fn primary(&self) -> Option<&RepoId> {
        self.remotes.first().map(|r| &r.repo)
    }

    /// Every repository once.
    #[must_use]
    pub fn repos(&self) -> Vec<RepoId> {
        self.remotes.iter().map(|r| r.repo.clone()).collect()
    }

    /// More than one forge is involved.
    #[must_use]
    pub fn is_multi(&self) -> bool {
        self.remotes.len() > 1
    }

    /// The repositories named by `which`: `all`, a remote name, a host, or `host/owner/name`.
    ///
    /// # Errors
    /// Nothing matches (the message lists what does) or the host is ambiguous.
    pub fn select(&self, which: &str) -> Result<Vec<RepoId>, String> {
        let w = which.trim();
        if w.eq_ignore_ascii_case("all") {
            return Ok(self.repos());
        }
        let by_name: Vec<&ProjectRemote> = self.remotes.iter().filter(|r| r.name == w).collect();
        let hits = if by_name.is_empty() {
            self.remotes
                .iter()
                .filter(|r| r.repo.host == w || r.repo.key() == w)
                .collect()
        } else {
            by_name
        };
        match hits.as_slice() {
            [] => Err(format!(
                "no remote '{w}': this project has {}",
                self.remotes
                    .iter()
                    .map(|r| format!("{} ({})", r.name, r.repo.host))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            [one] => Ok(vec![one.repo.clone()]),
            many => Err(format!(
                "'{w}' matches {} remotes: name one by its remote name",
                many.len()
            )),
        }
    }

    /// The remote name that reaches `repo`, aliases included.
    #[must_use]
    pub fn name_of(&self, repo: &RepoId) -> Option<&str> {
        self.remotes
            .iter()
            .find(|r| &r.repo == repo)
            .map(|r| r.name.as_str())
    }
}

fn remote_pairs(repo: &git2::Repository) -> Vec<(String, String)> {
    let Ok(names) = repo.remotes() else {
        return Vec::new();
    };
    names
        .iter()
        .flatten()
        .flatten()
        .filter_map(|n| {
            let r = repo.find_remote(n).ok()?;
            Some((n.to_string(), r.url().ok()?.to_string()))
        })
        .collect()
}

/// What a front-end shows: everything, or one repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Scope {
    /// Every remote.
    #[default]
    All,
    /// One repository.
    Only(RepoId),
}

impl Scope {
    /// Does an item of `repo` belong to the scope?
    #[must_use]
    pub fn allows(&self, repo: &RepoId) -> bool {
        match self {
            Self::All => true,
            Self::Only(r) => r == repo,
        }
    }

    /// The next scope in `All → remote 1 → remote 2 … → All`; a project with a single remote
    /// stays on `All`.
    #[must_use]
    pub fn next(&self, project: &Project) -> Self {
        if !project.is_multi() {
            return Self::All;
        }
        match self {
            Self::All => project
                .remotes
                .first()
                .map_or(Self::All, |r| Self::Only(r.repo.clone())),
            Self::Only(cur) => {
                let at = project.remotes.iter().position(|r| &r.repo == cur);
                match at.and_then(|i| project.remotes.get(i + 1)) {
                    Some(r) => Self::Only(r.repo.clone()),
                    None => Self::All,
                }
            }
        }
    }

    /// `all remotes` or the remote's name and host.
    #[must_use]
    pub fn label(&self, project: &Project) -> String {
        match self {
            Self::All => "all remotes".into(),
            Self::Only(r) => format!("{} ({})", project.name_of(r).unwrap_or(&r.host), r.host),
        }
    }
}

impl SectionData {
    /// Drops the items of repositories outside `scope`. Notifications without a repository stay.
    pub fn retain_scope(&mut self, scope: &Scope) {
        if *scope == Scope::All {
            return;
        }
        match self {
            Self::Prs(r) => r.items.retain(|p| scope.allows(&p.repo)),
            Self::Issues(r) => r.items.retain(|i| scope.allows(&i.repo)),
            Self::Runs(r) => r.items.retain(|x| scope.allows(&x.repo)),
            Self::Inbox(r) => r
                .items
                .retain(|n| n.repo.as_ref().is_none_or(|x| scope.allows(x))),
        }
    }
}

/// Where a remote stands against the local branch, by the remote-tracking refs from the last fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Same commit.
    InSync,
    /// The remote lacks `n` local commits (push needed).
    Ahead(usize),
    /// The remote has `n` commits the branch lacks (pull needed).
    Behind(usize),
    /// Both sides have commits the other lacks.
    Diverged {
        /// Local only.
        ahead: usize,
        /// Remote only.
        behind: usize,
    },
    /// The remote has no such branch (never pushed there, or not fetched).
    Missing,
}

impl SyncState {
    /// `in sync`, `ahead 2`, `behind 1`, `diverged +2/-1` or `no branch`.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::InSync => "in sync".into(),
            Self::Ahead(n) => format!("ahead {n}"),
            Self::Behind(n) => format!("behind {n}"),
            Self::Diverged { ahead, behind } => format!("diverged +{ahead}/-{behind}"),
            Self::Missing => "no branch".into(),
        }
    }

    /// Everything is where it should be.
    #[must_use]
    pub fn is_ok(self) -> bool {
        self == Self::InSync
    }
}

/// One remote and its [`SyncState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSync {
    /// Remote name.
    pub remote: String,
    /// Repository.
    pub repo: RepoId,
    /// Standing of the current branch.
    pub state: SyncState,
}

/// Compares the checked-out branch of the repository at `dir` with the same branch on every remote
/// of the project. Local data only: it shows what the last fetch saw. Empty on a detached HEAD.
#[must_use]
pub fn sync_status(dir: &Path, project: &Project) -> Vec<RemoteSync> {
    let Ok(repo) = git2::Repository::discover(dir) else {
        return Vec::new();
    };
    let Ok(head) = repo.head() else {
        return Vec::new();
    };
    let (Some(branch), Some(local)) = (head.shorthand().ok().map(str::to_string), head.target())
    else {
        return Vec::new();
    };
    if !head.is_branch() {
        return Vec::new();
    }
    project
        .remotes
        .iter()
        .map(|r| {
            let tip = repo
                .refname_to_id(&format!("refs/remotes/{}/{branch}", r.name))
                .ok();
            let state = match tip {
                None => SyncState::Missing,
                Some(t) => match repo.graph_ahead_behind(local, t) {
                    Ok((0, 0)) => SyncState::InSync,
                    Ok((a, 0)) => SyncState::Ahead(a),
                    Ok((0, b)) => SyncState::Behind(b),
                    Ok((ahead, behind)) => SyncState::Diverged { ahead, behind },
                    Err(_) => SyncState::Missing,
                },
            };
            RemoteSync {
                remote: r.name.clone(),
                repo: r.repo.clone(),
                state,
            }
        })
        .collect()
}

/// `github`, `codeberg`, `192.168.1.44`: a host without `www.`, `.com` or `.org`, for badges.
#[must_use]
pub fn short_host(host: &str) -> String {
    host.trim_start_matches("www.")
        .trim_end_matches(".com")
        .trim_end_matches(".org")
        .to_string()
}

/// The runs in `runs` that built the same commit and workflow as `run` on another repository.
#[must_use]
pub fn mirror_siblings<'a>(runs: &'a [Run], run: &Run) -> Vec<&'a Run> {
    if run.sha.is_empty() {
        return Vec::new();
    }
    runs.iter()
        .filter(|o| o.sha == run.sha && o.name == run.name && o.repo != run.repo)
        .collect()
}

/// The runs of one commit and workflow across forges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirrorGroup {
    /// Commit.
    pub sha: String,
    /// Workflow or pipeline name.
    pub name: String,
    /// One run per repository, in the order first seen.
    pub runs: Vec<Run>,
}

impl MirrorGroup {
    /// The worst status among the runs: a failure anywhere shows, then unfinished work, then
    /// cancellations, then success.
    #[must_use]
    pub fn status(&self) -> Status {
        let rank = |s: Status| match s {
            Status::Failed => 5,
            Status::Running => 4,
            Status::Waiting | Status::Queued | Status::Manual => 3,
            Status::Cancelled => 2,
            Status::Skipped => 1,
            Status::Success => 0,
        };
        self.runs
            .iter()
            .map(|r| r.status)
            .max_by_key(|s| rank(*s))
            .unwrap_or_default()
    }

    /// Every forge's run ended the same way.
    #[must_use]
    pub fn agree(&self) -> bool {
        self.runs.windows(2).all(|w| w[0].status == w[1].status)
    }

    /// Hosts of the runs, in order.
    #[must_use]
    pub fn hosts(&self) -> Vec<&str> {
        self.runs.iter().map(|r| r.repo.host.as_str()).collect()
    }
}

/// Folds runs with the same commit and workflow name on different repositories into one group;
/// runs of the same repository stay separate (a re-run is its own run). Order of first sight.
#[must_use]
pub fn group_mirrors(runs: &[Run]) -> Vec<MirrorGroup> {
    let mut groups: Vec<MirrorGroup> = Vec::new();
    for run in runs {
        let slot = (!run.sha.is_empty())
            .then(|| {
                groups.iter_mut().find(|g| {
                    g.sha == run.sha
                        && g.name == run.name
                        && g.runs.iter().all(|r| r.repo != run.repo)
                })
            })
            .flatten();
        match slot {
            Some(g) => g.runs.push(run.clone()),
            None => groups.push(MirrorGroup {
                sha: run.sha.clone(),
                name: run.name.clone(),
                runs: vec![run.clone()],
            }),
        }
    }
    groups
}

/// One remote as a front-end lists it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RemoteRow {
    /// Remote name.
    pub remote: String,
    /// `host/owner/name`.
    pub repo: String,
    /// Host.
    pub host: String,
    /// An account for the host is connected.
    pub account: bool,
    /// The standing of the checked-out branch there (`in sync`, `ahead 2`, …); `None` when unknown.
    pub branch: Option<String>,
    /// The URL as configured.
    pub url: String,
}

/// The remotes of `project` with account and branch state.
#[must_use]
pub fn remote_rows<C: gitkraft_platforms::HttpClient + Clone>(
    hub: &crate::Hub<C>,
    project: &Project,
    sync: &[RemoteSync],
) -> Vec<RemoteRow> {
    project
        .remotes
        .iter()
        .map(|r| RemoteRow {
            remote: r.name.clone(),
            repo: r.repo.key(),
            host: r.repo.host.clone(),
            account: hub.backend(&r.repo.host).is_some(),
            branch: sync
                .iter()
                .find(|s| s.remote == r.name)
                .map(|s| s.state.label()),
            url: r.url.clone(),
        })
        .collect()
}

/// The runs of every remote of `project`, newest first, folded by commit and workflow. A forge that
/// cannot be asked (no account, an error) is named in the second part instead of failing the rest.
pub async fn mirror_runs<C: gitkraft_platforms::HttpClient + Clone>(
    hub: &crate::Hub<C>,
    project: &Project,
    query: &gitkraft_ci::RunQuery,
) -> (Vec<MirrorGroup>, Vec<String>) {
    let mut runs = Vec::new();
    let mut failed = Vec::new();
    for repo in project.repos() {
        let Some(ci) = hub.ci(&repo.host) else {
            failed.push(format!("{}: no account", repo.key()));
            continue;
        };
        match ci.list_runs(&repo, query).await {
            Ok(r) => runs.extend(r),
            Err(e) => failed.push(format!("{}: {e}", repo.key())),
        }
    }
    runs.sort_by_key(|r| std::cmp::Reverse(r.created_at));
    (group_mirrors(&runs), failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect()
    }

    fn project() -> Project {
        Project::from_remotes(&pairs(&[
            ("codeberg", "ssh://git@codeberg.org/me/p.git"),
            ("gitea", "gitea@192.168.1.44:me/p.git"),
            ("origin", "git@github.com:me/p.git"),
            ("gh-https", "https://github.com/me/p"),
            ("local", "/srv/git/p.git"),
        ]))
    }

    #[test]
    fn remotes_come_origin_first_once_per_repository_and_skip_non_forge_urls() {
        let p = project();
        let names: Vec<_> = p.remotes.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["origin", "codeberg", "gitea"]);
        assert_eq!(p.aliases.len(), 1);
        assert_eq!(p.aliases[0].0, "gh-https");
        assert_eq!(p.primary().map(RepoId::key), Some("github.com/me/p".into()));
        assert!(p.is_multi());
        assert_eq!(p.repos().len(), 3);
        assert!(!Project::default().is_multi());
    }

    #[test]
    fn a_remote_is_picked_by_name_host_or_key_and_a_miss_lists_the_choices() {
        let p = project();
        assert_eq!(p.select("all").map(|v| v.len()), Ok(3));
        assert_eq!(
            p.select("codeberg").map(|v| v[0].host.clone()),
            Ok("codeberg.org".into())
        );
        assert_eq!(
            p.select("github.com").map(|v| v[0].key()),
            Ok("github.com/me/p".into())
        );
        assert_eq!(p.select("github.com/me/p").map(|v| v.len()), Ok(1));
        let e = p.select("gitlab").unwrap_err();
        assert!(
            e.contains("origin (github.com)") && e.contains("gitea (192.168.1.44)"),
            "{e}"
        );
        let twice = Project::from_remotes(&pairs(&[
            ("a", "git@host.x:one/p.git"),
            ("b", "git@host.x:two/p.git"),
        ]));
        assert!(twice.select("host.x").unwrap_err().contains("matches 2"));
        assert_eq!(
            twice.select("b").map(|v| v[0].owner.clone()),
            Ok("two".into())
        );
    }

    #[test]
    fn the_scope_cycles_through_the_remotes_and_filters_items() {
        let p = project();
        let mut s = Scope::All;
        let mut seen = vec![s.label(&p)];
        for _ in 0..3 {
            s = s.next(&p);
            seen.push(s.label(&p));
        }
        assert_eq!(
            seen,
            [
                "all remotes",
                "origin (github.com)",
                "codeberg (codeberg.org)",
                "gitea (192.168.1.44)"
            ]
        );
        assert_eq!(s.next(&p), Scope::All);
        assert_eq!(
            Scope::All.next(&Project::default()),
            Scope::All,
            "one forge: nothing to cycle"
        );
        let gh = RepoId::new("github.com", "me", "p");
        let cb = RepoId::new("codeberg.org", "me", "p");
        assert!(Scope::Only(gh.clone()).allows(&gh) && !Scope::Only(gh).allows(&cb));
    }

    fn run(id: u64, host: &str, sha: &str, name: &str, status: Status) -> Run {
        Run {
            id,
            repo: RepoId::new(host, "me", "p"),
            name: name.into(),
            sha: sha.into(),
            status,
            ..Run::default()
        }
    }

    #[test]
    fn runs_of_one_commit_on_several_forges_fold_into_one_group_with_the_worst_status() {
        let runs = vec![
            run(1, "github.com", "abc", "CI", Status::Success),
            run(2, "codeberg.org", "abc", "CI", Status::Failed),
            run(3, "github.com", "abc", "CI", Status::Success), // a re-run on the same forge
            run(4, "github.com", "def", "CI", Status::Running),
            run(5, "github.com", "abc", "Lint", Status::Success),
            run(6, "github.com", "", "CI", Status::Success),
            run(7, "codeberg.org", "", "CI", Status::Success),
        ];
        let g = group_mirrors(&runs);
        let shape: Vec<_> = g
            .iter()
            .map(|x| (x.sha.as_str(), x.name.as_str(), x.runs.len()))
            .collect();
        assert_eq!(
            shape,
            [
                ("abc", "CI", 2),
                ("abc", "CI", 1),
                ("def", "CI", 1),
                ("abc", "Lint", 1),
                ("", "CI", 1),
                ("", "CI", 1)
            ]
        );
        assert_eq!(g[0].status(), Status::Failed);
        assert!(!g[0].agree());
        assert_eq!(g[0].hosts(), ["github.com", "codeberg.org"]);
        assert_eq!(g[2].status(), Status::Running);
        assert!(g[2].agree());
    }

    #[test]
    fn badges_and_siblings_help_rows_say_where_a_run_came_from() {
        assert_eq!(short_host("github.com"), "github");
        assert_eq!(short_host("www.codeberg.org"), "codeberg");
        assert_eq!(short_host("192.168.1.44"), "192.168.1.44");
        let runs = vec![
            run(1, "github.com", "abc", "CI", Status::Success),
            run(2, "codeberg.org", "abc", "CI", Status::Failed),
            run(3, "github.com", "abc", "Lint", Status::Success),
            run(4, "github.com", "", "CI", Status::Success),
        ];
        let sib = mirror_siblings(&runs, &runs[0]);
        assert_eq!(sib.len(), 1);
        assert_eq!(sib[0].id, 2);
        assert!(
            mirror_siblings(&runs, &runs[3]).is_empty(),
            "no sha, no match"
        );
    }

    #[test]
    fn a_scope_drops_other_forges_from_every_kind_of_list() {
        let mut d = SectionData::Runs(crate::SectionResult {
            items: vec![
                run(1, "github.com", "a", "CI", Status::Success),
                run(2, "codeberg.org", "a", "CI", Status::Success),
            ],
            errors: vec![],
        });
        d.retain_scope(&Scope::All);
        assert_eq!(d.len(), 2);
        d.retain_scope(&Scope::Only(RepoId::new("codeberg.org", "me", "p")));
        assert_eq!(d.len(), 1);
    }

    fn commit(repo: &git2::Repository, msg: &str, parent: Option<git2::Oid>) -> git2::Oid {
        let sig = git2::Signature::now("t", "t@t").expect("sig");
        let tree = repo
            .find_tree(repo.treebuilder(None).expect("tb").write().expect("tree"))
            .expect("find");
        let parents: Vec<git2::Commit> = parent
            .map(|p| repo.find_commit(p).expect("parent"))
            .into_iter()
            .collect();
        let refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &refs)
            .expect("commit")
    }

    #[test]
    fn each_remote_is_compared_with_the_local_branch() {
        let dir = tempfile::tempdir().expect("tmp");
        let repo = git2::Repository::init(dir.path()).expect("init");
        repo.remote("origin", "git@github.com:me/p.git").expect("r");
        repo.remote("codeberg", "git@codeberg.org:me/p.git")
            .expect("r");
        repo.remote("gitea", "git@gitea.lan:me/p.git").expect("r");
        repo.remote("lab", "git@gitlab.com:me/p.git").expect("r");
        let one = commit(&repo, "one", None);
        let two = commit(&repo, "two", Some(one));
        let side = {
            // a commit on another branch that origin has and we lack, on top of `one`
            let sig = git2::Signature::now("t", "t@t").expect("sig");
            let tree = repo
                .find_tree(repo.treebuilder(None).expect("tb").write().expect("t"))
                .expect("f");
            let p = repo.find_commit(one).expect("p");
            repo.commit(None, &sig, &sig, "side", &tree, &[&p])
                .expect("c")
        };
        let branch = repo
            .head()
            .expect("head")
            .shorthand()
            .expect("name")
            .to_string();
        repo.reference(&format!("refs/remotes/origin/{branch}"), two, true, "")
            .expect("ref");
        repo.reference(&format!("refs/remotes/codeberg/{branch}"), one, true, "")
            .expect("ref");
        repo.reference(&format!("refs/remotes/gitea/{branch}"), side, true, "")
            .expect("ref");
        let project = Project::of_dir(dir.path());
        let s = sync_status(dir.path(), &project);
        let by: Vec<(String, String)> = s
            .iter()
            .map(|x| (x.remote.clone(), x.state.label()))
            .collect();
        assert_eq!(
            by,
            [
                ("origin".into(), "in sync".into()),
                ("codeberg".into(), "ahead 1".into()),
                ("gitea".into(), "diverged +1/-1".into()),
                ("lab".into(), "no branch".into()),
            ]
        );
        assert!(s[0].state.is_ok() && !s[1].state.is_ok());
        assert_eq!(SyncState::Behind(3).label(), "behind 3");
        assert!(sync_status(tempfile::tempdir().expect("t").path(), &project).is_empty());
    }
}
