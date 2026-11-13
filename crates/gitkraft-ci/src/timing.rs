//! Timings for the Gantt view and the critical path through a run's jobs.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::model::{Job, RunDetail};

/// One bar of the Gantt chart, in seconds from the run's start.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// Label (`job` or `job › step`).
    pub label: String,
    /// Offset from the run start.
    pub start: f64,
    /// Length.
    pub length: f64,
    /// Nesting: 0 for jobs, 1 for steps.
    pub depth: u8,
    /// On the critical path (jobs only).
    pub critical: bool,
}

fn secs(a: DateTime<Utc>, b: DateTime<Utc>) -> f64 {
    ((b - a).num_milliseconds() as f64 / 1000.0).max(0.0)
}

/// Job duration in seconds, when it has started and finished.
pub fn job_seconds(job: &Job) -> Option<f64> {
    Some(secs(job.started_at?, job.finished_at?))
}

/// Names of the jobs on the longest chain through the `needs` graph, in order.
/// Jobs without `needs` start a chain. Cycles and unknown names are ignored.
pub fn critical_path(jobs: &[Job]) -> Vec<String> {
    let by_name: HashMap<&str, &Job> = jobs.iter().map(|j| (j.name.as_str(), j)).collect();
    // best[name] = (total seconds of the longest chain ending at name, previous job)
    let mut best: HashMap<&str, (f64, Option<&str>)> = HashMap::new();
    fn visit<'a>(
        name: &'a str,
        by_name: &HashMap<&'a str, &'a Job>,
        best: &mut HashMap<&'a str, (f64, Option<&'a str>)>,
        stack: &mut Vec<&'a str>,
    ) -> f64 {
        if let Some((t, _)) = best.get(name) {
            return *t;
        }
        if stack.contains(&name) {
            return 0.0;
        }
        let Some(job) = by_name.get(name) else {
            return 0.0;
        };
        stack.push(name);
        let own = job_seconds(job).unwrap_or(0.0);
        let mut top = (0.0, None);
        for dep in &job.needs {
            if let Some((dep_name, _)) = by_name.get_key_value(dep.as_str()) {
                let t = visit(dep_name, by_name, best, stack);
                if t > top.0 {
                    top = (t, Some(*dep_name));
                }
            }
        }
        stack.pop();
        best.insert(name, (top.0 + own, top.1));
        top.0 + own
    }
    let mut stack = Vec::new();
    let mut end: Option<(&str, f64)> = None;
    for j in jobs {
        let t = visit(j.name.as_str(), &by_name, &mut best, &mut stack);
        if end.is_none_or(|(_, e)| t > e) {
            end = Some((j.name.as_str(), t));
        }
    }
    let mut path = Vec::new();
    let mut cur = end.map(|(n, _)| n);
    while let Some(n) = cur {
        path.push(n.to_string());
        cur = best.get(n).and_then(|(_, p)| *p);
    }
    path.reverse();
    path
}

/// Fills in `needs` when the platform does not report them (GitHub, Gitea): a job
/// is taken to wait for the job that finished last before it started. Jobs
/// that already have `needs` are left alone, and so are jobs that never started.
pub fn infer_needs(jobs: &mut [Job]) {
    if jobs.iter().any(|j| !j.needs.is_empty()) {
        return;
    }
    let finished: Vec<(String, DateTime<Utc>)> = jobs
        .iter()
        .filter_map(|j| Some((j.name.clone(), j.finished_at?)))
        .collect();
    for j in jobs.iter_mut() {
        let Some(start) = j.started_at else { continue };
        let latest = finished
            .iter()
            .filter(|(n, end)| *end <= start && *n != j.name)
            .max_by_key(|(_, end)| *end);
        if let Some((name, _)) = latest {
            j.needs = vec![name.clone()];
        }
    }
}

/// Gantt rows for a run: each started job, followed by its started steps.
pub fn gantt(detail: &RunDetail) -> Vec<Bar> {
    let origin = detail
        .run
        .started_at
        .or(detail.run.created_at)
        .or_else(|| detail.jobs.iter().filter_map(|j| j.started_at).min());
    let Some(origin) = origin else {
        return Vec::new();
    };
    let critical: Vec<String> = critical_path(&detail.jobs);
    let mut bars = Vec::new();
    for job in &detail.jobs {
        let Some(start) = job.started_at else {
            continue;
        };
        let end = job.finished_at.unwrap_or_else(Utc::now);
        bars.push(Bar {
            label: job.name.clone(),
            start: secs(origin, start),
            length: secs(start, end),
            depth: 0,
            critical: critical.contains(&job.name),
        });
        for step in &job.steps {
            let Some(s) = step.started_at else { continue };
            let e = step.finished_at.unwrap_or(end);
            bars.push(Bar {
                label: format!("{} › {}", job.name, step.name),
                start: secs(origin, s),
                length: secs(s, e),
                depth: 1,
                critical: false,
            });
        }
    }
    bars
}

