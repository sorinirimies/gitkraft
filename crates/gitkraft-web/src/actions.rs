//! Actions the pages send as JSON: comment, review, merge, close, CI re-run, notification switch.

use gitkraft_ci::{RerunScope, Run};
use gitkraft_platforms::model::{
    MergeMethod, MergeOptions, MergeTiming, ReviewVerdict, UpdateStyle,
};
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
    /// Comment on a GitHub discussion.
    DiscussionComment {
        /// `host/owner/name`.
        repo: String,
        /// Discussion number.
        number: u64,
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
        /// `now` (default), `when_green` or `queue`.
        #[serde(default)]
        when: Option<String>,
        /// Merge past branch rules (admin), or jump the queue.
        #[serde(default)]
        force: bool,
    },
    /// Bring a PR's branch up to date with its target.
    UpdateBranch {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
        /// `merge`, `rebase`, or absent for the platform's own way.
        #[serde(default)]
        style: Option<String>,
    },
    /// Cancel a scheduled auto-merge or leave the merge queue.
    CancelAutoMerge {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
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
    /// Ask the AI why a CI job failed (a draft: it changes nothing).
    Explain {
        /// `host/owner/name`.
        repo: String,
        /// Run id.
        run_id: u64,
        /// The job to explain; absent or not failed: the first failed one.
        #[serde(default)]
        job_id: Option<u64>,
    },
    /// Start a workflow by hand.
    Dispatch {
        /// `host/owner/name`.
        repo: String,
        /// Workflow file such as `ci.yml`; empty where there are none (GitLab).
        #[serde(default)]
        workflow: String,
        /// Branch or tag.
        git_ref: String,
        /// `key=value` pairs separated by spaces or new lines.
        #[serde(default)]
        inputs: String,
    },
    /// Fetch a pull request into a worktree next to the local checkout.
    Worktree {
        /// `host/owner/name`.
        repo: String,
        /// Number.
        number: u64,
    },
    /// Create or reuse this machine's key for a host, register it and configure ssh.
    SshSetup {
        /// Host name, e.g. `github.com`.
        host: String,
    },
    /// Remove a key from an account.
    SshRevoke {
        /// Host name.
        host: String,
        /// Platform id of the key.
        id: String,
    },
    /// Run `ssh -T git@host`.
    SshTest {
        /// Host name.
        host: String,
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
        ActionRequest::DiscussionComment { repo, number, body } => {
            if body.trim().is_empty() {
                return Err("a comment needs some text".into());
            }
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            backend
                .comment_on_discussion(&repo, number, body.trim())
                .await
                .map(|()| "comment posted".to_string())
        }
        ActionRequest::Merge {
            repo,
            number,
            method,
            when,
            force,
        } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let method = match method.as_deref() {
                None | Some("merge") => MergeMethod::Merge,
                Some("squash") => MergeMethod::Squash,
                Some("rebase") => MergeMethod::Rebase,
                Some(other) => return Err(format!("unknown merge method '{other}'")),
            };
            let timing = match when.as_deref() {
                None | Some("now") => MergeTiming::Now,
                Some("when_green") => MergeTiming::WhenGreen,
                Some("queue") => MergeTiming::Queue,
                Some(other) => return Err(format!("unknown merge timing '{other}'")),
            };
            let options = MergeOptions {
                method,
                timing,
                force,
            };
            backend
                .merge_with(&repo, number, &options)
                .await
                .map(|o| o.describe(number))
        }
        ActionRequest::UpdateBranch {
            repo,
            number,
            style,
        } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            let style = match style.as_deref() {
                None | Some("default") => UpdateStyle::Default,
                Some("merge") => UpdateStyle::Merge,
                Some("rebase") => UpdateStyle::Rebase,
                Some(other) => return Err(format!("unknown update style '{other}'")),
            };
            backend
                .update_branch(&repo, number, style)
                .await
                .map(|()| format!("#{number} is being updated"))
        }
        ActionRequest::CancelAutoMerge { repo, number } => {
            let repo = repo_of(&repo)?;
            let backend = hub.backend(&repo.host).ok_or("no account for this host")?;
            backend
                .cancel_auto_merge(&repo, number)
                .await
                .map(|()| format!("#{number} will no longer auto-merge"))
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
        ActionRequest::Explain {
            repo,
            run_id,
            job_id,
        } => {
            let repo = repo_of(&repo)?;
            let llm = tokio::task::spawn_blocking(gitkraft_hub::ai::remote_default)
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            return explain_with(session, &llm, &repo, run_id, job_id).await;
        }
        ActionRequest::Dispatch {
            repo,
            workflow,
            git_ref,
            inputs,
        } => {
            let repo = repo_of(&repo)?;
            let ci = hub.ci(&repo.host).ok_or("no account for this host")?;
            if workflow.contains(char::is_whitespace) || git_ref.contains(char::is_whitespace) {
                return Err("workflow and branch must not contain spaces".into());
            }
            let needs = !workflow.is_empty();
            let text = format!("{workflow} {git_ref} {}", inputs.replace('\n', " "));
            let d = gitkraft_hub::dispatch::parse(&text, needs)?;
            let out = gitkraft_hub::dispatch::start(ci, &repo, &d).await;
            session.invalidate();
            return out;
        }
        ActionRequest::Worktree { repo, number } => {
            let repo = repo_of(&repo)?;
            let kind = hub
                .backend(&repo.host)
                .ok_or("no account for this host")?
                .kind();
            let path = session.checkout_of(&repo).ok_or_else(|| {
                format!(
                    "no local checkout of {} is known: start gitkraft-web inside it, or pass --checkout DIR",
                    repo.full_name()
                )
            })?;
            return tokio::task::spawn_blocking(move || {
                gitkraft_hub::checkout::checkout_worktree(kind, &repo, number, &path)
            })
            .await
            .map_err(|e| e.to_string())?;
        }
        ActionRequest::SshSetup { host } => {
            let dir = session
                .ssh_dir()
                .ok_or("this session may not change the ssh directory")?
                .to_path_buf();
            let machine = gitkraft_hub::keys::machine_name();
            let out = gitkraft_hub::keys::setup_host(
                hub,
                &host,
                &dir,
                &gitkraft_platforms::ssh::SystemSsh,
                &machine,
            )
            .await;
            session.invalidate();
            return out.map(|t| t.lines().collect::<Vec<_>>().join(" · "));
        }
        ActionRequest::SshRevoke { host, id } => {
            let out = gitkraft_hub::keys::revoke(hub, &host, &id).await;
            session.invalidate();
            return out;
        }
        ActionRequest::SshTest { host } => {
            session.ssh_dir().ok_or("this session may not run ssh")?;
            let (probe, line) = tokio::task::spawn_blocking(move || {
                gitkraft_hub::keys::test_host(&gitkraft_platforms::ssh::SystemSsh, &host)
            })
            .await
            .map_err(|e| e.to_string())?;
            return if matches!(probe, gitkraft_platforms::ssh::Probe::Authenticated { .. }) {
                Ok(line)
            } else {
                Err(line)
            };
        }
        ActionRequest::NotifyToggle { repo, on } => {
            return toggle_notifications(session, &repo, on);
        }
    };
    session.invalidate();
    result.map_err(|e| e.to_string())
}

