//! Actions the pages send as JSON: comment, review, merge, close, CI re-run, notification switch.

use gitkraft_ci::{RerunScope, Run};
use gitkraft_platforms::model::{MergeMethod, ReviewVerdict};
use gitkraft_platforms::{ItemKind, RepoId};
use gitkraft_settings::files;
use gitkraft_settings::notify::set_enabled;
use serde::Deserialize;

use crate::session::Session;

/// Pull request or issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A pull / merge request.
    Pr,
    /// An issue.
    Issue,
}

/// One action of the page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionRequest {
    /// Post a comment.
    Comment {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
        /// PR or issue.
        kind: Kind,
        /// Text.
        body: String,
    },
    /// Merge a PR.
    Merge {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
        /// `merge`, `squash` or `rebase`.
        #[serde(default)]
        method: Option<String>,
    },
    /// Review a PR.
    Review {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
        /// `approve`, `request_changes` or `comment`.
        verdict: String,
        /// Note.
        #[serde(default)]
        body: String,
    },
    /// Close or reopen.
    SetOpen {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
        /// PR or issue.
        kind: Kind,
        /// The state it should have.
        open: bool,
    },
    /// Re-run CI.
    Rerun {
        /// `host/owner/name`.
        repo: String,
        /// Run id.
        run_id: u64,
        /// Branch of the run (GitLab needs it to start a fresh pipeline).
        #[serde(default)]
        branch: String,
        /// `all` or `failed`.
        scope: String,
    },
    /// Cancel a CI run.
    Cancel {
        /// `host/owner/name`.
        repo: String,
        /// Run id.
        run_id: u64,
    },
    /// Turn notifications on or off for a project (`repo` empty: everything).
    NotifyToggle {
        /// `host/owner/name`, or empty for the global switch.
        #[serde(default)]
        repo: String,
        /// New state.
        on: bool,
    },
}

fn repo_of(key: &str) -> Result<RepoId, String> {
    RepoId::from_key(key).ok_or_else(|| format!("'{key}' is not host/owner/name"))
}

/// Runs `req`; returns a short confirmation or the reason it failed.
pub async fn perform(session: &Session, req: ActionRequest) -> Result<String, String> {
    let hub = session.hub();
    let result = match req {
        ActionRequest::Comment {
            repo,
            number,
            kind,
            body,
        } => {
            if body.trim().is_empty() {
                return Err("a comment needs some text".into());
            }
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let kind = if kind == Kind::Pr {
                ItemKind::PullRequest
            } else {
                ItemKind::Issue
            };
            backend
                .post_comment(&repo, kind, number, body.trim())
                .await
                .map(|()| "comment posted".to_string())
        }
        ActionRequest::Merge {
            repo,
            number,
            method,
        } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let method = match method.as_deref() {
                None | Some("merge") => MergeMethod::Merge,
                Some("squash") => MergeMethod::Squash,
                Some("rebase") => MergeMethod::Rebase,
                Some(other) => return Err(format!("unknown merge method '{other}'")),
            };
            backend
                .merge(&repo, number, method)
                .await
                .map(|()| format!("#{number} merged"))
        }
        ActionRequest::Review {
            repo,
            number,
            verdict,
            body,
        } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let verdict = match verdict.as_str() {
                "approve" => ReviewVerdict::Approve,
                "request_changes" => ReviewVerdict::RequestChanges,
                "comment" => ReviewVerdict::Comment,
                other => return Err(format!("unknown verdict '{other}'")),
            };
            if verdict == ReviewVerdict::RequestChanges && body.trim().is_empty() {
                return Err("requesting changes needs a note".into());
            }
            backend
                .submit_review(&repo, number, verdict, body.trim())
                .await
                .map(|()| "review sent".to_string())
        }
        ActionRequest::SetOpen {
            repo,
            number,
            kind,
            open,
        } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let kind = if kind == Kind::Pr {
                ItemKind::PullRequest
            } else {
                ItemKind::Issue
            };
            backend
                .set_open(&repo, kind, number, open)
                .await
                .map(|()| format!("#{number} {}", if open { "reopened" } else { "closed" }))
        }
        ActionRequest::Rerun {
            repo,
            run_id,
            branch,
            scope,
        } => {
            let repo = repo_of(&repo)?;
            let ci = hub.ci(&repo.host).ok_or("no account for this host")?;
            let scope = match scope.as_str() {
                "all" => RerunScope::All,
                "failed" => RerunScope::Failed,
                other => return Err(format!("unknown scope '{other}'")),
            };
            let run = Run {
                id: run_id,
                branch,
                ..Run::default()
            };
            ci.rerun(&repo, &run, scope)
                .await
                .map(|()| "re-run started".to_string())
        }
        ActionRequest::Cancel { repo, run_id } => {
            let repo = repo_of(&repo)?;
            let ci = hub.ci(&repo.host).ok_or("no account for this host")?;
            ci.cancel(&repo, run_id)
                .await
                .map(|()| "run cancelled".to_string())
        }
        ActionRequest::NotifyToggle { repo, on } => {
            return toggle_notifications(session, &repo, on);
        }
    };
    session.invalidate();
    result.map_err(|e| e.to_string())
}

fn toggle_notifications(session: &Session, repo: &str, on: bool) -> Result<String, String> {
    let path = session
        .notify_path()
        .ok_or("this session has no notification config file")?;
    let key = if repo.is_empty() {
        None
    } else {
        Some(repo_of(repo)?.key())
    };
    let text = files::read_optional(path)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let edited = set_enabled(&text, key.as_deref(), on)?;
    files::write_atomic(path, &edited).map_err(|e| e.to_string())?;
    Ok(format!(
        "notifications {} for {}",
        if on { "on" } else { "off" },
        key.as_deref().unwrap_or("all projects")
    ))
}
