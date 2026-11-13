//! GitLab adapter: merge requests are mapped onto pull requests.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::api::Api;
use crate::error::{PlatformError, Result};
use crate::github_like::fold_days;
use crate::http::{HttpClient, Method};
use crate::model::{
    ActivityDay, Capabilities, ChecksState, Comment, Issue, ItemState, Label, ListQuery,
    MergeMethod, Notification, NotificationReason, PullRequest, Review, ReviewVerdict, StateFilter,
    SubjectKind, User,
};
use crate::platform::{RepoId, url_encode};
use gitkraft_settings::dashboard::Involvement;

/// Adapter for gitlab.com and self-hosted GitLab.
#[derive(Debug)]
pub struct GitLab<C> {
    pub(crate) api: Api<C>,
    host: String,
}

/// Which kind of item a number refers to; GitLab uses different endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// Pull / merge request.
    PullRequest,
    /// Issue.
    Issue,
}

fn project(repo: &RepoId) -> String {
    format!("/projects/{}", url_encode(&repo.full_name()))
}

fn item_path(repo: &RepoId, kind: ItemKind, number: u64) -> String {
    let seg = match kind {
        ItemKind::PullRequest => "merge_requests",
        ItemKind::Issue => "issues",
    };
    format!("{}/{seg}/{number}", project(repo))
}

impl<C: HttpClient> GitLab<C> {
    pub(crate) fn new(api: Api<C>, host: String) -> Self {
        Self { api, host }
    }

    pub(crate) fn capabilities(&self) -> Capabilities {
        Capabilities {
            merge_methods: true,
            reviews: true,
            notifications: true,
            contributions: true,
            checks: true,
        }
    }

    pub(crate) async fn current_user(&self) -> Result<User> {
        let dto: UserDto = self.api.get("/user").await?;
        Ok(dto.into())
    }

    fn list_params(q: &ListQuery, issues: bool) -> String {
        let state = match q.state {
            StateFilter::Open => "opened",
            StateFilter::Closed if issues => "closed",
            StateFilter::Closed | StateFilter::All => "all",
        };
        let mut p = format!(
            "state={state}&per_page={}&page={}",
            q.page.per_page, q.page.page
        );
        for (name, v) in [
            ("author_username", &q.author),
            ("assignee_username", &q.assignee),
            ("labels", &q.label),
        ] {
            if let Some(v) = v {
                p.push_str(&format!("&{name}={}", url_encode(v)));
            }
        }
        p
    }

    pub(crate) async fn list_pull_requests(
        &self,
        repo: &RepoId,
        q: &ListQuery,
    ) -> Result<Vec<PullRequest>> {
        let path = format!(
            "{}/merge_requests?{}",
            project(repo),
            Self::list_params(q, false)
        );
        let dtos: Vec<MrDto> = self.api.get(&path).await?;
        Ok(dtos
            .into_iter()
            .map(|d| d.into_model(repo))
            // GitLab has no "closed incl. merged" filter, so `all` is narrowed here.
            .filter(|pr| q.state != StateFilter::Closed || pr.state != ItemState::Open)
            .collect())
    }

    fn scope_params(who: Involvement, me: &str, state: StateFilter, issues: bool) -> String {
        let scope = match who {
            Involvement::Authored => "created_by_me",
            Involvement::Assigned => "assigned_to_me",
            Involvement::ReviewRequested => "all",
        };
        let state = match state {
            StateFilter::Open => "opened",
            StateFilter::Closed if issues => "closed",
            StateFilter::Closed | StateFilter::All => "all",
        };
        let mut p = format!("scope={scope}&state={state}&per_page=50&order_by=updated_at");
        if who == Involvement::ReviewRequested {
            p.push_str(&format!("&reviewer_username={}", url_encode(me)));
        }
        p
    }

