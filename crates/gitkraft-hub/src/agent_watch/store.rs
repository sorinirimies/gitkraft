//! Reports on disk: `<dir>/<job>/<timestamp>.json` plus `<timestamp>.md`, newest wins. The folder
//! is also the "last run" memory, so a restarted app does not run a job twice.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::scan::Finding;

/// What one run found and said.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// The job's name.
    pub job: String,
    /// Started.
    pub started: DateTime<Utc>,
    /// Finished.
    pub finished: DateTime<Utc>,
    /// Which harness looked (`builtin`, `pi`, …) or `none` when everything was green.
    pub harness: String,
    /// The model, when one was used.
    #[serde(default)]
    pub model: Option<String>,
    /// Repositories scanned.
    pub repos: usize,
    /// Workflows judged.
    pub workflows: usize,
    /// What is red.
    pub findings: Vec<Finding>,
    /// Repositories or accounts that could not be read.
    #[serde(default)]
    pub errors: Vec<String>,
    /// The agent's markdown answer, or a one-line summary when no agent ran.
    pub summary: String,
    /// The run itself failed (no harness, no account): the report says why.
    #[serde(default)]
    pub failed: Option<String>,
    /// Has the user looked at it.
    #[serde(default)]
    pub seen: bool,
}

impl Report {
    /// Does it need the user's attention?
    #[must_use]
    pub fn needs_you(&self) -> bool {
        self.failed.is_some() || !self.findings.is_empty()
    }

    /// One line: `3 findings · 12 repos · pi`.
    #[must_use]
    pub fn line(&self) -> String {
        if let Some(e) = &self.failed {
            return format!("failed: {e}");
        }
        let n = self.findings.len();
        format!(
            "{} · {} repos · {}",
            if n == 0 {
                "green".to_string()
            } else {
                format!("{n} red")
            },
            self.repos,
            self.harness
        )
    }

    /// The report as markdown, for reading outside gitkraft.
    #[must_use]
    pub fn markdown(&self) -> String {
        let mut s = format!(
            "# {} — {}\n\n_{} · {} repos, {} workflows_\n\n",
            self.job,
            self.started.format("%Y-%m-%d %H:%M UTC"),
            self.line(),
            self.repos,
            self.workflows
        );
        for f in &self.findings {
            s.push_str(&format!("- {} {}\n", f.headline(), f.url));
            if let Some(e) = &f.explanation {
                s.push_str(&format!("\n{}\n\n", e.trim()));
            }
        }
        if !self.errors.is_empty() {
            s.push_str("\nCould not read:\n");
            for e in &self.errors {
                s.push_str(&format!("- {e}\n"));
            }
        }
        s.push_str(&format!("\n{}\n", self.summary.trim()));
        s
    }
}

/// Held while a run is in progress; released on drop.
#[derive(Debug)]
pub struct Lock {
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// How long a lease survives without being renewed (a crashed instance's is then taken over).
pub const LEASE_STALE: std::time::Duration = std::time::Duration::from_secs(90);

/// The right to run the schedule: one running GitKraft instance holds it, so two open windows
/// (or the TUI beside the GUI) never both start the same daily job. It is renewed on every tick
/// and released when the instance goes away.
#[derive(Debug)]
pub struct Lease {
    path: PathBuf,
    token: String,
}

impl Lease {
    fn token() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("{}-{nanos}", std::process::id())
    }

