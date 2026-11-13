//! Change detection between two polls of the same repo, and the alerts for it.

use std::collections::BTreeMap;

use gitkraft_platforms::RepoId;
use gitkraft_platforms::notify::Alert;
use gitkraft_settings::notify::NotifyEvent;
use serde::{Deserialize, Serialize};

use crate::model::{Run, Status};

/// What a run or job did since the last poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CiEvent {
    /// A run finished red.
    RunFailed(Run),
    /// A run finished green where the previous run of that workflow and branch was red.
    RunRecovered(Run),
    /// A run you may care about finished (green, any previous state).
    RunSucceeded(Run),
}

/// The last status seen per `(workflow, branch)` and which run ids were already reported.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CiMemory {
    last: BTreeMap<String, Status>,
    reported: Vec<u64>,
}

/// How many reported run ids to remember.
const REPORTED_CAP: usize = 200;

fn key(r: &Run) -> String {
    format!("{}\u{0}{}", r.name, r.branch)
}

/// Compares `runs` (newest first, as platforms return them) with `memory`.
/// The first call for a repo (`memory` empty) only seeds it.
pub fn detect(memory: &mut CiMemory, runs: &[Run]) -> Vec<CiEvent> {
    let first = memory.last.is_empty() && memory.reported.is_empty();
    let mut events = Vec::new();
    // Oldest first, so a recovery is seen after the failure it follows.
    for r in runs.iter().rev().filter(|r| r.status.is_terminal()) {
        if memory.reported.contains(&r.id) {
            continue;
        }
        memory.reported.push(r.id);
        let k = key(r);
        let before = memory.last.get(&k).copied();
        if matches!(r.status, Status::Success | Status::Failed) {
            memory.last.insert(k, r.status);
        }
        if first {
            continue;
        }
        match (r.status, before) {
            (Status::Failed, _) => events.push(CiEvent::RunFailed(r.clone())),
            (Status::Success, Some(Status::Failed)) => {
                events.push(CiEvent::RunRecovered(r.clone()))
            }
            (Status::Success, _) => events.push(CiEvent::RunSucceeded(r.clone())),
            _ => {}
        }
    }
    if memory.reported.len() > REPORTED_CAP {
        let drop = memory.reported.len() - REPORTED_CAP;
        memory.reported.drain(..drop);
    }
    events
}

/// Turns CI events into alerts. Plain green runs are not alerts; only failures and recoveries are.
pub fn alerts(repo: &RepoId, events: &[CiEvent], me: &str) -> Vec<Alert> {
    events
        .iter()
        .filter_map(|e| {
            let (event, run) = match e {
                CiEvent::RunFailed(r) => (NotifyEvent::CiFailed, r),
                CiEvent::RunRecovered(r) => (NotifyEvent::CiRecovered, r),
                CiEvent::RunSucceeded(_) => return None,
            };
            let mine = !me.is_empty() && run.actor.eq_ignore_ascii_case(me);
            Some(Alert {
                repo: repo.clone(),
                event,
                number: Some(run.id),
                title: format!("{} · {}", run.name, run.title),
                url: run.url.clone(),
                branch: Some(run.branch.clone()),
                labels: Vec::new(),
                author: Some(run.actor.clone()),
                mine,
                involved: mine,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: u64, status: Status, branch: &str) -> Run {
        Run {
            id,
            name: "ci".into(),
            status,
            branch: branch.into(),
            actor: "Alice".into(),
            ..Run::default()
        }
    }

    #[test]
    fn first_call_seeds_only() {
        let mut m = CiMemory::default();
        assert!(
            detect(
                &mut m,
                &[
                    run(2, Status::Failed, "master"),
                    run(1, Status::Success, "master")
                ]
            )
            .is_empty()
        );
        assert!(
            detect(&mut m, &[run(2, Status::Failed, "master")]).is_empty(),
            "already reported"
        );
    }

    #[test]
    fn failure_then_recovery() {
        let mut m = CiMemory::default();
        detect(&mut m, &[run(1, Status::Success, "master")]);
        let ev = detect(
            &mut m,
            &[
                run(2, Status::Failed, "master"),
                run(1, Status::Success, "master"),
            ],
        );
        assert!(matches!(ev.as_slice(), [CiEvent::RunFailed(r)] if r.id == 2));
        let ev = detect(
            &mut m,
            &[
                run(3, Status::Success, "master"),
                run(2, Status::Failed, "master"),
            ],
        );
        assert!(matches!(ev.as_slice(), [CiEvent::RunRecovered(r)] if r.id == 3));
        let ev = detect(&mut m, &[run(4, Status::Success, "master")]);
        assert!(matches!(ev.as_slice(), [CiEvent::RunSucceeded(_)]));
    }

    #[test]
    fn branches_are_tracked_separately_and_active_runs_wait() {
        let mut m = CiMemory::default();
        detect(&mut m, &[run(1, Status::Failed, "dev")]);
        let ev = detect(
            &mut m,
            &[
                run(3, Status::Success, "master"),
                run(2, Status::Running, "dev"),
            ],
        );
        assert!(
            matches!(ev.as_slice(), [CiEvent::RunSucceeded(r)] if r.branch == "master"),
            "no recovery across branches"
        );
        let ev = detect(&mut m, &[run(2, Status::Success, "dev")]);
        assert!(
            matches!(ev.as_slice(), [CiEvent::RunRecovered(_)]),
            "the running run is reported once it finishes"
        );
    }

    #[test]
    fn cancelled_runs_do_not_change_the_last_state() {
        let mut m = CiMemory::default();
        detect(&mut m, &[run(1, Status::Failed, "master")]);
        assert!(detect(&mut m, &[run(2, Status::Cancelled, "master")]).is_empty());
        let ev = detect(&mut m, &[run(3, Status::Success, "master")]);
        assert!(matches!(ev.as_slice(), [CiEvent::RunRecovered(_)]));
    }

    #[test]
    fn memory_is_capped() {
        let mut m = CiMemory::default();
        detect(&mut m, &[run(0, Status::Success, "m")]);
        let runs: Vec<Run> = (1..=300)
            .rev()
            .map(|i| run(i, Status::Success, "m"))
            .collect();
        detect(&mut m, &runs);
        assert_eq!(m.reported.len(), REPORTED_CAP);
    }

    #[test]
    fn only_failures_and_recoveries_become_alerts() {
        let repo = RepoId::new("h", "o", "r");
        let events = vec![
            CiEvent::RunFailed(run(1, Status::Failed, "master")),
            CiEvent::RunRecovered(run(2, Status::Success, "master")),
            CiEvent::RunSucceeded(run(3, Status::Success, "master")),
        ];
        let a = alerts(&repo, &events, "alice");
        assert_eq!(
            a.iter().map(|a| a.event).collect::<Vec<_>>(),
            vec![NotifyEvent::CiFailed, NotifyEvent::CiRecovered]
        );
        assert!(a[0].mine, "actor matches me, case-insensitively");
        assert_eq!(a[0].branch.as_deref(), Some("master"));
        assert!(!alerts(&repo, &events, "")[0].mine);
    }
}
