//! GitLab CI: pipelines are runs, jobs are jobs, there are no steps.

use chrono::{DateTime, Utc};
use gitkraft_platforms::api::Api;
use gitkraft_platforms::http::HttpClient;
use gitkraft_platforms::{PlatformError, RepoId, Result, url_encode};
use serde::Deserialize;
use serde_json::json;

use crate::model::{Job, RerunScope, Run, RunDetail, RunQuery, Status};

/// Adapter for GitLab CI.
#[derive(Debug)]
pub struct GitLabCi<C> {
    pub(crate) api: Api<C>,
}

fn project(repo: &RepoId) -> String {
    format!("/projects/{}", url_encode(&repo.full_name()))
}

fn map_status(s: &str) -> Status {
    match s {
        "success" => Status::Success,
        "failed" => Status::Failed,
        "canceled" | "cancelled" | "canceling" => Status::Cancelled,
        "skipped" => Status::Skipped,
        "running" => Status::Running,
        "manual" => Status::Manual,
        "scheduled" | "waiting_for_resource" | "preparing" => Status::Waiting,
        _ => Status::Queued,
    }
}

fn status_param(s: Status) -> &'static str {
    match s {
        Status::Success => "success",
        Status::Failed => "failed",
        Status::Cancelled => "canceled",
        Status::Running => "running",
        Status::Queued => "pending",
        Status::Skipped => "skipped",
        Status::Manual => "manual",
        Status::Waiting => "scheduled",
    }
}

impl<C: HttpClient> GitLabCi<C> {
    pub(crate) fn new(api: Api<C>) -> Self {
        Self { api }
    }

    pub(crate) async fn list_runs(&self, repo: &RepoId, q: &RunQuery) -> Result<Vec<Run>> {
        let mut path = format!(
            "{}/pipelines?per_page={}&page={}&order_by=id&sort=desc",
            project(repo),
            q.per_page,
            q.page
        );
        if let Some(b) = &q.branch {
            path.push_str(&format!("&ref={}", url_encode(b)));
        }
        if let Some(s) = q.status {
            path.push_str(&format!("&status={}", status_param(s)));
        }
        let dtos: Vec<PipelineDto> = self.api.get(&path).await?;
        Ok(dtos.into_iter().map(|d| d.into_model(repo)).collect())
    }

    pub(crate) async fn run_detail(&self, repo: &RepoId, run_id: u64) -> Result<RunDetail> {
        let p: PipelineDto = self
            .api
            .get(&format!("{}/pipelines/{run_id}", project(repo)))
            .await?;
        let jobs: Vec<JobDto> = self
            .api
            .get(&format!(
                "{}/pipelines/{run_id}/jobs?per_page=100",
                project(repo)
            ))
            .await?;
        let mut jobs: Vec<Job> = jobs.into_iter().map(JobDto::into_model).collect();
        // GitLab runs stages in order: every job waits for the jobs of the stage before it.
        let mut stages: Vec<String> = Vec::new();
        for s in jobs.iter().filter_map(|j| j.stage.clone()) {
            if !stages.contains(&s) {
                stages.push(s);
            }
        }
        let names_in = |jobs: &[Job], stage: &str| -> Vec<String> {
            jobs.iter()
                .filter(|j| j.stage.as_deref() == Some(stage))
                .map(|j| j.name.clone())
                .collect()
        };
        let needs: Vec<Vec<String>> = jobs
            .iter()
            .map(|j| {
                let idx = stages.iter().position(|s| Some(s) == j.stage.as_ref());
                match idx {
                    Some(i) if i > 0 => names_in(&jobs, &stages[i - 1]),
                    _ => Vec::new(),
                }
            })
            .collect();
        for (j, n) in jobs.iter_mut().zip(needs) {
            j.needs = n;
        }
        Ok(RunDetail {
            run: p.into_model(repo),
            jobs,
        })
    }

