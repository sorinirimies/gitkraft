//! The tools that change things. Registered only when the server runs with `--write`.

use std::collections::BTreeMap;

use gitkraft_ci::RerunScope;
use gitkraft_hub::dispatch::{Dispatch, start};
use gitkraft_platforms::model::{
    MergeMethod, MergeOptions, MergeTiming, ReviewVerdict, UpdateStyle,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use crate::server::{GitkraftServer, ToolResult, failed, invalid, kind_of, out};

/// Post a comment.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CommentArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// `pr` or `issue`.
    pub kind: String,
    /// Number.
    pub number: u64,
    /// Markdown text.
    pub body: String,
}

/// Submit a review.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ReviewArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Pull request number.
    pub number: u64,
    /// `approve`, `request_changes` or `comment`.
    pub verdict: String,
    /// The review text (required for `request_changes` and `comment`).
    pub body: Option<String>,
}

/// Merge a pull request.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct MergeArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Pull request number.
    pub number: u64,
    /// `merge` (default), `squash` or `rebase`.
    pub method: Option<String>,
    /// `now` (default), `when_green` (auto-merge once the checks pass) or `queue` (merge queue / train).
    pub when: Option<String>,
    /// Merge past branch protection, or jump the queue (admin; the server must run with --allow-force).
    pub force: Option<bool>,
}

/// Bring a pull request's branch up to date.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct UpdateArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Pull request number.
    pub number: u64,
    /// `merge`, `rebase` or omitted for the platform's own way (GitHub merges, GitLab rebases).
    pub style: Option<String>,
}

/// A pull request.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct PrArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Pull request number.
    pub number: u64,
}

/// Close or reopen.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct OpenArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// `pr` or `issue`.
    pub kind: String,
    /// Number.
    pub number: u64,
    /// `true` reopens, `false` closes.
    pub open: bool,
}

/// Change labels.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LabelsArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// `pr` or `issue`.
    pub kind: String,
    /// Number.
    pub number: u64,
    /// Label names to add (they must exist; see list_labels).
    #[serde(default)]
    pub add: Vec<String>,
    /// Label names to remove.
    #[serde(default)]
    pub remove: Vec<String>,
}

/// Re-run a CI run.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RerunArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Run id.
    pub run_id: u64,
    /// `failed` (default) re-runs only the failed jobs, `all` every job.
    pub scope: Option<String>,
}

/// Cancel a CI run.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct CancelArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Run id.
    pub run_id: u64,
}

/// Start a workflow by hand.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct DispatchArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Workflow file (`ci.yml`; see list_workflows). Leave out on GitLab and Woodpecker.
    pub workflow: Option<String>,
    /// Branch or tag to run on.
    pub git_ref: String,
    /// Inputs of the workflow. They are checked against the workflow file first.
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}

fn method(s: Option<&str>) -> Result<MergeMethod, rmcp::ErrorData> {
    match s {
        None | Some("merge") => Ok(MergeMethod::Merge),
        Some("squash") => Ok(MergeMethod::Squash),
        Some("rebase") => Ok(MergeMethod::Rebase),
        Some(o) => Err(invalid(format!(
            "method must be merge, squash or rebase, not '{o}'"
        ))),
    }
}

fn timing(s: Option<&str>) -> Result<MergeTiming, rmcp::ErrorData> {
    match s {
        None | Some("now") => Ok(MergeTiming::Now),
        Some("when_green") => Ok(MergeTiming::WhenGreen),
        Some("queue") => Ok(MergeTiming::Queue),
        Some(o) => Err(invalid(format!(
            "when must be now, when_green or queue, not '{o}'"
        ))),
    }
}

