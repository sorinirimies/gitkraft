//! One interface over every platform's CI.

use gitkraft_platforms::api::Api;
use gitkraft_platforms::http::HttpClient;
use gitkraft_platforms::{Account, PlatformError, PlatformKind, RepoId, Result};

use crate::github_like::ActionsBackend;
use crate::gitlab::GitLabCi;
use crate::model::{RerunScope, Run, RunDetail, RunQuery};

/// The CI adapter for one account.
#[derive(Debug)]
pub enum CiBackend<C> {
    /// GitHub Actions, Gitea / Forgejo / Codeberg Actions.
    Actions(ActionsBackend<C>),
    /// GitLab CI.
    GitLab(GitLabCi<C>),
}

impl<C: HttpClient> CiBackend<C> {
    /// Builds the adapter for `account` on top of `client`.
    pub fn new(account: &Account, client: C) -> Self {
        let api = Api::new(
            client,
            account.kind,
            account.api_base(),
            account.token.clone(),
        );
        match account.kind {
            PlatformKind::GitLab => Self::GitLab(GitLabCi::new(api)),
            PlatformKind::GitHub | PlatformKind::Gitea => Self::Actions(ActionsBackend::new(api)),
        }
    }

    /// Name of the CI system, for the UI.
    pub fn system_name(&self) -> &'static str {
        match self {
            Self::Actions(b) => b.host_label(),
            Self::GitLab(_) => "GitLab CI",
        }
    }

    /// Lists runs, newest first.
    pub async fn list_runs(&self, repo: &RepoId, q: &RunQuery) -> Result<Vec<Run>> {
        match self {
            Self::Actions(b) => b.list_runs(repo, q).await,
            Self::GitLab(b) => b.list_runs(repo, q).await,
        }
    }

    /// A run with its jobs and steps.
    pub async fn run_detail(&self, repo: &RepoId, run_id: u64) -> Result<RunDetail> {
        match self {
            Self::Actions(b) => b.run_detail(repo, run_id).await,
            Self::GitLab(b) => b.run_detail(repo, run_id).await,
        }
    }

    /// The raw log of one job (parse it with [`crate::log::parse_log`]).
    pub async fn job_log(&self, repo: &RepoId, job_id: u64) -> Result<String> {
        match self {
            Self::Actions(b) => b.job_log(repo, job_id).await,
            Self::GitLab(b) => b.job_log(repo, job_id).await,
        }
    }

    /// Re-runs all or only the failed jobs of `run`.
    pub async fn rerun(&self, repo: &RepoId, run: &Run, scope: RerunScope) -> Result<()> {
        match self {
            Self::Actions(b) => b.rerun(repo, run, scope).await,
            Self::GitLab(b) => b.rerun(repo, run, scope).await,
        }
    }

    /// Re-runs one job.
    pub async fn rerun_job(&self, repo: &RepoId, job_id: u64) -> Result<()> {
        match self {
            Self::Actions(b) => b.rerun_job(repo, job_id).await,
            Self::GitLab(b) => b.rerun_job(repo, job_id).await,
        }
    }

    /// Cancels a run.
    pub async fn cancel(&self, repo: &RepoId, run_id: u64) -> Result<()> {
        match self {
            Self::Actions(b) => b.cancel(repo, run_id).await,
            Self::GitLab(b) => b.cancel(repo, run_id).await,
        }
    }

    /// Starts a manual job (GitLab). Other platforms have no such thing.
    pub async fn play_job(&self, repo: &RepoId, job_id: u64) -> Result<()> {
        match self {
            Self::Actions(_) => Err(PlatformError::Unsupported("manual jobs on this platform")),
            Self::GitLab(b) => b.play_job(repo, job_id).await,
        }
    }

    /// Files the run produced.
    pub async fn artifacts(
        &self,
        repo: &RepoId,
        run_id: u64,
    ) -> Result<Vec<crate::model::Artifact>> {
        match self {
            Self::Actions(b) => b.artifacts(repo, run_id).await,
            Self::GitLab(b) => b.artifacts(repo, run_id).await,
        }
    }

    /// The bytes of an artifact (a zip archive).
    pub async fn download_artifact(&self, repo: &RepoId, artifact_id: u64) -> Result<Vec<u8>> {
        match self {
            Self::Actions(b) => b.download_artifact(repo, artifact_id).await,
            Self::GitLab(b) => b.download_artifact(repo, artifact_id).await,
        }
    }

    /// Workflows that can be started by hand (GitLab has no such list: it returns none).
    pub async fn list_workflows(&self, repo: &RepoId) -> Result<Vec<crate::model::Workflow>> {
        match self {
            Self::Actions(b) => b.list_workflows(repo).await,
            Self::GitLab(_) => Ok(Vec::new()),
        }
    }

    /// Triggers a workflow on `git_ref` with inputs (GitLab: CI variables).
    pub async fn dispatch(
        &self,
        repo: &RepoId,
        workflow: &str,
        git_ref: &str,
        inputs: &[(String, String)],
    ) -> Result<()> {
        match self {
            Self::Actions(b) => b.dispatch(repo, workflow, git_ref, inputs).await,
            Self::GitLab(b) => b.dispatch(repo, workflow, git_ref, inputs).await,
        }
    }
}
