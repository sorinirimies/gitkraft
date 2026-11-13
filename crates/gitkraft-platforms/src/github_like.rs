//! GitHub and Gitea/Forgejo/Codeberg: Gitea deliberately mirrors the GitHub
//! REST API, so one adapter serves both and branches only where they differ.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::api::{Api, parse};
use crate::error::{PlatformError, Result};
use crate::http::{HttpClient, Method};
use crate::model::{
    ActivityDay, ActivityEntry, ActivityKind, Capabilities, ChecksState, Comment, DayActivity,
    Issue, ItemState, Label, ListQuery, MergeMethod, MergeOptions, MergeOutcome, MergeTiming,
    Notification, NotificationReason, PullRequest, Review, ReviewVerdict, StateFilter, SubjectKind,
    UpdateStyle, User,
};
use crate::platform::{PlatformKind, RepoId};
use gitkraft_settings::dashboard::Involvement;

/// Adapter for GitHub (and Enterprise) and Gitea-family hosts.
#[derive(Debug)]
pub struct GitHubLike<C> {
    pub(crate) api: Api<C>,
    host: String,
}

impl<C: HttpClient> GitHubLike<C> {
    pub(crate) fn new(api: Api<C>, host: String) -> Self {
        Self { api, host }
    }

    fn is_github(&self) -> bool {
        self.api.kind == PlatformKind::GitHub
    }

    pub(crate) fn repo_path(repo: &RepoId) -> String {
        format!("/repos/{}/{}", repo.owner, repo.name)
    }