    /// Takes the lease in `dir`; `None` while another live instance holds it.
    #[must_use]
    pub fn acquire(dir: &Path) -> Option<Self> {
        std::fs::create_dir_all(dir).ok()?;
        let path = dir.join("scheduler.lease");
        let fresh = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < LEASE_STALE);
        if fresh {
            return None;
        }
        let token = Self::token();
        std::fs::write(&path, &token).ok()?;
        // Two instances may have written at once: the last writer is the one the file names.
        (std::fs::read_to_string(&path).ok()? == token).then_some(Self { path, token })
    }

    /// Keeps the lease alive; `false` when somebody else has taken it.
    pub fn renew(&self) -> bool {
        if std::fs::read_to_string(&self.path).is_ok_and(|t| t == self.token) {
            std::fs::write(&self.path, &self.token).is_ok()
        } else {
            false
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if std::fs::read_to_string(&self.path).is_ok_and(|t| t == self.token) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// The report folder.
#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

fn safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl Store {
    /// A store under `dir` (created when a report is saved).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The default folder: the user's data directory.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("gitkraft").join("agent-watch"))
    }

    fn job_dir(&self, job: &str) -> PathBuf {
        self.dir.join(safe(job))
    }

    /// Writes `report` (json and markdown).
    ///
    /// # Errors
    /// The folder or a file cannot be written.
    pub fn save(&self, report: &Report) -> Result<PathBuf, String> {
        let dir = self.job_dir(&report.job);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = report.started.format("%Y%m%dT%H%M%SZ").to_string();
        let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{stem}.json")), json).map_err(|e| e.to_string())?;
        let md = dir.join(format!("{stem}.md"));
        std::fs::write(&md, report.markdown()).map_err(|e| e.to_string())?;
        Ok(md)
    }

    /// Reports of `job`, newest first; unreadable files are skipped.
    #[must_use]
    pub fn reports(&self, job: &str) -> Vec<Report> {
        let Ok(rd) = std::fs::read_dir(self.job_dir(job)) else {
            return Vec::new();
        };
        let mut v: Vec<Report> = rd
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok())
            .collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.started));
        v
    }

    /// The newest report of `job`.
    #[must_use]
    pub fn latest(&self, job: &str) -> Option<Report> {
        self.reports(job).into_iter().next()
    }

    /// Marks every report of `job` as seen.
    ///
    /// # Errors
    /// A file cannot be rewritten.
    pub fn mark_seen(&self, job: &str) -> Result<(), String> {
        for mut r in self.reports(job).into_iter().filter(|r| !r.seen) {
            r.seen = true;
            self.save(&r)?;
        }
        Ok(())
    }

    /// Reports nobody has looked at that need the user, over every job.
    #[must_use]
    pub fn unseen(&self, jobs: &[String]) -> usize {
        jobs.iter()
            .filter_map(|j| self.latest(j))
            .filter(|r| !r.seen && r.needs_you())
            .count()
    }

    /// Takes the "a run is in progress" lock; `None` when another process holds it (a lock older
    /// than an hour is a crashed run's and is taken over).
    #[must_use]
    pub fn try_lock(&self) -> Option<Lock> {
        std::fs::create_dir_all(&self.dir).ok()?;
        let path = self.dir.join("run.lock");
        let stale = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > std::time::Duration::from_secs(3600));
        if stale {
            let _ = std::fs::remove_file(&path);
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .ok()
            .map(|_| Lock { path })
    }

    /// The folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn report(job: &str, minute: u32, findings: usize) -> Report {
        let t = Utc
            .with_ymd_and_hms(2026, 3, 1, 8, minute, 0)
            .single()
            .expect("time");
        Report {
            job: job.into(),
            started: t,
            finished: t,
            harness: "none".into(),
            model: None,
            repos: 3,
            workflows: 5,
            findings: (0..findings)
                .map(|i| Finding {
                    repo: "h/o/r".into(),
                    kind: super::super::scan::FindingKind::FailedRun,
                    what: format!("w{i}"),
                    url: "u".into(),
                    run_id: Some(1),
                    branch: None,
                    number: None,
                    explanation: Some("because".into()),
                })
                .collect(),
            errors: vec![],
            summary: "sum".into(),
            failed: None,
            seen: false,
        }
    }

    #[test]
    fn reports_are_kept_newest_first_and_a_job_name_cannot_escape_the_folder() {
        let tmp = tempfile::tempdir().expect("tmp");
        let s = Store::new(tmp.path());
        s.save(&report("ci/../x", 1, 0)).expect("save");
        s.save(&report("ci/../x", 5, 2)).expect("save");
        let r = s.reports("ci/../x");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].findings.len(), 2, "newest first");
        assert!(tmp.path().join("ci____x").is_dir());
        assert!(r[0].markdown().contains("h/o/r w0 u") && r[0].markdown().contains("because"));
    }

    #[test]
    fn one_instance_holds_the_schedule_and_another_takes_over_when_it_goes() {
        let tmp = tempfile::tempdir().expect("tmp");
        let first = Lease::acquire(tmp.path()).expect("free");
        assert!(first.renew());
        assert!(Lease::acquire(tmp.path()).is_none(), "held and fresh");
        drop(first);
        let second = Lease::acquire(tmp.path()).expect("released on drop");
        // a crashed instance leaves an old file behind
        let old = std::time::SystemTime::now() - LEASE_STALE - std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(tmp.path().join("scheduler.lease"))
            .and_then(|f| f.set_modified(old))
            .expect("age it");
        let third = Lease::acquire(tmp.path()).expect("stale: taken over");
        assert!(!second.renew(), "the old holder learns it lost it");
        assert!(third.renew());
    }

    #[test]
    fn only_one_run_holds_the_lock_at_a_time() {
        let tmp = tempfile::tempdir().expect("tmp");
        let s = Store::new(tmp.path());
        let first = s.try_lock().expect("free");
        assert!(s.try_lock().is_none(), "held");
        drop(first);
        assert!(s.try_lock().is_some(), "released on drop");
    }

    #[test]
    fn unseen_counts_reports_that_need_you_until_they_are_marked_seen() {
        let tmp = tempfile::tempdir().expect("tmp");
        let s = Store::new(tmp.path());
        s.save(&report("a", 1, 1)).expect("save");
        s.save(&report("b", 1, 0)).expect("save");
        let jobs = vec!["a".to_string(), "b".to_string()];
        assert_eq!(s.unseen(&jobs), 1, "a green report needs nobody");
        s.mark_seen("a").expect("seen");
        assert_eq!(s.unseen(&jobs), 0);
        assert_eq!(s.reports("a").len(), 1, "marking does not duplicate");
    }
}
