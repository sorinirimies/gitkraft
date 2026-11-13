//! Normalized CI types: run (pipeline) → job → step, the same on every platform.

use chrono::{DateTime, Utc};
use gitkraft_platforms::RepoId;
use serde::{Deserialize, Serialize};

/// State of a run, job or step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Status {
    /// Waiting for a runner.
    #[default]
    Queued,
    /// Executing.
    Running,
    /// Finished green.
    Success,
    /// Finished red.
    Failed,
    /// Cancelled by someone.
    Cancelled,
    /// Not run.
    Skipped,
    /// Waits for a manual start.
    Manual,
    /// Waits for approval or a dependency.
    Waiting,
}

impl Status {
    /// Finished, whatever the outcome.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Success | Self::Failed | Self::Cancelled | Self::Skipped
        )
    }

    /// Still moving: queued, waiting or running.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Waiting)
    }

    /// One glyph; the ASCII form works in any terminal and without colour.
    pub fn glyph(self, ascii: bool) -> &'static str {
        match (self, ascii) {
            (Self::Success, false) => "✔",
            (Self::Success, true) => "+",
            (Self::Failed, false) => "✘",
            (Self::Failed, true) => "x",
            (Self::Running, false) => "●",
            (Self::Running, true) => "*",
            (Self::Queued, false) => "○",
            (Self::Queued, true) => ".",
            (Self::Cancelled, false) => "⊘",
            (Self::Cancelled, true) => "-",
            (Self::Skipped, false) => "↷",
            (Self::Skipped, true) => ">",
            (Self::Manual, false) => "▶",
            (Self::Manual, true) => "p",
            (Self::Waiting, false) => "⧗",
            (Self::Waiting, true) => "w",
        }
    }
}

/// One step of a job (GitHub / Gitea). GitLab jobs have none.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Step {
    /// 1-based position.
    pub number: u32,
    /// Name.
    pub name: String,
    /// State.
    pub status: Status,
    /// Start time.
    pub started_at: Option<DateTime<Utc>>,
    /// End time.
    pub finished_at: Option<DateTime<Utc>>,
}

/// One job of a run.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Job {
    /// Platform id (used to fetch the log).
    pub id: u64,
    /// Name, matrix values included.
    pub name: String,
    /// Stage (GitLab).
    pub stage: Option<String>,
    /// State.
    pub status: Status,
    /// Failure is allowed (GitLab `allow_failure`).
    pub allow_failure: bool,
    /// Names of jobs it waits for.
    pub needs: Vec<String>,
    /// Runner that ran it.
    pub runner: Option<String>,
    /// Steps, when the platform has them.
    pub steps: Vec<Step>,
    /// When it was created / queued.
    pub created_at: Option<DateTime<Utc>>,
    /// Start time.
    pub started_at: Option<DateTime<Utc>>,
    /// End time.
    pub finished_at: Option<DateTime<Utc>>,
    /// Web URL.
    pub url: String,
}

/// One workflow run or pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Run {
    /// Platform id.
    pub id: u64,
    /// Project.
    pub repo: RepoId,
    /// Workflow name (GitLab: pipeline source).
    pub name: String,
    /// Workflow file name (`ci.yml`), what dispatching a run needs. Empty on GitLab.
    #[serde(default)]
    pub workflow: String,
    /// Commit message or run title.
    pub title: String,
    /// Run number within the workflow.
    pub number: u64,
    /// State.
    pub status: Status,
    /// Branch or tag.
    pub branch: String,
    /// Commit sha.
    pub sha: String,
    /// What triggered it: `push`, `pull_request`, `schedule`…
    pub event: String,
    /// Who triggered it.
    pub actor: String,
    /// Created.
    pub created_at: Option<DateTime<Utc>>,
    /// Started running.
    pub started_at: Option<DateTime<Utc>>,
    /// Finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Web URL.
    pub url: String,
}

impl Run {
    /// Wall-clock duration of a finished run.
    pub fn duration(&self) -> Option<chrono::Duration> {
        Some(self.finished_at? - self.started_at.or(self.created_at)?)
    }
}

/// A workflow that can be started by hand.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Workflow {
    /// Platform id.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// File name, e.g. `ci.yml`.
    pub file: String,
    /// Enabled.
    pub active: bool,
}

/// A run with its jobs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RunDetail {
    /// The run.
    pub run: Run,
    /// Its jobs, in platform order.
    pub jobs: Vec<Job>,
}

/// Filters for run lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunQuery {
    /// Only this branch.
    pub branch: Option<String>,
    /// Only runs in this state.
    pub status: Option<Status>,
    /// 1-based page.
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
}

impl Default for RunQuery {
    fn default() -> Self {
        Self {
            branch: None,
            status: None,
            page: 1,
            per_page: 30,
        }
    }
}

/// What to re-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerunScope {
    /// Every job.
    All,
    /// Only the failed jobs.
    Failed,
}
