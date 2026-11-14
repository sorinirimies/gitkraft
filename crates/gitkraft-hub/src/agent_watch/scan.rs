//! What is red right now in the selected repositories: the latest finished run of every workflow
//! on the default branch, and open pull requests with a failing check. Read only.

use futures::{StreamExt, stream};
use gitkraft_ci::{Run, RunQuery, Status};
use gitkraft_platforms::HttpClient;
use gitkraft_platforms::model::{ChecksState, ListQuery, RepoInfo, StateFilter};
use serde::{Deserialize, Serialize};

use crate::hub::Hub;

/// What kind of red it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// The latest run of a workflow on the default branch failed.
    FailedRun,
    /// An open pull request has a failing check.
    FailingPr,
}

/// One red thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// `host/owner/name`.
    pub repo: String,
    /// What kind.
    pub kind: FindingKind,
    /// The workflow name, or the pull request title.
    pub what: String,
    /// Web address.
    pub url: String,
    /// The run, for a failed run.
    #[serde(default)]
    pub run_id: Option<u64>,
    /// The branch, for a failed run.
    #[serde(default)]
    pub branch: Option<String>,
    /// The pull request number.
    #[serde(default)]
    pub number: Option<u64>,
    /// What the agent made of it, once it has looked.
    #[serde(default)]
    pub explanation: Option<String>,
}

impl Finding {
    /// `repo workflow` or `repo #12 title`, for a list or a notification.
    #[must_use]
    pub fn headline(&self) -> String {
        match (self.kind, self.number) {
            (FindingKind::FailingPr, Some(n)) => format!("{} #{n} {}", self.repo, self.what),
            _ => format!("{} {}", self.repo, self.what),
        }
    }
}

/// The outcome of one scan.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Scan {
    /// Repositories looked at.
    pub repos: usize,
    /// Workflows whose latest run was judged.
    pub workflows: usize,
    /// Everything red.
    pub findings: Vec<Finding>,
    /// Repositories that could not be read (no CI, no access, a down server).
    pub errors: Vec<String>,
}

impl Scan {
    /// Nothing red.
    #[must_use]
    pub fn is_green(&self) -> bool {
        self.findings.is_empty()
    }
}

/// The latest finished run per workflow name; cancelled and skipped runs say nothing about health.
#[must_use]
pub fn latest_per_workflow(runs: &[Run]) -> Vec<&Run> {
    let mut sorted: Vec<&Run> = runs
        .iter()
        .filter(|r| {
            r.status.is_terminal() && !matches!(r.status, Status::Cancelled | Status::Skipped)
        })
        .collect();
    sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
    let mut seen = std::collections::HashSet::new();
    sorted.retain(|r| seen.insert(r.name.clone()));
    sorted
}

async fn scan_one<C: HttpClient + Clone>(
    hub: &Hub<C>,
    info: &RepoInfo,
) -> Result<(usize, Vec<Finding>), String> {
    let key = info.repo.key();
    let mut findings = Vec::new();
    let mut workflows = 0;
    let ci = hub
        .ci(&info.repo.host)
        .ok_or_else(|| format!("{key}: no account"))?;
    let q = RunQuery {
        branch: info.default_branch.clone(),
        per_page: 40,
        ..RunQuery::default()
    };
    let runs = ci
        .list_runs(&info.repo, &q)
        .await
        .map_err(|e| format!("{key}: {e}"))?;
    for r in latest_per_workflow(&runs) {
        workflows += 1;
        if r.status == Status::Failed {
            findings.push(Finding {
                repo: key.clone(),
                kind: FindingKind::FailedRun,
                what: r.name.clone(),
                url: r.url.clone(),
                run_id: Some(r.id),
                branch: Some(r.branch.clone()),
                number: None,
                explanation: None,
            });
        }
    }
    if let Some(b) = hub.backend(&info.repo.host) {
        let q = ListQuery {
            state: StateFilter::Open,
            ..ListQuery::default()
        };
        // A repository without pull requests enabled is not a scan failure.
        if let Ok(prs) = b.list_pull_requests(&info.repo, &q).await {
            findings.extend(
                prs.into_iter()
                    .filter(|p| p.checks == Some(ChecksState::Failure))
                    .map(|p| Finding {
                        repo: key.clone(),
                        kind: FindingKind::FailingPr,
                        what: p.title,
                        url: p.url,
                        run_id: None,
                        branch: None,
                        number: Some(p.number),
                        explanation: None,
                    }),
            );
        }
    }
    Ok((workflows, findings))
}

/// Scans `repos`, eight at a time. A repository that cannot be read is listed in `errors` and
/// the rest go on.
pub async fn scan<C: HttpClient + Clone>(hub: &Hub<C>, repos: &[&RepoInfo]) -> Scan {
    let results: Vec<_> = stream::iter(repos.iter().copied())
        .map(|info| async move { scan_one(hub, info).await })
        .buffered(8)
        .collect()
        .await;
    let mut out = Scan {
        repos: repos.len(),
        ..Scan::default()
    };
    for r in results {
        match r {
            Ok((w, f)) => {
                out.workflows += w;
                out.findings.extend(f);
            }
            Err(e) => out.errors.push(e),
        }
    }
    out
}

/// Every repository the accounts can see, plus a line per account that could not be listed.
pub async fn universe<C: HttpClient + Clone>(hub: &Hub<C>) -> (Vec<RepoInfo>, Vec<String>) {
    let mut all = Vec::new();
    let mut errors = Vec::new();
    for host in hub.hosts() {
        let Some(b) = hub.backend(host) else { continue };
        match b.list_repositories().await {
            Ok(mut v) => all.append(&mut v),
            Err(e) => errors.push(format!("{host}: {e}")),
        }
    }
    (all, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn run(id: u64, name: &str, status: Status, hour: u32) -> Run {
        Run {
            id,
            name: name.into(),
            status,
            created_at: Utc.with_ymd_and_hms(2026, 3, 1, hour, 0, 0).single(),
            ..Run::default()
        }
    }

    #[test]
    fn only_the_latest_finished_run_of_a_workflow_counts_and_cancelled_runs_are_ignored() {
        let runs = [
            run(1, "CI", Status::Success, 8),
            run(2, "CI", Status::Failed, 9),
            run(3, "CI", Status::Running, 10),
            run(4, "CI", Status::Cancelled, 11),
            run(5, "Docs", Status::Success, 9),
        ];
        let latest = latest_per_workflow(&runs);
        assert_eq!(latest.len(), 2);
        let ci = latest.iter().find(|r| r.name == "CI").expect("ci");
        assert_eq!(
            (ci.id, ci.status),
            (2, Status::Failed),
            "the cancelled run does not hide the failure"
        );
    }

    #[test]
    fn a_failing_pr_reads_with_its_number() {
        let f = Finding {
            repo: "github.com/o/r".into(),
            kind: FindingKind::FailingPr,
            what: "bump x".into(),
            url: "u".into(),
            run_id: None,
            branch: None,
            number: Some(4),
            explanation: None,
        };
        assert_eq!(f.headline(), "github.com/o/r #4 bump x");
    }
}
