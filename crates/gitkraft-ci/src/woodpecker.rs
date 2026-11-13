//! Woodpecker CI (what Codeberg builds with): pipelines are runs, steps are jobs.
//!
//! Woodpecker nests workflows between a pipeline and its steps; here every step is a [`Job`] whose
//! `stage` is its workflow, so a log can be fetched per step like on GitLab.

use base64::Engine as _;
use chrono::{DateTime, Utc};
use gitkraft_platforms::api::Api;
use gitkraft_platforms::http::{HttpClient, Method};
use gitkraft_platforms::{PlatformError, RepoId, Result};
use serde::Deserialize;
use serde_json::json;

use crate::model::{Job, RerunScope, Run, RunDetail, RunQuery, Status};

/// Adapter for a Woodpecker server.
#[derive(Debug)]
pub struct WoodpeckerCi<C> {
    pub(crate) api: Api<C>,
    /// Server URL, for links.
    pub(crate) server: String,
}

fn map_status(s: &str) -> Status {
    match s {
        "success" => Status::Success,
        "failure" | "error" => Status::Failed,
        "killed" | "canceled" | "cancelled" => Status::Cancelled,
        "skipped" | "declined" => Status::Skipped,
        "running" | "started" => Status::Running,
        "blocked" => Status::Waiting,
        _ => Status::Queued,
    }
}

fn time(secs: Option<i64>) -> Option<DateTime<Utc>> {
    secs.filter(|s| *s > 0)
        .and_then(|s| DateTime::from_timestamp(s, 0))
}

#[derive(Deserialize)]
struct RepoDto {
    id: u64,
}

#[derive(Deserialize)]
struct StepDto {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    started: Option<i64>,
    #[serde(default)]
    finished: Option<i64>,
}

#[derive(Deserialize)]
struct WorkflowDto {
    #[serde(default)]
    name: String,
    #[serde(default)]
    children: Vec<StepDto>,
}

#[derive(Deserialize)]
struct PipelineDto {
    #[serde(default)]
    number: u64,
    #[serde(default)]
    status: String,
    #[serde(default)]
    event: String,
    #[serde(default)]
    branch: String,
    #[serde(default)]
    commit: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    created: Option<i64>,
    #[serde(default)]
    started: Option<i64>,
    #[serde(default)]
    finished: Option<i64>,
    #[serde(default)]
    workflows: Vec<WorkflowDto>,
}

#[derive(Deserialize)]
struct LogLineDto {
    #[serde(default)]
    data: String,
}

impl<C: HttpClient> WoodpeckerCi<C> {
    pub(crate) fn new(api: Api<C>, server: String) -> Self {
        Self { api, server }
    }

    /// Woodpecker addresses a repository by its own numeric id.
    async fn repo_id(&self, repo: &RepoId) -> Result<u64> {
        let r: RepoDto = self
            .api
            .get(&format!(
                "/repos/lookup/{}/{}",
                gitkraft_platforms::url_encode(&repo.owner),
                gitkraft_platforms::url_encode(&repo.name)
            ))
            .await?;
        Ok(r.id)
    }

    fn run_of(&self, repo: &RepoId, p: &PipelineDto, id: u64) -> Run {
        let title = if p.title.is_empty() {
            p.message.lines().next().unwrap_or_default().to_string()
        } else {
            p.title.clone()
        };
        Run {
            id: p.number,
            repo: repo.clone(),
            name: "pipeline".into(),
            title,
            number: p.number,
            status: map_status(&p.status),
            branch: p.branch.clone(),
            sha: p.commit.clone(),
            event: p.event.clone(),
            actor: p.author.clone(),
            created_at: time(p.created),
            started_at: time(p.started),
            finished_at: time(p.finished),
            url: format!("{}/repos/{id}/pipeline/{}", self.server, p.number),
            ..Run::default()
        }
    }

