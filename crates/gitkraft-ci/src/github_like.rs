//! GitHub Actions and Gitea/Forgejo Actions. Gitea 1.24+ mirrors GitHub's
//! endpoints; an older server answers 404, which surfaces as `NotFound`.

use chrono::{DateTime, Utc};
use gitkraft_platforms::api::Api;
use gitkraft_platforms::http::{HttpClient, Method};
use gitkraft_platforms::{PlatformKind, RepoId, Result};
use serde::Deserialize;
use serde_json::json;

use crate::model::{Artifact, Job, RerunScope, Run, RunDetail, RunQuery, Status, Step, Workflow};
use crate::timing::infer_needs;

/// Adapter for GitHub and Gitea-family Actions.
#[derive(Debug)]
pub struct ActionsBackend<C> {
    pub(crate) api: Api<C>,
}

fn base(repo: &RepoId) -> String {
    format!("/repos/{}/{}/actions", repo.owner, repo.name)
}

/// GitHub `status` + `conclusion`, or a Gitea status, to [`Status`].
pub(crate) fn map_status(status: &str, conclusion: Option<&str>) -> Status {
    let c = conclusion.unwrap_or_default();
    match (status, c) {
        ("completed", "success") | ("success", _) => Status::Success,
        ("completed", "failure" | "timed_out" | "startup_failure") | ("failure" | "failed", _) => {
            Status::Failed
        }
        ("completed", "cancelled") | ("cancelled" | "canceled", _) => Status::Cancelled,
        ("completed", "skipped" | "neutral" | "stale") | ("skipped", _) => Status::Skipped,
        ("completed", "action_required") => Status::Waiting,
        ("in_progress" | "running", _) => Status::Running,
        ("waiting" | "blocked" | "pending" | "requested", _) => Status::Waiting,
        ("queued", _) => Status::Queued,
        (_, "success") => Status::Success,
        (_, "failure") => Status::Failed,
        _ => Status::Queued,
    }
}

impl<C: HttpClient> ActionsBackend<C> {
    pub(crate) fn new(api: Api<C>) -> Self {
        Self { api }
    }

    fn is_github(&self) -> bool {
        self.api.kind == PlatformKind::GitHub
    }

    pub(crate) async fn list_runs(&self, repo: &RepoId, q: &RunQuery) -> Result<Vec<Run>> {
        let mut path = format!(
            "{}/runs?per_page={}&page={}",
            base(repo),
            q.per_page,
            q.page
        );
        if let Some(b) = &q.branch {
            path.push_str(&format!("&branch={}", gitkraft_platforms::url_encode(b)));
        }
        if let Some(s) = q.status {
            // Server-side filters differ per platform; the unified filter is applied after.
            let name = match s {
                Status::Success => "success",
                Status::Failed => "failure",
                Status::Cancelled => "cancelled",
                Status::Running => "in_progress",
                Status::Queued => "queued",
                Status::Skipped => "skipped",
                Status::Waiting | Status::Manual => "waiting",
            };
            path.push_str(&format!("&status={name}"));
        }
        let res: RunsResponse = self.api.get(&path).await?;
        let host = repo.host.clone();
        Ok(res
            .workflow_runs
            .into_iter()
            .map(|d| d.into_model(repo, &host))
            .filter(|r| q.status.is_none_or(|s| r.status == s))
            .collect())
    }

    pub(crate) async fn run_detail(&self, repo: &RepoId, run_id: u64) -> Result<RunDetail> {
        let run: RunDto = self
            .api
            .get(&format!("{}/runs/{run_id}", base(repo)))
            .await?;
        let jobs: JobsResponse = self
            .api
            .get(&format!("{}/runs/{run_id}/jobs?per_page=100", base(repo)))
            .await?;
        let mut jobs: Vec<Job> = jobs.jobs.into_iter().map(JobDto::into_model).collect();
        infer_needs(&mut jobs);
        Ok(RunDetail {
            run: run.into_model(repo, &repo.host),
            jobs,
        })
    }

    pub(crate) async fn job_log(&self, repo: &RepoId, job_id: u64) -> Result<String> {
        // GitHub answers a log request with a redirect and refuses anything but its JSON media
        // type; the redirect target serves the plain text.
        let accept = if self.is_github() {
            "application/vnd.github+json"
        } else {
            "text/plain"
        };
        self.api
            .get_text(&format!("{}/jobs/{job_id}/logs", base(repo)), accept)
            .await
    }

