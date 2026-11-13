//! Statistics over a list of finished runs, for the CI insight charts.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{Run, Status};

/// Numbers for one workflow (or for everything).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Summary {
    /// Finished runs counted (cancelled and skipped are left out).
    pub finished: usize,
    /// Of those, green.
    pub succeeded: usize,
    /// `succeeded / finished`, 0 when nothing finished.
    pub success_rate: f64,
    /// Median duration in seconds.
    pub median_secs: Option<f64>,
    /// 90th percentile duration in seconds.
    pub p90_secs: Option<f64>,
    /// Longest queue (created → started) in seconds.
    pub max_queue_secs: Option<f64>,
}

fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted.get(rank).copied()
}

/// Summarises `runs`.
pub fn summarize(runs: &[Run]) -> Summary {
    let done: Vec<&Run> = runs
        .iter()
        .filter(|r| matches!(r.status, Status::Success | Status::Failed))
        .collect();
    let succeeded = done.iter().filter(|r| r.status == Status::Success).count();
    let mut durations: Vec<f64> = done
        .iter()
        .filter_map(|r| r.duration())
        .map(|d| d.num_milliseconds() as f64 / 1000.0)
        .collect();
    durations.sort_by(f64::total_cmp);
    let max_queue = runs
        .iter()
        .filter_map(|r| Some((r.started_at? - r.created_at?).num_milliseconds() as f64 / 1000.0))
        .filter(|q| *q >= 0.0)
        .max_by(f64::total_cmp);
    Summary {
        finished: done.len(),
        succeeded,
        success_rate: if done.is_empty() {
            0.0
        } else {
            succeeded as f64 / done.len() as f64
        },
        median_secs: percentile(&durations, 0.5),
        p90_secs: percentile(&durations, 0.9),
        max_queue_secs: max_queue,
    }
}

/// Per-workflow summaries, by workflow name.
pub fn by_workflow(runs: &[Run]) -> BTreeMap<String, Summary> {
    let mut groups: BTreeMap<String, Vec<Run>> = BTreeMap::new();
    for r in runs {
        groups.entry(r.name.clone()).or_default().push(r.clone());
    }
    groups
        .into_iter()
        .map(|(k, v)| (k, summarize(&v)))
        .collect()
}

/// Workflows that look flaky: the same commit has both a green and a red run
/// of it (a re-run flipped the result). Returns `(workflow, flaky_commits)`,
/// most flaky first.
pub fn flaky_workflows(runs: &[Run]) -> Vec<(String, usize)> {
    let mut outcomes: HashMap<(&str, &str), HashSet<Status>> = HashMap::new();
    for r in runs
        .iter()
        .filter(|r| matches!(r.status, Status::Success | Status::Failed))
    {
        outcomes
            .entry((r.name.as_str(), r.sha.as_str()))
            .or_default()
            .insert(r.status);
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for ((workflow, _), set) in &outcomes {
        if set.len() == 2 {
            *counts.entry(workflow).or_default() += 1;
        }
    }
    let mut out: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out
}

/// Duration of each finished run in seconds, oldest first: the series for a trend chart.
pub fn duration_series(runs: &[Run]) -> Vec<f64> {
    let mut with_time: Vec<_> = runs
        .iter()
        .filter(|r| r.status.is_terminal())
        .filter_map(|r| {
            Some((
                r.created_at?,
                r.duration()?.num_milliseconds() as f64 / 1000.0,
            ))
        })
        .collect();
    with_time.sort_by_key(|(t, _)| *t);
    with_time.into_iter().map(|(_, d)| d).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn run(name: &str, sha: &str, status: Status, created: i64, dur: i64) -> Run {
        let at = |s: i64| DateTime::from_timestamp(1_800_000_000 + s, 0);
        Run {
            name: name.into(),
            sha: sha.into(),
            status,
            created_at: at(created),
            started_at: at(created + 2),
            finished_at: at(created + 2 + dur),
            ..Run::default()
        }
    }

    #[test]
    fn summary_counts_only_green_and_red() {
        let runs = vec![
            run("ci", "a", Status::Success, 0, 100),
            run("ci", "b", Status::Failed, 200, 50),
            run("ci", "c", Status::Success, 400, 300),
            run("ci", "d", Status::Cancelled, 600, 5),
        ];
        let s = summarize(&runs);
        assert_eq!((s.finished, s.succeeded), (3, 2));
        assert!((s.success_rate - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(s.median_secs, Some(100.0));
        assert_eq!(s.p90_secs, Some(300.0));
        assert_eq!(s.max_queue_secs, Some(2.0));
    }

    #[test]
    fn empty_input_is_all_zero() {
        let s = summarize(&[]);
        assert_eq!((s.finished, s.success_rate, s.median_secs), (0, 0.0, None));
    }

    #[test]
    fn grouped_by_workflow() {
        let runs = vec![
            run("a", "1", Status::Success, 0, 1),
            run("b", "1", Status::Failed, 0, 1),
        ];
        let g = by_workflow(&runs);
        assert_eq!(g["a"].success_rate, 1.0);
        assert_eq!(g["b"].success_rate, 0.0);
    }

    #[test]
    fn flaky_means_both_outcomes_on_one_commit() {
        let runs = vec![
            run("ci", "x", Status::Failed, 0, 1),
            run("ci", "x", Status::Success, 10, 1),
            run("ci", "y", Status::Failed, 20, 1),
            run("lint", "x", Status::Success, 0, 1),
            run("lint", "x", Status::Success, 5, 1),
            run("slow", "p", Status::Failed, 0, 1),
            run("slow", "p", Status::Success, 5, 1),
            run("slow", "q", Status::Success, 5, 1),
            run("slow", "q", Status::Failed, 9, 1),
        ];
        assert_eq!(
            flaky_workflows(&runs),
            vec![("slow".to_string(), 2), ("ci".to_string(), 1)]
        );
    }

    #[test]
    fn series_is_oldest_first_and_skips_active() {
        let runs = vec![
            run("ci", "a", Status::Success, 100, 30),
            run("ci", "b", Status::Success, 0, 10),
            Run {
                status: Status::Running,
                ..run("ci", "c", Status::Running, 50, 1)
            },
        ];
        assert_eq!(duration_series(&runs), vec![10.0, 30.0]);
    }
}