#[tool_router(router = write_router, vis = "pub(crate)")]
impl GitkraftServer {
    #[tool(
        description = "Post a comment on a pull request or issue.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn comment(&self, Parameters(a): Parameters<CommentArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        if a.body.trim().is_empty() {
            return Err(invalid("body is empty"));
        }
        self.backend(&repo)?
            .post_comment(&repo, kind_of(&a.kind)?, a.number, &a.body)
            .await
            .map_err(|e| failed("comment", e))?;
        out(&json!({"ok": true, "message": format!("commented on {}#{}", repo.key(), a.number)}))
    }

    #[tool(
        description = "Review a pull request: approve, request changes or comment.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn review(&self, Parameters(a): Parameters<ReviewArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let verdict = match a.verdict.as_str() {
            "approve" => ReviewVerdict::Approve,
            "request_changes" => ReviewVerdict::RequestChanges,
            "comment" => ReviewVerdict::Comment,
            o => {
                return Err(invalid(format!(
                    "verdict must be approve, request_changes or comment, not '{o}'"
                )));
            }
        };
        let body = a.body.unwrap_or_default();
        if verdict != ReviewVerdict::Approve && body.trim().is_empty() {
            return Err(invalid("a review that is not an approval needs a body"));
        }
        self.backend(&repo)?
            .submit_review(&repo, a.number, verdict, &body)
            .await
            .map_err(|e| failed("review", e))?;
        out(
            &json!({"ok": true, "message": format!("reviewed {}#{} ({})", repo.key(), a.number, a.verdict)}),
        )
    }

    #[tool(
        description = "Merge a pull request: now, when the checks pass (auto-merge), or through the merge queue; as merge commit, squash or rebase. `force` merges past branch protection or jumps the queue and is refused unless the server runs with --allow-force. Replies with what happened: merged, scheduled or queued.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub(crate) async fn merge_pull_request(
        &self,
        Parameters(a): Parameters<MergeArgs>,
    ) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let force = a.force.unwrap_or(false);
        if force && !self.policy.allow_force {
            return Err(invalid(
                "force merge is off: start the server with --write --allow-force to allow it",
            ));
        }
        let options = MergeOptions {
            method: method(a.method.as_deref())?,
            timing: timing(a.when.as_deref())?,
            force,
        };
        let outcome = self
            .backend(&repo)?
            .merge_with(&repo, a.number, &options)
            .await
            .map_err(|e| failed("merge", e))?;
        out(&json!({"ok": true, "message": outcome.describe(a.number)}))
    }

    #[tool(
        description = "Bring a pull request's branch up to date with its target branch.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn update_branch(&self, Parameters(a): Parameters<UpdateArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let style = match a.style.as_deref() {
            None | Some("default") => UpdateStyle::Default,
            Some("merge") => UpdateStyle::Merge,
            Some("rebase") => UpdateStyle::Rebase,
            Some(o) => return Err(invalid(format!("style must be merge or rebase, not '{o}'"))),
        };
        self.backend(&repo)?
            .update_branch(&repo, a.number, style)
            .await
            .map_err(|e| failed("update_branch", e))?;
        out(&json!({"ok": true, "message": format!("#{} is being updated", a.number)}))
    }

    #[tool(
        description = "Cancel a scheduled auto-merge or take a pull request out of the merge queue.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn cancel_auto_merge(&self, Parameters(a): Parameters<PrArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        self.backend(&repo)?
            .cancel_auto_merge(&repo, a.number)
            .await
            .map_err(|e| failed("cancel_auto_merge", e))?;
        out(&json!({"ok": true, "message": format!("#{} will no longer auto-merge", a.number)}))
    }

    #[tool(
        description = "Close or reopen a pull request or issue.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn set_open(&self, Parameters(a): Parameters<OpenArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        self.backend(&repo)?
            .set_open(&repo, kind_of(&a.kind)?, a.number, a.open)
            .await
            .map_err(|e| failed("set_open", e))?;
        out(
            &json!({"ok": true, "message": format!("#{} is now {}", a.number, if a.open { "open" } else { "closed" })}),
        )
    }

    #[tool(
        description = "Add and remove labels on a pull request or issue.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn edit_labels(&self, Parameters(a): Parameters<LabelsArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        if a.add.is_empty() && a.remove.is_empty() {
            return Err(invalid("give labels to add or remove"));
        }
        self.backend(&repo)?
            .edit_labels(&repo, kind_of(&a.kind)?, a.number, &a.add, &a.remove)
            .await
            .map_err(|e| failed("edit_labels", e))?;
        out(&json!({"ok": true, "message": format!("labels of #{} changed", a.number)}))
    }

    #[tool(
        description = "Re-run a CI run: only its failed jobs (default) or all of them.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn ci_rerun(&self, Parameters(a): Parameters<RerunArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let scope = match a.scope.as_deref() {
            None | Some("failed") => RerunScope::Failed,
            Some("all") => RerunScope::All,
            Some(o) => return Err(invalid(format!("scope must be failed or all, not '{o}'"))),
        };
        let ci = self.ci(&repo)?;
        let run = ci
            .run_detail(&repo, a.run_id)
            .await
            .map_err(|e| failed("ci_rerun", e))?
            .run;
        ci.rerun(&repo, &run, scope)
            .await
            .map_err(|e| failed("ci_rerun", e))?;
        out(&json!({"ok": true, "message": format!("run {} is starting again", a.run_id)}))
    }

    #[tool(
        description = "Cancel a running CI run.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    pub(crate) async fn ci_cancel(&self, Parameters(a): Parameters<CancelArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        self.ci(&repo)?
            .cancel(&repo, a.run_id)
            .await
            .map_err(|e| failed("ci_cancel", e))?;
        out(&json!({"ok": true, "message": format!("run {} cancelled", a.run_id)}))
    }

    #[tool(
        description = "Start a workflow by hand (GitHub/Gitea workflow_dispatch, a GitLab or Woodpecker pipeline) on a branch or tag, with inputs checked against the workflow file.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    pub(crate) async fn dispatch_workflow(
        &self,
        Parameters(a): Parameters<DispatchArgs>,
    ) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let d = Dispatch {
            workflow: a.workflow.unwrap_or_default(),
            git_ref: a.git_ref,
            inputs: a.inputs.into_iter().collect(),
        };
        let message = start(self.ci(&repo)?, &repo, &d)
            .await
            .map_err(|e| failed("dispatch_workflow", e))?;
        out(&json!({"ok": true, "message": message}))
    }
}
