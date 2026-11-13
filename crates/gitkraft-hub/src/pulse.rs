//! A repo's pulse: what happened in the last N days.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use gitkraft_ci::{Run, Status};
use gitkraft_platforms::model::{Issue, ItemState, PullRequest};

/// Activity summary for a period.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pulse {
    /// Days covered.
    pub days: u32,
    /// PRs opened in the period.
    pub prs_opened: usize,
    /// PRs merged in the period.
    pub prs_merged: usize,
    /// PRs closed without merging.
    pub prs_closed: usize,
    /// PRs still open.
    pub prs_open_now: usize,
    /// Issues opened in the period.
    pub issues_opened: usize,
    /// Issues closed in the period.
    pub issues_closed: usize,
    /// CI runs finished green / red in the period.
    pub ci_green: usize,
    /// Red CI runs.
    pub ci_red: usize,
    /// PR authors by number of PRs opened, most active first.
    pub top_authors: Vec<(String, usize)>,
    /// PRs opened per day, oldest first, one entry per day of the period (for a bar chart).
    pub prs_per_day: Vec<f64>,
}

/// Builds the pulse. A PR counts as merged / closed in the period by its `updated_at`,
/// which is when it last changed (the platforms' lists do not carry a separate close time).
pub fn pulse(
    prs: &[PullRequest],
    issues: &[Issue],
    runs: &[Run],
    days: u32,
    now: DateTime<Utc>,
) -> Pulse {
    let since = now - Duration::days(i64::from(days));
    let within =
        |t: Option<DateTime<Utc>>| t.is_some_and(|t| t >= since && t <= now + Duration::minutes(1));
    let mut authors: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_day = vec![0.0; days as usize];
    let mut p = Pulse {
        days,
        ..Pulse::default()
    };
    for pr in prs {
        if within(pr.created_at) {
            p.prs_opened += 1;
            *authors.entry(pr.author.login.as_str()).or_default() += 1;
            if days > 0
                && let Some(t) = pr.created_at
            {
                let idx = (days as i64 - 1) - (now - t).num_days().clamp(0, i64::from(days) - 1);
                if let Some(slot) = per_day.get_mut(idx as usize) {
                    *slot += 1.0;
                }
            }
        }
        match pr.state {
            ItemState::Open => p.prs_open_now += 1,
            ItemState::Merged if within(pr.updated_at) => p.prs_merged += 1,
            ItemState::Closed if within(pr.updated_at) => p.prs_closed += 1,
            _ => {}
        }
    }
    for i in issues {
        if within(i.created_at) {
            p.issues_opened += 1;
        }
        if i.state == ItemState::Closed && within(i.updated_at) {
            p.issues_closed += 1;
        }
    }
    for r in runs.iter().filter(|r| within(r.finished_at)) {
        match r.status {
            Status::Success => p.ci_green += 1,
            Status::Failed => p.ci_red += 1,
            _ => {}
        }
    }
    let mut top: Vec<(String, usize)> = authors
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top.truncate(5);
    p.top_authors = top;
    p.prs_per_day = per_day;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::model::User;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("ts")
    }

    fn pr(
        author: &str,
        state: ItemState,
        created_days_ago: i64,
        updated_days_ago: i64,
    ) -> PullRequest {
        PullRequest {
            author: User {
                login: author.into(),
                avatar_url: None,
            },
            state,
            created_at: Some(now() - Duration::days(created_days_ago)),
            updated_at: Some(now() - Duration::days(updated_days_ago)),
            ..PullRequest::default()
        }
    }

    #[test]
    fn counts_only_what_falls_inside_the_period() {
        let prs = vec![
            pr("a", ItemState::Merged, 2, 1),
            pr("a", ItemState::Open, 3, 0),
            pr("b", ItemState::Closed, 5, 4),
            pr("c", ItemState::Merged, 40, 39),
        ];
        let issues = vec![
            Issue {
                state: ItemState::Open,
                created_at: Some(now() - Duration::days(1)),
                ..Issue::default()
            },
            Issue {
                state: ItemState::Closed,
                created_at: Some(now() - Duration::days(90)),
                updated_at: Some(now() - Duration::days(2)),
                ..Issue::default()
            },
        ];
        let runs = vec![
            Run {
                status: Status::Success,
                finished_at: Some(now() - Duration::days(1)),
                ..Run::default()
            },
            Run {
                status: Status::Failed,
                finished_at: Some(now() - Duration::days(2)),
                ..Run::default()
            },
            Run {
                status: Status::Failed,
                finished_at: Some(now() - Duration::days(60)),
                ..Run::default()
            },
            Run {
                status: Status::Cancelled,
                finished_at: Some(now()),
                ..Run::default()
            },
        ];
        let p = pulse(&prs, &issues, &runs, 7, now());
        assert_eq!(
            (p.prs_opened, p.prs_merged, p.prs_closed, p.prs_open_now),
            (3, 1, 1, 1)
        );
        assert_eq!((p.issues_opened, p.issues_closed), (1, 1));
        assert_eq!((p.ci_green, p.ci_red), (1, 1));
        assert_eq!(
            p.top_authors,
            vec![("a".to_string(), 2), ("b".to_string(), 1)]
        );
    }

    #[test]
    fn per_day_series_has_one_slot_per_day_newest_last() {
        let prs = vec![
            pr("a", ItemState::Open, 0, 0),
            pr("a", ItemState::Open, 0, 0),
            pr("a", ItemState::Open, 6, 0),
        ];
        let p = pulse(&prs, &[], &[], 7, now());
        assert_eq!(p.prs_per_day.len(), 7);
        assert_eq!(p.prs_per_day[6], 2.0);
        assert_eq!(p.prs_per_day[0], 1.0);
        assert_eq!(p.prs_per_day.iter().sum::<f64>(), 3.0);
    }

    #[test]
    fn empty_input_and_zero_days_do_not_panic() {
        assert_eq!(pulse(&[], &[], &[], 7, now()).prs_opened, 0);
        let p = pulse(&[pr("a", ItemState::Open, 0, 0)], &[], &[], 0, now());
        assert!(p.prs_per_day.is_empty());
    }
}