/// Asks `llm` why a job of the run failed.
pub(crate) async fn explain_with(
    session: &Session,
    llm: &impl gitkraft_hub::ai::Llm,
    repo: &RepoId,
    run_id: u64,
    job_id: Option<u64>,
) -> Result<String, String> {
    let ci = session
        .hub()
        .ci(&repo.host)
        .ok_or("no account for this host")?;
    let detail = ci
        .run_detail(repo, run_id)
        .await
        .map_err(|e| e.to_string())?;
    let job = gitkraft_hub::ai::failed_job(&detail.jobs, job_id)
        .ok_or_else(|| format!("no failed job in run {run_id}"))?;
    let ctx = gitkraft_hub::ai::failure_context(ci, &detail.run, job).await?;
    let text = gitkraft_hub::ai::explain_failure(llm, &ctx)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("{} / {}\n\n{text}", ctx.workflow, ctx.job))
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

#[cfg(test)]
mod tests {
    use gitkraft_hub::Hub;
    use gitkraft_hub::ai::FakeLlm;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, DynClient, PlatformKind, Secret};
    use gitkraft_settings::dashboard::Dashboard;

    use super::*;
    use crate::session::Paths;

    fn session(fake: &FakeClient) -> Session {
        let mut account =
            Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
        account.me = Some("me".into());
        let hub = Hub::new(vec![account], &DynClient::new(fake.clone()));
        let paths = Paths {
            dashboard: None,
            notify: None,
            use_state_dir: false,
            ssh_dir: None,
            checkouts: Vec::new(),
        };
        Session::new(hub, Dashboard { sections: vec![] }, paths, 0)
    }

    const BASE: &str = "https://api.github.com/repos/o/r/actions";

    fn failed_run(fake: &FakeClient) {
        fake.push_json(
            &format!("{BASE}/runs/5"),
            r#"{"id":5,"name":"CI","run_number":9,"status":"completed","conclusion":"failure","head_branch":"main"}"#,
        );
        fake.push_json(
            &format!("{BASE}/runs/5/jobs"),
            r#"{"jobs":[{"id":10,"name":"lint","status":"completed","conclusion":"success","steps":[]},{"id":11,"name":"build","status":"completed","conclusion":"failure","steps":[]}]}"#,
        );
    }

    #[tokio::test]
    async fn explain_sends_the_redacted_log_of_the_failed_job_and_returns_the_answer() {
        let fake = FakeClient::new();
        failed_run(&fake);
        fake.push_status(
            &format!("{BASE}/jobs/11/logs"),
            200,
            "error[E0425]: no x\nGITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz0123456789\n",
        );
        let llm = FakeLlm::saying("Cause: typo");
        let repo = RepoId::from_key("github.com/o/r").expect("repo");
        let out = explain_with(&session(&fake), &llm, &repo, 5, None)
            .await
            .expect("answer");
        assert!(out.starts_with("CI / build"), "{out}");
        assert!(out.contains("Cause: typo"));
        let prompts = llm.prompts();
        assert!(prompts[0].1.contains("E0425"));
        assert!(
            !prompts[0].1.contains("ghp_abcdefghijkl"),
            "secret reached the model"
        );
    }

    #[tokio::test]
    async fn explain_names_the_requested_job_only_when_it_failed_and_refuses_a_run_without_failures()
     {
        let fake = FakeClient::new();
        failed_run(&fake);
        fake.push_status(&format!("{BASE}/jobs/11/logs"), 200, "boom");
        let repo = RepoId::from_key("github.com/o/r").expect("repo");
        // Job 10 passed: the failed one is explained instead.
        let out = explain_with(&session(&fake), &FakeLlm::saying("x"), &repo, 5, Some(10))
            .await
            .expect("answer");
        assert!(out.contains("CI / build"));

        let fake = FakeClient::new();
        fake.push_json(
            &format!("{BASE}/runs/5"),
            r#"{"id":5,"name":"CI","status":"completed","conclusion":"success"}"#,
        );
        fake.push_json(
            &format!("{BASE}/runs/5/jobs"),
            r#"{"jobs":[{"id":10,"name":"lint","status":"completed","conclusion":"success","steps":[]}]}"#,
        );
        let err = explain_with(&session(&fake), &FakeLlm::saying("x"), &repo, 5, None)
            .await
            .expect_err("nothing to explain");
        assert!(err.contains("no failed job"), "{err}");
    }
}