    pub(crate) async fn list_runs(&self, repo: &RepoId, q: &RunQuery) -> Result<Vec<Run>> {
        let id = self.repo_id(repo).await?;
        let mut path = format!(
            "/repos/{id}/pipelines?page={}&perPage={}",
            q.page, q.per_page
        );
        if let Some(b) = &q.branch {
            path.push_str(&format!("&branch={}", gitkraft_platforms::url_encode(b)));
        }
        let ps: Vec<PipelineDto> = self.api.get(&path).await?;
        Ok(ps
            .iter()
            .map(|p| self.run_of(repo, p, id))
            .filter(|r| q.status.is_none_or(|s| r.status == s))
            .collect())
    }

    pub(crate) async fn run_detail(&self, repo: &RepoId, run_id: u64) -> Result<RunDetail> {
        let id = self.repo_id(repo).await?;
        let p: PipelineDto = self
            .api
            .get(&format!("/repos/{id}/pipelines/{run_id}"))
            .await?;
        let run = self.run_of(repo, &p, id);
        let jobs = p
            .workflows
            .iter()
            .flat_map(|w| {
                w.children.iter().map(|s| Job {
                    id: s.id,
                    name: s.name.clone(),
                    stage: Some(w.name.clone()),
                    status: map_status(&s.state),
                    started_at: time(s.started),
                    finished_at: time(s.finished),
                    url: run.url.clone(),
                    ..Job::default()
                })
            })
            .collect();
        Ok(RunDetail { run, jobs })
    }

    /// The log of step `job_id`: needs the pipeline number, which the step id alone does not give, so
    /// the repository's latest pipelines are searched.
    pub(crate) async fn job_log(&self, repo: &RepoId, job_id: u64) -> Result<String> {
        let id = self.repo_id(repo).await?;
        let ps: Vec<PipelineDto> = self
            .api
            .get(&format!("/repos/{id}/pipelines?perPage=30"))
            .await?;
        for summary in ps {
            let p: PipelineDto = self
                .api
                .get(&format!("/repos/{id}/pipelines/{}", summary.number))
                .await?;
            if p.workflows
                .iter()
                .any(|w| w.children.iter().any(|s| s.id == job_id))
            {
                let lines: Vec<LogLineDto> = self
                    .api
                    .get(&format!("/repos/{id}/logs/{}/{job_id}", p.number))
                    .await?;
                return Ok(decode_log(&lines));
            }
        }
        Err(PlatformError::NotFound(format!("step {job_id}")))
    }

    /// Woodpecker restarts a whole pipeline; there is no partial re-run.
    pub(crate) async fn rerun(&self, repo: &RepoId, run: &Run, _scope: RerunScope) -> Result<()> {
        let id = self.repo_id(repo).await?;
        self.api
            .send(
                Method::Post,
                &format!("/repos/{id}/pipelines/{}", run.id),
                Some(&json!({})),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn cancel(&self, repo: &RepoId, run_id: u64) -> Result<()> {
        let id = self.repo_id(repo).await?;
        self.api
            .send(
                Method::Post,
                &format!("/repos/{id}/pipelines/{run_id}/cancel"),
                Some(&json!({})),
            )
            .await
            .map(|_| ())
    }

    /// Starts a pipeline on `git_ref` with variables.
    pub(crate) async fn dispatch(
        &self,
        repo: &RepoId,
        git_ref: &str,
        variables: &[(String, String)],
    ) -> Result<()> {
        let id = self.repo_id(repo).await?;
        let vars: serde_json::Map<String, serde_json::Value> = variables
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
        self.api
            .send(
                Method::Post,
                &format!("/repos/{id}/pipelines"),
                Some(&json!({ "branch": git_ref, "variables": vars })),
            )
            .await
            .map(|_| ())
    }
}

/// Log lines carry base64 text; older servers send it plain.
fn decode_log(lines: &[LogLineDto]) -> String {
    let mut out = String::new();
    for l in lines {
        let text = base64::engine::general_purpose::STANDARD
            .decode(l.data.trim())
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_else(|| l.data.clone());
        out.push_str(text.trim_end_matches('\n'));
        out.push('\n');
    }
    out
}