    pub(crate) async fn rerun(&self, repo: &RepoId, run: &Run, scope: RerunScope) -> Result<()> {
        let suffix = match scope {
            RerunScope::All => "rerun",
            RerunScope::Failed => "rerun-failed-jobs",
        };
        self.api
            .post_json(
                &format!("{}/runs/{}/{suffix}", base(repo), run.id),
                &json!({}),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn rerun_job(&self, repo: &RepoId, job_id: u64) -> Result<()> {
        self.api
            .post_json(&format!("{}/jobs/{job_id}/rerun", base(repo)), &json!({}))
            .await
            .map(|_| ())
    }

    pub(crate) async fn cancel(&self, repo: &RepoId, run_id: u64) -> Result<()> {
        self.api
            .post_json(&format!("{}/runs/{run_id}/cancel", base(repo)), &json!({}))
            .await
            .map(|_| ())
    }

    pub(crate) async fn artifacts(&self, repo: &RepoId, run_id: u64) -> Result<Vec<Artifact>> {
        #[derive(Deserialize)]
        struct A {
            id: u64,
            #[serde(default)]
            name: String,
            #[serde(default)]
            size_in_bytes: u64,
            #[serde(default)]
            expired: bool,
        }
        #[derive(Deserialize)]
        struct As {
            #[serde(default)]
            artifacts: Vec<A>,
        }
        let r: As = self
            .api
            .get(&format!("{}/runs/{run_id}/artifacts", base(repo)))
            .await?;
        Ok(r.artifacts
            .into_iter()
            .map(|a| Artifact {
                id: a.id,
                name: a.name,
                size: a.size_in_bytes,
                expired: a.expired,
            })
            .collect())
    }

    pub(crate) async fn download_artifact(
        &self,
        repo: &RepoId,
        artifact_id: u64,
    ) -> Result<Vec<u8>> {
        self.api
            .get_bytes(&format!("{}/artifacts/{artifact_id}/zip", base(repo)))
            .await
    }

    pub(crate) async fn list_workflows(&self, repo: &RepoId) -> Result<Vec<Workflow>> {
        #[derive(Deserialize)]
        struct Wf {
            id: u64,
            #[serde(default)]
            name: String,
            #[serde(default)]
            path: String,
            #[serde(default)]
            state: String,
        }
        #[derive(Deserialize)]
        struct Wfs {
            #[serde(default)]
            workflows: Vec<Wf>,
        }
        let r: Wfs = self.api.get(&format!("{}/workflows", base(repo))).await?;
        Ok(r.workflows
            .into_iter()
            .map(|w| Workflow {
                id: w.id,
                file: w.path.rsplit('/').next().unwrap_or_default().to_string(),
                name: w.name,
                active: w.state.is_empty() || w.state == "active",
            })
            .collect())
    }

    /// The text of a workflow file at `git_ref` (`.github/workflows` first, then `.gitea/workflows`).
    pub(crate) async fn workflow_file(
        &self,
        repo: &RepoId,
        file: &str,
        git_ref: &str,
    ) -> Result<String> {
        use base64::Engine as _;
        let mut last = None;
        for dir in [".github/workflows", ".gitea/workflows"] {
            let path = format!(
                "/repos/{}/{}/contents/{dir}/{}?ref={}",
                repo.owner,
                repo.name,
                gitkraft_platforms::url_encode(file),
                gitkraft_platforms::url_encode(git_ref)
            );
            match self.api.get::<serde_json::Value>(&path).await {
                Ok(v) => {
                    let b64: String = v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .split_whitespace()
                        .collect();
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .map_err(|e| gitkraft_platforms::PlatformError::Parse(e.to_string()))?;
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
                Err(e) => last = Some(e),
            }
        }
        Err(last
            .unwrap_or_else(|| gitkraft_platforms::PlatformError::NotFound("workflow file".into())))
    }

    pub(crate) async fn dispatch(
        &self,
        repo: &RepoId,
        workflow: &str,
        git_ref: &str,
        inputs: &[(String, String)],
    ) -> Result<()> {
        let inputs: serde_json::Map<String, serde_json::Value> =
            inputs.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let body = json!({ "ref": git_ref, "inputs": inputs });
        self.api
            .send(
                Method::Post,
                &format!(
                    "{}/workflows/{}/dispatches",
                    base(repo),
                    gitkraft_platforms::url_encode(workflow)
                ),
                Some(&body),
            )
            .await
            .map(|_| ())
    }

    pub(crate) fn host_label(&self) -> &'static str {
        if self.is_github() {
            "GitHub Actions"
        } else {
            "Gitea Actions"
        }
    }
}

#[derive(Debug, Deserialize)]
struct RunsResponse {
    #[serde(default)]
    workflow_runs: Vec<RunDto>,
}

#[derive(Debug, Deserialize, Default)]
struct ActorDto {
    #[serde(default)]
    login: String,
}

#[derive(Debug, Deserialize)]
struct RunDto {
    id: u64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    display_title: Option<String>,
    #[serde(default)]
    run_number: u64,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    head_branch: Option<String>,
    #[serde(default)]
    head_sha: Option<String>,
    #[serde(default)]
    event: String,
    #[serde(default)]
    actor: Option<ActorDto>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    run_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    stopped: Option<DateTime<Utc>>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    path: Option<String>,
}

impl RunDto {
    fn into_model(self, repo: &RepoId, _host: &str) -> Run {
        let status = map_status(&self.status, self.conclusion.as_deref());
        let started = self.run_started_at.or(self.started_at).or(self.created_at);
        // `updated_at` is the finish time of a completed run.
        let finished = status
            .is_terminal()
            .then(|| self.stopped.or(self.updated_at))
            .flatten();
        let workflow = self
            .path
            .as_deref()
            .and_then(|p| p.split('@').next())
            .and_then(|p| p.rsplit('/').next())
            .map(str::to_string)
            .unwrap_or_default();
        let name = self
            .name
            .filter(|n| !n.is_empty())
            .or_else(|| {
                // `.gitea/workflows/ci.yml@refs/heads/master` → `ci.yml`
                let p = self.path.as_deref()?.split('@').next()?;
                p.rsplit('/').next().map(str::to_string)
            })
            .unwrap_or_default();
        Run {
            id: self.id,
            repo: repo.clone(),
            name,
            workflow,
            title: self.display_title.unwrap_or_default(),
            number: self.run_number,
            status,
            branch: self.head_branch.unwrap_or_default(),
            sha: self.head_sha.unwrap_or_default(),
            event: self.event,
            actor: self.actor.map(|a| a.login).unwrap_or_default(),
            created_at: self.created_at,
            started_at: started,
            finished_at: finished,
            url: self.html_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct JobsResponse {
    #[serde(default)]
    jobs: Vec<JobDto>,
}

#[derive(Debug, Deserialize)]
struct StepDto {
    #[serde(default)]
    number: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct JobDto {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    steps: Vec<StepDto>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    runner_name: Option<String>,
}

impl JobDto {
    fn into_model(self) -> Job {
        Job {
            id: self.id,
            name: self.name,
            stage: None,
            status: map_status(&self.status, self.conclusion.as_deref()),
            allow_failure: false,
            needs: Vec::new(),
            runner: self.runner_name.filter(|r| !r.is_empty()),
            steps: self
                .steps
                .into_iter()
                .map(|s| Step {
                    number: s.number,
                    name: s.name,
                    status: map_status(&s.status, s.conclusion.as_deref()),
                    started_at: s.started_at,
                    finished_at: s.completed_at,
                })
                .collect(),
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.completed_at,
            url: self.html_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_and_gitea_statuses_map_the_same_way() {
        let cases = [
            (("completed", Some("success")), Status::Success),
            (("completed", Some("failure")), Status::Failed),
            (("completed", Some("timed_out")), Status::Failed),
            (("completed", Some("cancelled")), Status::Cancelled),
            (("completed", Some("skipped")), Status::Skipped),
            (("completed", Some("action_required")), Status::Waiting),
            (("in_progress", None), Status::Running),
            (("queued", None), Status::Queued),
            (("waiting", None), Status::Waiting),
            (("success", None), Status::Success),
            (("failure", None), Status::Failed),
            (("running", None), Status::Running),
            (("blocked", None), Status::Waiting),
            (("weird", None), Status::Queued),
        ];
        for ((s, c), want) in cases {
            assert_eq!(map_status(s, c), want, "{s}/{c:?}");
        }
    }
}