/// The slowest steps across a run, longest first: `(job, step, seconds)`.
pub fn slowest_steps(detail: &RunDetail, n: usize) -> Vec<(String, String, f64)> {
    let mut all: Vec<_> = detail
        .jobs
        .iter()
        .flat_map(|j| {
            j.steps.iter().filter_map(move |s| {
                Some((
                    j.name.clone(),
                    s.name.clone(),
                    secs(s.started_at?, s.finished_at?),
                ))
            })
        })
        .collect();
    all.sort_by(|a, b| b.2.total_cmp(&a.2));
    all.truncate(n);
    all
}

/// `75` → `1m 15s`, `3700` → `1h 01m`.
pub fn format_seconds(total: f64) -> String {
    let t = total.max(0.0).round() as u64;
    match (t / 3600, (t % 3600) / 60, t % 60) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m {s:02}s"),
        (h, m, _) => format!("{h}h {m:02}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Run, Step};

    fn t(s: i64) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(1_800_000_000 + s, 0)
    }

    fn job(name: &str, start: i64, end: i64, needs: &[&str]) -> Job {
        Job {
            name: name.into(),
            started_at: t(start),
            finished_at: t(end),
            needs: needs.iter().map(|s| s.to_string()).collect(),
            ..Job::default()
        }
    }

    #[test]
    fn critical_path_follows_the_longest_chain() {
        // build(10) → test(30) → deploy(5) = 45; lint(100) alone = 100.
        let jobs = vec![
            job("build", 0, 10, &[]),
            job("test", 10, 40, &["build"]),
            job("deploy", 40, 45, &["test"]),
            job("lint", 0, 20, &[]),
        ];
        assert_eq!(critical_path(&jobs), ["build", "test", "deploy"]);
        let heavy = vec![job("a", 0, 5, &[]), job("b", 0, 100, &[])];
        assert_eq!(critical_path(&heavy), ["b"]);
    }

    #[test]
    fn critical_path_survives_cycles_unknown_needs_and_empty() {
        let jobs = vec![job("a", 0, 5, &["b"]), job("b", 0, 5, &["a", "ghost"])];
        let path = critical_path(&jobs);
        assert!(!path.is_empty() && path.len() <= 2);
        assert!(critical_path(&[]).is_empty());
    }

    fn detail() -> RunDetail {
        let mut build = job("build", 5, 25, &[]);
        build.steps = vec![
            Step {
                number: 1,
                name: "checkout".into(),
                started_at: t(5),
                finished_at: t(7),
                ..Step::default()
            },
            Step {
                number: 2,
                name: "compile".into(),
                started_at: t(7),
                finished_at: t(25),
                ..Step::default()
            },
            Step {
                number: 3,
                name: "never ran".into(),
                ..Step::default()
            },
        ];
        RunDetail {
            run: Run {
                started_at: t(0),
                ..Run::default()
            },
            jobs: vec![
                build,
                Job {
                    name: "queued".into(),
                    ..Job::default()
                },
            ],
        }
    }

    #[test]
    fn gantt_offsets_are_relative_to_run_start_and_skip_unstarted() {
        let bars = gantt(&detail());
        assert_eq!(bars.len(), 3);
        assert_eq!(
            (bars[0].start, bars[0].length, bars[0].depth),
            (5.0, 20.0, 0)
        );
        assert_eq!(bars[2].label, "build › compile");
        assert_eq!((bars[2].start, bars[2].length), (7.0, 18.0));
        assert!(bars[0].critical && !bars[1].critical);
        assert!(gantt(&RunDetail::default()).is_empty());
    }

    #[test]
    fn slowest_steps_sorted_and_limited() {
        let s = slowest_steps(&detail(), 1);
        assert_eq!(s, vec![("build".to_string(), "compile".to_string(), 18.0)]);
    }

    #[test]
    fn needs_are_inferred_from_timing_only_when_absent() {
        let mut jobs = vec![
            job("build", 0, 10, &[]),
            job("test", 12, 30, &[]),
            job("lint", 0, 5, &[]),
        ];
        infer_needs(&mut jobs);
        assert_eq!(jobs[1].needs, ["build"]);
        assert!(jobs[0].needs.is_empty() && jobs[2].needs.is_empty());
        let mut explicit = vec![
            job("a", 0, 5, &[]),
            job("b", 6, 9, &["a"]),
            job("c", 10, 12, &[]),
        ];
        infer_needs(&mut explicit);
        assert!(explicit[2].needs.is_empty());
    }

    #[test]
    fn seconds_format() {
        assert_eq!(format_seconds(0.0), "0s");
        assert_eq!(format_seconds(75.0), "1m 15s");
        assert_eq!(format_seconds(3700.0), "1h 01m");
        assert_eq!(format_seconds(-5.0), "0s");
    }

    #[test]
    fn backwards_clock_never_gives_negative_length() {
        let j = job("x", 10, 5, &[]);
        assert_eq!(job_seconds(&j), Some(0.0));
    }
}
