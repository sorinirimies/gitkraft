//! Agent-watch: scheduled jobs that check the CI of the projects you pick, on any forge, and
//! hand what is red to the agent you prefer. Drafts only: an agent reads and explains, it never
//! pushes, merges or re-runs anything.
//!
//! The pieces are plain data and pure functions so every front-end shares them:
//! [`config`] (jobs on disk), [`schedule`] (when a job is due), [`select`] (which repositories),
//! [`scan`] (what is red), [`harness`] (who looks at it) and [`run`] (one job from start to report).

pub mod app;
pub mod config;
pub mod harness;
pub mod run;
pub mod scan;
pub mod schedule;
pub mod select;
pub mod state;
pub mod store;

use chrono::{DateTime, Local, NaiveDateTime};

pub use config::{AgentWatch, HarnessKind, Job, Notify};
pub use run::{Deps, Outcome, announce, execute, run_and_file, run_job};
pub use scan::{Finding, FindingKind, Scan};
pub use schedule::Schedule;
pub use select::{Selector, Visibility};
pub use state::{AgentAction, AgentsState, Flag, Pane};
pub use store::{Lease, Report, Store};

/// The local wall-clock time of `t`, what schedules are written in.
#[must_use]
pub fn local(t: DateTime<chrono::Utc>) -> NaiveDateTime {
    t.with_timezone(&Local).naive_local()
}

/// The enabled jobs that are owed a run at `now` (local time), by the newest report of each.
#[must_use]
pub fn due<'a>(cfg: &'a AgentWatch, store: &Store, now: NaiveDateTime) -> Vec<&'a Job> {
    cfg.jobs
        .iter()
        .filter(|j| j.enabled)
        .filter(|j| {
            let last = store.latest(&j.name).map(|r| local(r.started));
            j.parsed_schedule().is_ok_and(|s| s.due(now, last))
        })
        .collect()
}

/// When `job` runs next after `now` (local time), by its newest report.
#[must_use]
pub fn next_run(job: &Job, store: &Store, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let last = store.latest(&job.name).map(|r| local(r.started));
    job.parsed_schedule().ok()?.next(now, last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn a_job_is_due_until_a_report_exists_for_its_latest_slot() {
        let tmp = tempfile::tempdir().expect("tmp");
        let store = Store::new(tmp.path());
        let mut cfg = AgentWatch::default();
        cfg.add(Job::new("a", "daily 08:00")).expect("add");
        let mut off = Job::new("b", "daily 08:00");
        off.enabled = false;
        cfg.add(off).expect("add");
        let now = NaiveDateTime::parse_from_str("2026-03-03 09:00", "%Y-%m-%d %H:%M").expect("t");
        assert_eq!(due(&cfg, &store, now).len(), 1, "the disabled one never is");
        let started = Utc.from_local_datetime(&now).single().expect("utc");
        store
            .save(&Report {
                job: "a".into(),
                started,
                finished: started,
                harness: "none".into(),
                model: None,
                repos: 0,
                workflows: 0,
                findings: vec![],
                errors: vec![],
                summary: String::new(),
                failed: None,
                seen: false,
            })
            .expect("save");
        let now2 = local(started) + chrono::Duration::minutes(30);
        assert!(
            due(&cfg, &store, now2).is_empty(),
            "ran after the 08:00 slot"
        );
    }
}