    /// Merge requests across all projects, by how you relate to them.
    pub(crate) async fn search_pull_requests(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<PullRequest>> {
        let path = format!(
            "/merge_requests?{}",
            Self::scope_params(who, me, state, false)
        );
        let dtos: Vec<MrDto> = self.api.get(&path).await?;
        Ok(dtos
            .into_iter()
            .filter_map(|d| {
                let repo = d.repo(&self.host)?;
                Some(d.into_model(&repo))
            })
            .filter(|pr| state != StateFilter::Closed || pr.state != ItemState::Open)
            .collect())
    }

    /// Issues across all projects, by how you relate to them.
    pub(crate) async fn search_issues(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<Issue>> {
        let path = format!("/issues?{}", Self::scope_params(who, me, state, true));
        let dtos: Vec<IssueDto> = self.api.get(&path).await?;
        Ok(dtos
            .into_iter()
            .filter_map(|d| {
                let repo = d.repo(&self.host)?;
                Some(d.into_model(&repo))
            })
            .collect())
    }

    pub(crate) async fn pull_request(&self, repo: &RepoId, number: u64) -> Result<PullRequest> {
        let dto: MrDto = self
            .api
            .get(&item_path(repo, ItemKind::PullRequest, number))
            .await?;
        Ok(dto.into_model(repo))
    }

    pub(crate) async fn pull_request_diff(&self, repo: &RepoId, number: u64) -> Result<String> {
        let changes: Changes = self
            .api
            .get(&format!(
                "{}/changes",
                item_path(repo, ItemKind::PullRequest, number)
            ))
            .await?;
        let mut out = String::new();
        for c in changes.changes {
            out.push_str(&format!(
                "diff --git a/{o} b/{n}\n--- a/{o}\n+++ b/{n}\n{d}",
                o = c.old_path,
                n = c.new_path,
                d = c.diff
            ));
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        Ok(out)
    }

    pub(crate) async fn list_issues(&self, repo: &RepoId, q: &ListQuery) -> Result<Vec<Issue>> {
        let path = format!("{}/issues?{}", project(repo), Self::list_params(q, true));
        let dtos: Vec<IssueDto> = self.api.get(&path).await?;
        Ok(dtos.into_iter().map(|d| d.into_model(repo)).collect())
    }

    pub(crate) async fn issue(&self, repo: &RepoId, number: u64) -> Result<Issue> {
        let dto: IssueDto = self
            .api
            .get(&item_path(repo, ItemKind::Issue, number))
            .await?;
        Ok(dto.into_model(repo))
    }

    pub(crate) async fn comments(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
    ) -> Result<Vec<Comment>> {
        let notes: Vec<NoteDto> = self
            .api
            .get(&format!(
                "{}/notes?per_page=100&sort=asc",
                item_path(repo, kind, number)
            ))
            .await?;
        Ok(notes
            .into_iter()
            .filter(|n| !n.system)
            .map(|n| Comment {
                id: n.id,
                author: n.author.map(Into::into).unwrap_or_default(),
                body: n.body,
                created_at: n.created_at,
            })
            .collect())
    }

    pub(crate) async fn reviews(&self, repo: &RepoId, number: u64) -> Result<Vec<Review>> {
        let approvals: Approvals = self
            .api
            .get(&format!(
                "{}/approvals",
                item_path(repo, ItemKind::PullRequest, number)
            ))
            .await?;
        Ok(approvals
            .approved_by
            .into_iter()
            .map(|a| Review {
                author: a.user.into(),
                verdict: Some(ReviewVerdict::Approve),
                body: String::new(),
                submitted_at: None,
            })
            .collect())
    }

    pub(crate) async fn post_comment(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
        body: &str,
    ) -> Result<()> {
        self.api
            .post_json(
                &format!("{}/notes", item_path(repo, kind, number)),
                &json!({ "body": body }),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn set_open(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
        open: bool,
    ) -> Result<()> {
        let event = if open { "reopen" } else { "close" };
        self.api
            .send(
                Method::Put,
                &item_path(repo, kind, number),
                Some(&json!({ "state_event": event })),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn merge(
        &self,
        repo: &RepoId,
        number: u64,
        method: MergeMethod,
    ) -> Result<()> {
        if method == MergeMethod::Rebase {
            return Err(PlatformError::Unsupported("rebase merge on GitLab"));
        }
        let squash = method == MergeMethod::Squash;
        self.api
            .send(
                Method::Put,
                &format!("{}/merge", item_path(repo, ItemKind::PullRequest, number)),
                Some(&json!({ "squash": squash })),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn submit_review(
        &self,
        repo: &RepoId,
        number: u64,
        verdict: ReviewVerdict,
        body: &str,
    ) -> Result<()> {
        match verdict {
            ReviewVerdict::Approve => {
                self.api
                    .post_json(
                        &format!("{}/approve", item_path(repo, ItemKind::PullRequest, number)),
                        &json!({}),
                    )
                    .await?;
                if !body.trim().is_empty() {
                    self.post_comment(repo, ItemKind::PullRequest, number, body)
                        .await?;
                }
                Ok(())
            }
            ReviewVerdict::Comment => {
                self.post_comment(repo, ItemKind::PullRequest, number, body)
                    .await
            }
            ReviewVerdict::RequestChanges => {
                Err(PlatformError::Unsupported("request changes on GitLab"))
            }
        }
    }

    pub(crate) async fn comment_on_line(
        &self,
        repo: &RepoId,
        number: u64,
        path: &str,
        line: u32,
        body: &str,
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct Refs {
            #[serde(default)]
            base_sha: String,
            #[serde(default)]
            start_sha: String,
            #[serde(default)]
            head_sha: String,
        }
        #[derive(Deserialize)]
        struct Mr {
            diff_refs: Option<Refs>,
        }
        let item = item_path(repo, ItemKind::PullRequest, number);
        let mr: Mr = self.api.get(&item).await?;
        let r = mr.diff_refs.ok_or(PlatformError::Unsupported(
            "this merge request has no diff refs yet",
        ))?;
        self.api
            .post_json(
                &format!("{item}/discussions"),
                &json!({
                    "body": body,
                    "position": {
                        "position_type": "text",
                        "base_sha": r.base_sha,
                        "start_sha": r.start_sha,
                        "head_sha": r.head_sha,
                        "old_path": path,
                        "new_path": path,
                        "new_line": line,
                    },
                }),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn notifications(&self) -> Result<Vec<Notification>> {
        let todos: Vec<TodoDto> = self.api.get("/todos?state=pending&per_page=50").await?;
        Ok(todos
            .into_iter()
            .map(|t| t.into_model(&self.host))
            .collect())
    }

    pub(crate) async fn mark_read(&self, id: &str) -> Result<()> {
        self.api
            .post_json(&format!("/todos/{id}/mark_as_done"), &json!({}))
            .await
            .map(|_| ())
    }

    pub(crate) async fn contributions(&self, user: &str) -> Result<Vec<ActivityDay>> {
        let url = format!(
            "https://{}/users/{}/calendar.json",
            self.host,
            url_encode(user)
        );
        let map: std::collections::HashMap<String, u32> = self.api.get(&url).await?;
        Ok(fold_days(map.into_iter().filter_map(|(d, c)| {
            Some((d.parse::<NaiveDate>().ok()?, c))
        })))
    }
}

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct UserDto {
    #[serde(default)]
    username: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

impl From<UserDto> for User {
    fn from(d: UserDto) -> Self {
        User {
            login: d.username,
            avatar_url: d.avatar_url.filter(|u| !u.is_empty()),
        }
    }
}

fn labels(v: Vec<String>) -> Vec<Label> {
    v.into_iter()
        .map(|name| Label {
            name,
            color: String::new(),
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct PipelineDto {
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize, Default)]
struct References {
    #[serde(default)]
    full: String,
}

/// `group/sub/proj!12` or `group/proj#3` → `group/sub/proj`.
fn repo_from_reference(full: &str, host: &str) -> Option<RepoId> {
    let path = full.split(['!', '#']).next()?;
    let (owner, name) = path.rsplit_once('/')?;
    (!owner.is_empty() && !name.is_empty()).then(|| RepoId::new(host, owner, name))
}

#[derive(Debug, Deserialize)]
struct MrDto {
    #[serde(default)]
    references: Option<References>,
    iid: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<UserDto>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    draft: Option<bool>,
    #[serde(default)]
    source_branch: String,
    #[serde(default)]
    target_branch: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<UserDto>,
    #[serde(default)]
    reviewers: Vec<UserDto>,
    #[serde(default)]
    user_notes_count: u32,
    #[serde(default)]
    has_conflicts: Option<bool>,
    #[serde(default)]
    head_pipeline: Option<PipelineDto>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    web_url: String,
}

impl MrDto {
    fn repo(&self, host: &str) -> Option<RepoId> {
        repo_from_reference(&self.references.as_ref()?.full, host)
    }

    fn into_model(self, repo: &RepoId) -> PullRequest {
        let state = match self.state.as_str() {
            "opened" => ItemState::Open,
            "merged" => ItemState::Merged,
            _ => ItemState::Closed,
        };
        let checks = self.head_pipeline.map(|p| match p.status.as_str() {
            "success" => ChecksState::Success,
            "failed" | "canceled" => ChecksState::Failure,
            _ => ChecksState::Pending,
        });
        PullRequest {
            repo: repo.clone(),
            number: self.iid,
            draft: self.draft.unwrap_or(false) || self.title.to_lowercase().starts_with("draft:"),
            title: self.title,
            body: self.description.unwrap_or_default(),
            author: self.author.map(Into::into).unwrap_or_default(),
            state,
            source_branch: self.source_branch,
            target_branch: self.target_branch,
            labels: labels(self.labels),
            assignees: self.assignees.into_iter().map(Into::into).collect(),
            reviewers: self.reviewers.into_iter().map(Into::into).collect(),
            comments: self.user_notes_count,
            additions: None,
            deletions: None,
            mergeable: self.has_conflicts.map(|c| !c),
            checks,
            created_at: self.created_at,
            updated_at: self.updated_at,
            url: self.web_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct MilestoneDto {
    #[serde(default)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct IssueDto {
    #[serde(default)]
    references: Option<References>,
    iid: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<UserDto>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<UserDto>,
    #[serde(default)]
    user_notes_count: u32,
    #[serde(default)]
    milestone: Option<MilestoneDto>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    web_url: String,
}

impl IssueDto {
    fn repo(&self, host: &str) -> Option<RepoId> {
        repo_from_reference(&self.references.as_ref()?.full, host)
    }

    fn into_model(self, repo: &RepoId) -> Issue {
        Issue {
            repo: repo.clone(),
            number: self.iid,
            title: self.title,
            body: self.description.unwrap_or_default(),
            author: self.author.map(Into::into).unwrap_or_default(),
            state: if self.state == "opened" {
                ItemState::Open
            } else {
                ItemState::Closed
            },
            labels: labels(self.labels),
            assignees: self.assignees.into_iter().map(Into::into).collect(),
            comments: self.user_notes_count,
            milestone: self.milestone.map(|m| m.title).filter(|t| !t.is_empty()),
            created_at: self.created_at,
            updated_at: self.updated_at,
            url: self.web_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NoteDto {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    author: Option<UserDto>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    system: bool,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct Changes {
    #[serde(default)]
    changes: Vec<ChangeDto>,
}
#[derive(Debug, Deserialize)]
struct ChangeDto {
    #[serde(default)]
    old_path: String,
    #[serde(default)]
    new_path: String,
    #[serde(default)]
    diff: String,
}

#[derive(Debug, Deserialize)]
struct Approvals {
    #[serde(default)]
    approved_by: Vec<ApprovedBy>,
}
#[derive(Debug, Deserialize)]
struct ApprovedBy {
    user: UserDto,
}

#[derive(Debug, Deserialize)]
struct TodoTarget {
    #[serde(default)]
    iid: Option<u64>,
    #[serde(default)]
    title: String,
}
#[derive(Debug, Deserialize)]
struct TodoProject {
    #[serde(default)]
    path_with_namespace: String,
}
#[derive(Debug, Deserialize)]
struct TodoDto {
    id: u64,
    #[serde(default)]
    action_name: String,
    #[serde(default)]
    target_type: String,
    target: Option<TodoTarget>,
    project: Option<TodoProject>,
    #[serde(default)]
    target_url: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

impl TodoDto {
    fn into_model(self, host: &str) -> Notification {
        let kind = match self.target_type.as_str() {
            "MergeRequest" => SubjectKind::PullRequest,
            "Issue" => SubjectKind::Issue,
            "Commit" => SubjectKind::Commit,
            _ => SubjectKind::Other,
        };
        let reason = match self.action_name.as_str() {
            "review_requested" | "approval_required" => NotificationReason::ReviewRequested,
            "mentioned" | "directly_addressed" => NotificationReason::Mention,
            "assigned" => NotificationReason::Assigned,
            "build_failed" => NotificationReason::CiActivity,
            _ => NotificationReason::Other,
        };
        let repo = self.project.and_then(|p| {
            let (owner, name) = p.path_with_namespace.rsplit_once('/')?;
            Some(RepoId::new(host, owner, name))
        });
        Notification {
            id: self.id.to_string(),
            repo,
            kind,
            number: self.target.as_ref().and_then(|t| t.iid),
            title: self.target.map(|t| t.title).unwrap_or_default(),
            reason,
            unread: self.state == "pending",
            updated_at: self.updated_at.or(self.created_at),
            url: self.target_url,
        }
    }
}