    fn state_param(state: StateFilter) -> &'static str {
        match state {
            StateFilter::Open => "open",
            StateFilter::Closed => "closed",
            StateFilter::All => "all",
        }
    }

    pub(crate) fn capabilities(&self) -> Capabilities {
        Capabilities {
            merge_methods: true,
            reviews: true,
            notifications: true,
            contributions: !self.is_github() || self.api.token.is_some(),
            checks: true,
        }
    }

    pub(crate) async fn current_user(&self) -> Result<User> {
        let dto: UserDto = self.api.get("/user").await?;
        Ok(dto.into())
    }

    pub(crate) async fn list_pull_requests(
        &self,
        repo: &RepoId,
        q: &ListQuery,
    ) -> Result<Vec<PullRequest>> {
        let path = format!(
            "{}/pulls?state={}&per_page={}&page={}",
            Self::repo_path(repo),
            Self::state_param(q.state),
            q.page.per_page,
            q.page.page
        );
        let dtos: Vec<PullDto> = self.api.get(&path).await?;
        // Neither platform filters PR lists by author or label server side.
        Ok(dtos
            .into_iter()
            .map(|d| d.into_model(repo))
            .filter(|pr| {
                q.author
                    .as_ref()
                    .is_none_or(|a| pr.author.login.eq_ignore_ascii_case(a))
                    && q.assignee.as_ref().is_none_or(|a| {
                        pr.assignees.iter().any(|u| u.login.eq_ignore_ascii_case(a))
                    })
                    && q.label
                        .as_ref()
                        .is_none_or(|l| pr.labels.iter().any(|x| x.name == *l))
            })
            .collect())
    }

    pub(crate) async fn pull_request(&self, repo: &RepoId, number: u64) -> Result<PullRequest> {
        let dto: PullDto = self
            .api
            .get(&format!("{}/pulls/{number}", Self::repo_path(repo)))
            .await?;
        let sha = dto.head.as_ref().and_then(|h| h.sha.clone());
        let mut pr = dto.into_model(repo);
        if let Some(sha) = sha {
            pr.checks = self.checks(repo, &sha).await.ok().flatten();
        }
        Ok(pr)
    }

    async fn checks(&self, repo: &RepoId, sha: &str) -> Result<Option<ChecksState>> {
        let status: CombinedStatus = self
            .api
            .get(&format!("{}/commits/{sha}/status", Self::repo_path(repo)))
            .await?;
        Ok(match status.state.as_str() {
            "success" => Some(ChecksState::Success),
            "failure" | "error" => Some(ChecksState::Failure),
            "pending" => Some(ChecksState::Pending),
            _ => None,
        })
    }

    pub(crate) async fn pull_request_diff(&self, repo: &RepoId, number: u64) -> Result<String> {
        let accept = if self.is_github() {
            "application/vnd.github.v3.diff"
        } else {
            "text/plain"
        };
        self.api
            .get_text(
                &format!("{}/pulls/{number}.diff", Self::repo_path(repo)),
                accept,
            )
            .await
    }

    pub(crate) async fn list_issues(&self, repo: &RepoId, q: &ListQuery) -> Result<Vec<Issue>> {
        let mut path = format!(
            "{}/issues?state={}&per_page={}&page={}",
            Self::repo_path(repo),
            Self::state_param(q.state),
            q.page.per_page,
            q.page.page
        );
        if self.is_github() {
            push_param(&mut path, "creator", q.author.as_deref());
            push_param(&mut path, "assignee", q.assignee.as_deref());
        } else {
            path.push_str("&type=issues");
            push_param(&mut path, "created_by", q.author.as_deref());
            push_param(&mut path, "assigned_by", q.assignee.as_deref());
        }
        push_param(&mut path, "labels", q.label.as_deref());
        let dtos: Vec<IssueDto> = self.api.get(&path).await?;
        // GitHub's issue list also returns pull requests.
        Ok(dtos
            .into_iter()
            .filter(|d| d.pull_request.is_none())
            .map(|d| d.into_model(repo))
            .collect())
    }

    pub(crate) async fn issue(&self, repo: &RepoId, number: u64) -> Result<Issue> {
        let dto: IssueDto = self
            .api
            .get(&format!("{}/issues/{number}", Self::repo_path(repo)))
            .await?;
        Ok(dto.into_model(repo))
    }

    /// Pull requests across every repo you can see, by how you relate to them.
    pub(crate) async fn search_pull_requests(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<PullRequest>> {
        let hits = self.search("pr", who, state, me).await?;
        Ok(hits
            .into_iter()
            .filter_map(|d| d.into_search_pr(&self.host))
            .collect())
    }

    /// Issues across every repo you can see, by how you relate to them.
    pub(crate) async fn search_issues(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<Issue>> {
        let hits = self.search("issue", who, state, me).await?;
        Ok(hits
            .into_iter()
            .filter_map(|d| d.into_search_issue(&self.host))
            .collect())
    }

    async fn search(
        &self,
        kind: &str,
        who: Involvement,
        state: StateFilter,
        _me: &str,
    ) -> Result<Vec<IssueDto>> {
        if self.is_github() {
            let relation = match who {
                Involvement::Authored => "author:@me",
                Involvement::Assigned => "assignee:@me",
                Involvement::ReviewRequested => "review-requested:@me",
            };
            let state = match state {
                StateFilter::Open => "+is:open",
                StateFilter::Closed => "+is:closed",
                StateFilter::All => "",
            };
            let path =
                format!("/search/issues?q=is:{kind}{state}+{relation}&sort=updated&per_page=50");
            let res: SearchResponse = self.api.get(&path).await?;
            Ok(res.items)
        } else {
            let type_ = if kind == "pr" { "pulls" } else { "issues" };
            let relation = match who {
                Involvement::Authored => "created=true",
                Involvement::Assigned => "assigned=true",
                Involvement::ReviewRequested => "review_requested=true",
            };
            let path = format!(
                "/repos/issues/search?type={type_}&state={}&{relation}&limit=50",
                Self::state_param(state)
            );
            self.api.get(&path).await
        }
    }

    pub(crate) async fn comments(&self, repo: &RepoId, number: u64) -> Result<Vec<Comment>> {
        let dtos: Vec<CommentDto> = self
            .api
            .get(&format!(
                "{}/issues/{number}/comments?per_page=100",
                Self::repo_path(repo)
            ))
            .await?;
        Ok(dtos.into_iter().map(Into::into).collect())
    }

    pub(crate) async fn reviews(&self, repo: &RepoId, number: u64) -> Result<Vec<Review>> {
        let dtos: Vec<ReviewDto> = self
            .api
            .get(&format!(
                "{}/pulls/{number}/reviews?per_page=100",
                Self::repo_path(repo)
            ))
            .await?;
        Ok(dtos.into_iter().map(Into::into).collect())
    }

    pub(crate) async fn post_comment(&self, repo: &RepoId, number: u64, body: &str) -> Result<()> {
        self.api
            .post_json(
                &format!("{}/issues/{number}/comments", Self::repo_path(repo)),
                &json!({ "body": body }),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn list_labels(&self, repo: &RepoId) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct L {
            name: String,
        }
        let ls: Vec<L> = self
            .api
            .get(&format!(
                "{}/labels?per_page=100&limit=100",
                Self::repo_path(repo)
            ))
            .await?;
        Ok(ls.into_iter().map(|l| l.name).collect())
    }

    pub(crate) async fn edit_labels(
        &self,
        repo: &RepoId,
        number: u64,
        add: &[String],
        remove: &[String],
    ) -> Result<()> {
        let base = format!("{}/issues/{number}/labels", Self::repo_path(repo));
        if !add.is_empty() {
            self.api.post_json(&base, &json!({ "labels": add })).await?;
        }
        if remove.is_empty() {
            return Ok(());
        }
        // GitHub removes by name; Gitea by id.
        let ids: Vec<(u64, String)> = if self.is_github() {
            Vec::new()
        } else {
            #[derive(Deserialize)]
            struct L {
                id: u64,
                name: String,
            }
            let ls: Vec<L> = self
                .api
                .get(&format!("{}/labels?limit=100", Self::repo_path(repo)))
                .await?;
            ls.into_iter().map(|l| (l.id, l.name)).collect()
        };
        for name in remove {
            let key = if self.is_github() {
                crate::url_encode(name)
            } else {
                ids.iter()
                    .find(|(_, n)| n == name)
                    .map(|(id, _)| id.to_string())
                    .ok_or_else(|| PlatformError::NotFound(format!("label '{name}'")))?
            };
            self.api
                .send(Method::Delete, &format!("{base}/{key}"), None)
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn set_open(&self, repo: &RepoId, number: u64, open: bool) -> Result<()> {
        let state = if open { "open" } else { "closed" };
        self.api
            .send(
                Method::Patch,
                &format!("{}/issues/{number}", Self::repo_path(repo)),
                Some(&json!({ "state": state })),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn merge_with(
        &self,
        repo: &RepoId,
        number: u64,
        options: &MergeOptions,
    ) -> Result<MergeOutcome> {
        let name = match options.method {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        };
        let path = format!("{}/pulls/{number}/merge", Self::repo_path(repo));
        if self.is_github() {
            return match options.timing {
                MergeTiming::Now => self
                    .api
                    .send(Method::Put, &path, Some(&json!({ "merge_method": name })))
                    .await
                    .map(|_| MergeOutcome::Merged)
                    .map_err(|e| if options.force { force_hint(e) } else { e }),
                MergeTiming::WhenGreen => {
                    let id = self.node_id(repo, number).await?;
                    self.graphql(
                        "mutation($id: ID!, $m: PullRequestMergeMethod!) { enablePullRequestAutoMerge(input: {pullRequestId: $id, mergeMethod: $m}) { clientMutationId } }",
                        &json!({ "id": id, "m": name.to_uppercase() }),
                    )
                    .await?;
                    Ok(MergeOutcome::Scheduled)
                }
                MergeTiming::Queue => {
                    let id = self.node_id(repo, number).await?;
                    self.graphql(
                        "mutation($id: ID!, $jump: Boolean) { enqueuePullRequest(input: {pullRequestId: $id, jump: $jump}) { clientMutationId } }",
                        &json!({ "id": id, "jump": options.force }),
                    )
                    .await?;
                    Ok(MergeOutcome::Queued)
                }
            };
        }
        let mut body = json!({ "Do": name });
        match options.timing {
            MergeTiming::Now => {
                if options.force {
                    body["force_merge"] = json!(true);
                }
            }
            MergeTiming::WhenGreen | MergeTiming::Queue => {
                if options.force {
                    return Err(PlatformError::Unsupported("jumping a queue on Gitea"));
                }
                body["merge_when_checks_succeed"] = json!(true);
            }
        }
        self.api.post_json(&path, &body).await?;
        Ok(match options.timing {
            MergeTiming::Now => MergeOutcome::Merged,
            MergeTiming::WhenGreen => MergeOutcome::Scheduled,
            // Gitea has no queue: it schedules the merge for when checks pass.
            MergeTiming::Queue => MergeOutcome::Scheduled,
        })
    }

    pub(crate) async fn update_branch(
        &self,
        repo: &RepoId,
        number: u64,
        style: UpdateStyle,
    ) -> Result<()> {
        let base = format!("{}/pulls/{number}", Self::repo_path(repo));
        if self.is_github() {
            if style == UpdateStyle::Rebase {
                return Err(PlatformError::Unsupported("rebasing a branch on GitHub"));
            }
            return self
                .api
                .send(
                    Method::Put,
                    &format!("{base}/update-branch"),
                    Some(&json!({})),
                )
                .await
                .map(|_| ());
        }
        let style = if style == UpdateStyle::Rebase {
            "rebase"
        } else {
            "merge"
        };
        self.api
            .post_json(&format!("{base}/update?style={style}"), &json!({}))
            .await
            .map(|_| ())
    }

    pub(crate) async fn cancel_auto_merge(&self, repo: &RepoId, number: u64) -> Result<()> {
        if !self.is_github() {
            return self
                .api
                .send(
                    Method::Delete,
                    &format!("{}/pulls/{number}/merge", Self::repo_path(repo)),
                    None,
                )
                .await
                .map(|_| ());
        }
        let id = self.node_id(repo, number).await?;
        let vars = json!({ "id": id });
        let disable = self
            .graphql(
                "mutation($id: ID!) { disablePullRequestAutoMerge(input: {pullRequestId: $id}) { clientMutationId } }",
                &vars,
            )
            .await;
        let Err(first) = disable else { return Ok(()) };
        // Not auto-merging: it may be in the queue instead.
        self.graphql(
            "mutation($id: ID!) { dequeuePullRequest(input: {id: $id}) { clientMutationId } }",
            &vars,
        )
        .await
        .map(|_| ())
        .map_err(|_| first)
    }

    async fn node_id(&self, repo: &RepoId, number: u64) -> Result<String> {
        #[derive(Deserialize)]
        struct N {
            node_id: String,
        }
        let n: N = self
            .api
            .get(&format!("{}/pulls/{number}", Self::repo_path(repo)))
            .await?;
        Ok(n.node_id)
    }

    /// One GraphQL call; GraphQL reports failures inside a 200, so `errors` is checked here.
    pub(crate) async fn graphql(
        &self,
        query: &str,
        variables: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = graphql_url(&self.api.base);
        let res = self
            .api
            .post_json(&url, &json!({ "query": query, "variables": variables }))
            .await?;
        let v: serde_json::Value = parse(&res)?;
        if let Some(msg) = v["errors"]
            .as_array()
            .and_then(|e| e.first())
            .and_then(|e| e["message"].as_str())
        {
            return Err(PlatformError::Http {
                status: 422,
                message: msg.to_string(),
            });
        }
        Ok(v)
    }

    pub(crate) async fn submit_review(
        &self,
        repo: &RepoId,
        number: u64,
        verdict: ReviewVerdict,
        body: &str,
    ) -> Result<()> {
        let event = match verdict {
            ReviewVerdict::Approve => "APPROVE",
            ReviewVerdict::RequestChanges => "REQUEST_CHANGES",
            ReviewVerdict::Comment => "COMMENT",
        };
        self.api
            .post_json(
                &format!("{}/pulls/{number}/reviews", Self::repo_path(repo)),
                &json!({ "event": event, "body": body }),
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn comment_on_line(
        &self,
        repo: &RepoId,
        number: u64,
        at: &crate::diffpos::Anchor,
        body: &str,
    ) -> Result<()> {
        use crate::diffpos::Side;
        let (path, line) = (at.path.as_str(), at.line);
        let pr: PullDto = self
            .api
            .get(&format!("{}/pulls/{number}", Self::repo_path(repo)))
            .await?;
        let sha = pr
            .head
            .and_then(|h| h.sha)
            .ok_or(PlatformError::Unsupported(
                "this pull request has no head commit",
            ))?;
        if self.is_github() {
            self.api
                .post_json(
                    &format!("{}/pulls/{number}/comments", Self::repo_path(repo)),
                    &json!({ "body": body, "commit_id": sha, "path": path, "line": line, "side": if at.side == Side::Old { "LEFT" } else { "RIGHT" } }),
                )
                .await
                .map(|_| ())
        } else {
            let position_key = if at.side == Side::Old {
                "old_position"
            } else {
                "new_position"
            };
            self.api
                .post_json(
                    &format!("{}/pulls/{number}/reviews", Self::repo_path(repo)),
                    &json!({
                        "event": "COMMENT",
                        "body": "",
                        "commit_id": sha,
                        "comments": [{
                            "path": path,
                            "body": body,
                            (position_key): line,
                        }],
                    }),
                )
                .await
                .map(|_| ())
        }
    }

    pub(crate) async fn notifications(&self) -> Result<Vec<Notification>> {
        let dtos: Vec<NotificationDto> = self.api.get("/notifications?per_page=50").await?;
        Ok(dtos.into_iter().map(|d| d.into_model(&self.host)).collect())
    }

    pub(crate) async fn mark_read(&self, id: &str) -> Result<()> {
        self.api
            .send(Method::Patch, &format!("/notifications/threads/{id}"), None)
            .await
            .map(|_| ())
    }

    pub(crate) async fn contributions(&self, user: &str) -> Result<Vec<ActivityDay>> {
        if self.is_github() {
            self.github_contributions(user).await
        } else {
            let points: Vec<HeatPoint> = self.api.get(&format!("/users/{user}/heatmap")).await?;
            Ok(fold_days(points.into_iter().filter_map(|p| {
                let date = DateTime::from_timestamp(p.timestamp, 0)?.date_naive();
                Some((date, p.contributions))
            })))
        }
    }

    /// What `user` did on `date`: GitHub's own breakdown through GraphQL, Gitea's activity feed otherwise.
    pub(crate) async fn day_activity(&self, user: &str, date: NaiveDate) -> Result<DayActivity> {
        if self.is_github() {
            self.github_day(user, date).await
        } else {
            self.gitea_day(user, date).await
        }
    }

    async fn github_day(&self, user: &str, date: NaiveDate) -> Result<DayActivity> {
        if self.api.token.is_none() {
            return Err(PlatformError::Auth(
                "GitHub's daily activity needs a token".into(),
            ));
        }
        let query = "query($u:String!,$from:DateTime!,$to:DateTime!){user(login:$u){contributionsCollection(from:$from,to:$to){\
             contributionCalendar{totalContributions}\
             commitContributionsByRepository(maxRepositories:25){repository{nameWithOwner url} contributions{totalCount}}\
             pullRequestContributions(first:50){nodes{pullRequest{NODE}}}\
             issueContributions(first:50){nodes{issue{NODE}}}\
             pullRequestReviewContributions(first:50){nodes{pullRequest{NODE}}}\
             }}}"
            .replace("NODE", "number title url repository{nameWithOwner}");
        let v = self
            .graphql(
                &query,
                &json!({
                    "u": user,
                    "from": format!("{date}T00:00:00Z"),
                    "to": format!("{date}T23:59:59Z"),
                }),
            )
            .await?;
        let c = &v["data"]["user"]["contributionsCollection"];
        if c.is_null() {
            return Err(PlatformError::NotFound(format!("user {user}")));
        }
        let mut entries = Vec::new();
        for r in c["commitContributionsByRepository"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let count = r["contributions"]["totalCount"].as_u64().unwrap_or(0) as u32;
            if count > 0 {
                entries.push(ActivityEntry {
                    kind: ActivityKind::Commits,
                    repo: r["repository"]["nameWithOwner"]
                        .as_str()
                        .unwrap_or_default()
                        .into(),
                    title: format!("{count} commit{}", if count == 1 { "" } else { "s" }),
                    number: None,
                    count,
                    url: r["repository"]["url"].as_str().unwrap_or_default().into(),
                });
            }
        }
        for (field, key, kind) in [
            (
                "pullRequestContributions",
                "pullRequest",
                ActivityKind::PullRequest,
            ),
            ("issueContributions", "issue", ActivityKind::Issue),
            (
                "pullRequestReviewContributions",
                "pullRequest",
                ActivityKind::Review,
            ),
        ] {
            for n in c[field]["nodes"].as_array().into_iter().flatten() {
                let i = &n[key];
                entries.push(ActivityEntry {
                    kind,
                    repo: i["repository"]["nameWithOwner"]
                        .as_str()
                        .unwrap_or_default()
                        .into(),
                    title: i["title"].as_str().unwrap_or_default().into(),
                    number: i["number"].as_u64(),
                    count: 1,
                    url: i["url"].as_str().unwrap_or_default().into(),
                });
            }
        }
        Ok(DayActivity {
            date,
            total: c["contributionCalendar"]["totalContributions"]
                .as_u64()
                .unwrap_or(0) as u32,
            entries,
        })
    }

    async fn gitea_day(&self, user: &str, date: NaiveDate) -> Result<DayActivity> {
        let feed: Vec<GiteaActivity> = self
            .api
            .get(&format!(
                "/users/{user}/activities/feeds?only-performed-by=true&date={date}&limit=50"
            ))
            .await?;
        let entries: Vec<ActivityEntry> =
            feed.into_iter().map(|a| a.into_entry(&self.host)).collect();
        let total = entries.iter().map(|e| e.count).sum();
        Ok(DayActivity {
            date,
            total,
            entries,
        })
    }

    async fn github_contributions(&self, user: &str) -> Result<Vec<ActivityDay>> {
        if self.api.token.is_none() {
            return Err(PlatformError::Auth(
                "the GitHub contribution calendar needs a token".into(),
            ));
        }
        let url = if self.host == "github.com" {
            "https://api.github.com/graphql".to_string()
        } else {
            format!("https://{}/api/graphql", self.host)
        };
        let query = "query($u:String!){user(login:$u){contributionsCollection{contributionCalendar{weeks{contributionDays{date contributionCount}}}}}}";
        let res = self
            .api
            .post_json(&url, &json!({ "query": query, "variables": { "u": user } }))
            .await?;
        let value: GraphQl = parse(&res)?;
        let user = value
            .data
            .and_then(|d| d.user)
            .ok_or_else(|| PlatformError::NotFound(format!("user {user}")))?;
        Ok(fold_days(
            user.contributions_collection
                .contribution_calendar
                .weeks
                .into_iter()
                .flat_map(|w| w.contribution_days)
                .filter_map(|d| Some((d.date.parse::<NaiveDate>().ok()?, d.contribution_count))),
        ))
    }
}

fn push_param(path: &mut String, name: &str, value: Option<&str>) {
    if let Some(v) = value {
        path.push('&');
        path.push_str(name);
        path.push('=');
        path.push_str(v);
    }
}

/// Sums counts per day and sorts by date.
pub(crate) fn fold_days(days: impl Iterator<Item = (NaiveDate, u32)>) -> Vec<ActivityDay> {
    let mut map = std::collections::BTreeMap::new();
    for (date, count) in days {
        *map.entry(date).or_insert(0u32) += count;
    }
    map.into_iter()
        .map(|(date, count)| ActivityDay { date, count })
        .collect()
}

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
struct UserDto {
    #[serde(default)]
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

impl From<UserDto> for User {
    fn from(d: UserDto) -> Self {
        User {
            login: d.login,
            avatar_url: d.avatar_url.filter(|u| !u.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct LabelDto {
    #[serde(default)]
    name: String,
    #[serde(default)]
    color: String,
}

fn labels(v: Vec<LabelDto>) -> Vec<Label> {
    v.into_iter()
        .map(|l| Label {
            name: l.name,
            color: l.color.trim_start_matches('#').to_string(),
        })
        .collect()
}

#[derive(Debug, Deserialize, Default)]
struct BranchDto {
    #[serde(rename = "ref", default)]
    name: String,
    #[serde(default)]
    sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PullDto {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    user: Option<UserDto>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    draft: Option<bool>,
    #[serde(default)]
    merged: Option<bool>,
    #[serde(default)]
    merged_at: Option<String>,
    #[serde(default)]
    head: Option<BranchDto>,
    #[serde(default)]
    base: Option<BranchDto>,
    #[serde(default)]
    labels: Vec<LabelDto>,
    #[serde(default)]
    assignees: Vec<UserDto>,
    #[serde(default)]
    requested_reviewers: Vec<UserDto>,
    #[serde(default)]
    comments: Option<u32>,
    #[serde(default)]
    additions: Option<u32>,
    #[serde(default)]
    deletions: Option<u32>,
    #[serde(default)]
    mergeable: Option<bool>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    html_url: String,
}

impl PullDto {
    fn into_model(self, repo: &RepoId) -> PullRequest {
        let merged = self.merged.unwrap_or(false) || self.merged_at.is_some();
        let state = if merged {
            ItemState::Merged
        } else if self.state == "open" {
            ItemState::Open
        } else {
            ItemState::Closed
        };
        let upper = self.title.to_uppercase();
        let draft =
            self.draft.unwrap_or(false) || upper.starts_with("WIP:") || upper.starts_with("[WIP]");
        PullRequest {
            repo: repo.clone(),
            number: self.number,
            title: self.title,
            body: self.body.unwrap_or_default(),
            author: self.user.map(Into::into).unwrap_or_default(),
            state,
            draft,
            source_branch: self.head.map(|h| h.name).unwrap_or_default(),
            target_branch: self.base.map(|b| b.name).unwrap_or_default(),
            labels: labels(self.labels),
            assignees: self.assignees.into_iter().map(Into::into).collect(),
            reviewers: self
                .requested_reviewers
                .into_iter()
                .map(Into::into)
                .collect(),
            comments: self.comments.unwrap_or(0),
            additions: self.additions,
            deletions: self.deletions,
            mergeable: self.mergeable,
            checks: None,
            created_at: self.created_at,
            updated_at: self.updated_at,
            url: self.html_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<IssueDto>,
}

#[derive(Debug, Deserialize)]
struct CombinedStatus {
    #[serde(default)]
    state: String,
}

#[derive(Debug, Deserialize, Default)]
struct MilestoneDto {
    #[serde(default)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct IssueDto {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    user: Option<UserDto>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    labels: Vec<LabelDto>,
    #[serde(default)]
    assignees: Option<Vec<UserDto>>,
    #[serde(default)]
    comments: Option<u32>,
    #[serde(default)]
    milestone: Option<MilestoneDto>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
    /// GitHub search results name the repo by API URL.
    #[serde(default)]
    repository_url: Option<String>,
    /// Gitea search results embed the repo.
    #[serde(default)]
    repository: Option<NotificationRepo>,
    #[serde(default)]
    draft: Option<bool>,
}

impl IssueDto {
    /// `owner/name` of the repo a search hit belongs to.
    fn repo_full_name(&self) -> Option<String> {
        if let Some(r) = &self.repository
            && !r.full_name.is_empty()
        {
            return Some(r.full_name.clone());
        }
        let url = self.repository_url.as_deref()?;
        let tail = url.split("/repos/").nth(1)?;
        Some(tail.trim_end_matches('/').to_string())
    }

    /// A search hit that is a pull request, as a [`PullRequest`] (branches are not in search results).
    fn into_search_pr(self, host: &str) -> Option<PullRequest> {
        let pr = self.pull_request.clone()?;
        let full = self.repo_full_name()?;
        let (owner, name) = full.rsplit_once('/')?;
        let repo = RepoId::new(host, owner, name);
        let merged = pr.get("merged_at").is_some_and(|v| !v.is_null())
            || pr.get("merged").and_then(|v| v.as_bool()).unwrap_or(false);
        let draft = self.draft.unwrap_or(false);
        let mut model = self.into_model(&repo);
        if merged {
            model.state = ItemState::Merged;
        }
        Some(PullRequest {
            repo,
            number: model.number,
            title: model.title,
            body: model.body,
            author: model.author,
            state: model.state,
            draft,
            labels: model.labels,
            assignees: model.assignees,
            comments: model.comments,
            created_at: model.created_at,
            updated_at: model.updated_at,
            url: model.url,
            ..PullRequest::default()
        })
    }

    /// A search hit that is an issue.
    fn into_search_issue(self, host: &str) -> Option<Issue> {
        if self.pull_request.is_some() {
            return None;
        }
        let full = self.repo_full_name()?;
        let (owner, name) = full.rsplit_once('/')?;
        Some(self.into_model(&RepoId::new(host, owner, name)))
    }

    fn into_model(self, repo: &RepoId) -> Issue {
        Issue {
            repo: repo.clone(),
            number: self.number,
            title: self.title,
            body: self.body.unwrap_or_default(),
            author: self.user.map(Into::into).unwrap_or_default(),
            state: if self.state == "open" {
                ItemState::Open
            } else {
                ItemState::Closed
            },
            labels: labels(self.labels),
            assignees: self
                .assignees
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            comments: self.comments.unwrap_or(0),
            milestone: self.milestone.map(|m| m.title).filter(|t| !t.is_empty()),
            created_at: self.created_at,
            updated_at: self.updated_at,
            url: self.html_url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CommentDto {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    user: Option<UserDto>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

impl From<CommentDto> for Comment {
    fn from(d: CommentDto) -> Self {
        Comment {
            id: d.id,
            author: d.user.map(Into::into).unwrap_or_default(),
            body: d.body,
            created_at: d.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReviewDto {
    #[serde(default)]
    user: Option<UserDto>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    submitted_at: Option<DateTime<Utc>>,
}

impl From<ReviewDto> for Review {
    fn from(d: ReviewDto) -> Self {
        let verdict = match d.state.to_uppercase().as_str() {
            "APPROVED" => Some(ReviewVerdict::Approve),
            "CHANGES_REQUESTED" | "REQUEST_CHANGES" => Some(ReviewVerdict::RequestChanges),
            "COMMENTED" | "COMMENT" => Some(ReviewVerdict::Comment),
            _ => None,
        };
        Review {
            author: d.user.map(Into::into).unwrap_or_default(),
            verdict,
            body: d.body,
            submitted_at: d.submitted_at,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NotificationSubject {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct NotificationRepo {
    #[serde(default)]
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct NotificationDto {
    #[serde(default)]
    id: serde_json::Value,
    #[serde(default)]
    unread: bool,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    updated_at: Option<DateTime<Utc>>,
    subject: NotificationSubject,
    #[serde(default)]
    repository: Option<NotificationRepo>,
}

impl NotificationDto {
    fn into_model(self, host: &str) -> Notification {
        let kind = match self.subject.kind.as_str() {
            "PullRequest" | "Pull" => SubjectKind::PullRequest,
            "Issue" => SubjectKind::Issue,
            "Commit" => SubjectKind::Commit,
            "Release" => SubjectKind::Release,
            _ => SubjectKind::Other,
        };
        let reason = match self.reason.as_deref() {
            Some("review_requested") => NotificationReason::ReviewRequested,
            Some("mention" | "team_mention") => NotificationReason::Mention,
            Some("assign") => NotificationReason::Assigned,
            Some("comment" | "author") => NotificationReason::Comment,
            Some("ci_activity") => NotificationReason::CiActivity,
            Some("subscribed") => NotificationReason::Subscribed,
            _ => NotificationReason::Other,
        };
        let number = self
            .subject
            .url
            .as_deref()
            .and_then(|u| u.trim_end_matches('/').rsplit('/').next())
            .and_then(|n| n.parse().ok());
        let full = self
            .repository
            .map(|r| r.full_name)
            .filter(|n| !n.is_empty());
        let repo = full.as_deref().and_then(|f| {
            let (owner, name) = f.rsplit_once('/')?;
            Some(RepoId::new(host, owner, name))
        });
        let url = self
            .subject
            .html_url
            .clone()
            .unwrap_or_else(|| match (&repo, number) {
                (Some(r), Some(n)) => {
                    let seg = if kind == SubjectKind::PullRequest {
                        "pull"
                    } else {
                        "issues"
                    };
                    format!("https://{host}/{}/{seg}/{n}", r.full_name())
                }
                (Some(r), None) => format!("https://{host}/{}", r.full_name()),
                _ => format!("https://{host}"),
            });
        let id = match self.id {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        Notification {
            id,
            repo,
            kind,
            number,
            title: self.subject.title,
            reason,
            unread: self.unread,
            updated_at: self.updated_at,
            url,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GiteaActivity {
    #[serde(default)]
    op_type: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    ref_name: String,
    #[serde(default)]
    repo: Option<GiteaActivityRepo>,
}

#[derive(Debug, Deserialize)]
struct GiteaActivityRepo {
    #[serde(default)]
    full_name: String,
}

impl GiteaActivity {
    fn into_entry(self, host: &str) -> ActivityEntry {
        use ActivityKind::*;
        let repo = self.repo.map(|r| r.full_name).unwrap_or_default();
        // issue and PR activities carry "number|title"
        let (number, title) = match self.content.split_once('|') {
            Some((n, t)) => (n.trim().parse::<u64>().ok(), t.to_string()),
            None => (None, self.content.clone()),
        };
        let (kind, count, title) = match self.op_type.as_str() {
            "commit_repo" | "mirror_sync_push" => {
                let len = serde_json::from_str::<serde_json::Value>(&self.content)
                    .ok()
                    .and_then(|v| v["Len"].as_u64())
                    .unwrap_or(1) as u32;
                let branch = self.ref_name.trim_start_matches("refs/heads/");
                (
                    Commits,
                    len,
                    format!(
                        "{len} commit{} to {branch}",
                        if len == 1 { "" } else { "s" }
                    ),
                )
            }
            "create_issue" | "close_issue" | "reopen_issue" => (Issue, 1, title),
            "create_pull_request"
            | "merge_pull_request"
            | "close_pull_request"
            | "reopen_pull_request"
            | "auto_merge_pull_request" => (PullRequest, 1, title),
            "approve_pull_request" | "reject_pull_request" => (Review, 1, title),
            "comment_issue" | "comment_pull" => (Comment, 1, title),
            other => (Other, 1, other.replace('_', " ")),
        };
        let path = match (kind, number) {
            (PullRequest | Review, Some(n)) => format!("/pulls/{n}"),
            (Issue | Comment, Some(n)) => format!("/issues/{n}"),
            _ => String::new(),
        };
        ActivityEntry {
            kind,
            url: if repo.is_empty() {
                String::new()
            } else {
                format!("https://{host}/{repo}{path}")
            },
            repo,
            title,
            number,
            count,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HeatPoint {
    timestamp: i64,
    contributions: u32,
}

#[derive(Debug, Deserialize)]
struct GraphQl {
    data: Option<GraphData>,
}
#[derive(Debug, Deserialize)]
struct GraphData {
    user: Option<GraphUser>,
}
#[derive(Debug, Deserialize)]
struct GraphUser {
    #[serde(rename = "contributionsCollection")]
    contributions_collection: GraphCollection,
}
#[derive(Debug, Deserialize)]
struct GraphCollection {
    #[serde(rename = "contributionCalendar")]
    contribution_calendar: GraphCalendar,
}
#[derive(Debug, Deserialize)]
struct GraphCalendar {
    weeks: Vec<GraphWeek>,
}
#[derive(Debug, Deserialize)]
struct GraphWeek {
    #[serde(rename = "contributionDays")]
    contribution_days: Vec<GraphDay>,
}
#[derive(Debug, Deserialize)]
struct GraphDay {
    date: String,
    #[serde(rename = "contributionCount")]
    contribution_count: u32,
}

/// The GraphQL endpoint for a REST base: `api.github.com` has `/graphql`, Enterprise swaps `/api/v3` for `/api/graphql`.
fn graphql_url(base: &str) -> String {
    match base.strip_suffix("/api/v3") {
        Some(root) => format!("{root}/api/graphql"),
        None => format!("{base}/graphql"),
    }
}

/// Explains why a forced merge can still be refused.
fn force_hint(e: PlatformError) -> PlatformError {
    const HINT: &str = " (forcing needs admin rights and a branch rule that lets admins bypass)";
    match e {
        PlatformError::Http { status, message } => PlatformError::Http {
            status,
            message: format!("{message}{HINT}"),
        },
        PlatformError::Forbidden(m) => PlatformError::Forbidden(format!("{m}{HINT}")),
        other => other,
    }
}