    pub(crate) async fn job_log(&self, repo: &RepoId, job_id: u64) -> Result<String> {
        self.api
            .get_text(
                &format!("{}/jobs/{job_id}/trace", project(repo)),
                "text/plain",
            )
            .await
    }

    pub(crate) async fn rerun(&self, repo: &RepoId, run: &Run, scope: RerunScope) -> Result<()> {
        match scope {
            RerunScope::Failed => self
                .api
                .post_json(
                    &format!("{}/pipelines/{}/retry", project(repo), run.id),
                    &json!({}),
                )
                .await
                .map(|_| ()),
            RerunScope::All => {
                if run.branch.is_empty() {
                    return Err(PlatformError::Unsupported(
                        "re-running a pipeline without a ref",
                    ));
                }
                self.dispatch(repo, "", &run.branch, &[]).await
            }
        }
    }

    pub(crate) async fn rerun_job(&self, repo: &RepoId, job_id: u64) -> Result<()> {
        self.api
            .post_json(
                &format!("{}/jobs/{job_id}/retry", project(repo)),
                &json!({}),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn cancel(&self, repo: &RepoId, run_id: u64) -> Result<()> {
        self.api
            .post_json(
                &format!("{}/pipelines/{run_id}/cancel", project(repo)),
                &json!({}),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn play_job(&self, repo: &RepoId, job_id: u64) -> Result<()> {
        self.api
            .post_json(&format!("{}/jobs/{job_id}/play", project(repo)), &json!({}))
            .await
            .map(|_| ())
    }

    /// Starts a new pipeline on `git_ref` with CI variables. `_workflow` is unused: GitLab has one pipeline file.
    pub(crate) async fn dispatch(
        &self,
        repo: &RepoId,
        _workflow: &str,
        git_ref: &str,
        inputs: &[(String, String)],
    ) -> Result<()> {
        let variables: Vec<_> = inputs
            .iter()
            .map(|(k, v)| json!({ "key": k, "value": v }))
            .collect();
        self.api
            .post_json(
                &format!("{}/pipeline", project(repo)),
                &json!({ "ref": git_ref, "variables": variables }),
            )
            .await
            .map(|_| ())
    }
}

#[derive(Debug, Deserialize, Default)]
struct UserDto {
    #[serde(default)]
    username: String,
}

#[derive(Debug, Deserialize)]
struct PipelineDto {
    id: u64,
    #[serde(default)]
    iid: Option<u64>,
    #[serde(default)]
    status: String,
    #[serde(default, rename = "ref")]
    git_ref: String,
    #[serde(default)]
    sha: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    user: Option<UserDto>,
    #[serde(default)]
    web_url: String,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
}

impl PipelineDto {
    fn into_model(self, repo: &RepoId) -> Run {
        let status = map_status(&self.status);
        let source = self.source.unwrap_or_default();
        Run {
            id: self.id,
            repo: repo.clone(),
            name: self
                .name
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| "pipeline".into()),
            workflow: String::new(),
            title: self.sha.chars().take(8).collect(),
            number: self.iid.unwrap_or(self.id),
            status,
            branch: self.git_ref,
            sha: self.sha,
            event: source,
            actor: self.user.map(|u| u.username).unwrap_or_default(),
            created_at: self.created_at,
            started_at: self.started_at.or(self.created_at),
            finished_at: self
                .finished_at
                .or_else(|| status.is_terminal().then_some(self.updated_at).flatten()),
            url: self.web_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RunnerDto {
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct JobDto {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    allow_failure: bool,
    #[serde(default)]
    runner: Option<RunnerDto>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    web_url: String,
}

impl JobDto {
    fn into_model(self) -> Job {
        Job {
            id: self.id,
            name: self.name,
            stage: self.stage,
            status: map_status(&self.status),
            allow_failure: self.allow_failure,
            needs: Vec::new(),
            runner: self.runner.map(|r| r.description).filter(|d| !d.is_empty()),
            steps: Vec::new(),
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            url: self.web_url,
        }
    }
}
