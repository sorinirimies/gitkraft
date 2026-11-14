//! The tools that only read.

use gitkraft_ci::RunQuery;
use gitkraft_hub::{SectionData, args};
use gitkraft_platforms::model::{ListQuery, Page, StateFilter};
use gitkraft_settings::dashboard::SectionKind;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{schemars, tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use crate::server::{GitkraftServer, ToolResult, failed, invalid, kind_of, out, slim, truncate};

/// A repository.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct RepoArgs {
    /// `host/owner/name`, or `owner/name`. Default: the checkout the server runs in.
    pub repo: Option<String>,
}

/// Items by relation to you.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct MineArgs {
    /// `authored` (default), `assigned` or `review_requested`.
    pub involvement: Option<String>,
    /// `open` (default), `closed` or `all`.
    pub state: Option<String>,
}

/// List pull requests or issues of a repository.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct ListArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// `open` (default), `closed` or `all`.
    pub state: Option<String>,
    /// Only items by this login.
    pub author: Option<String>,
    /// Only items assigned to this login.
    pub assignee: Option<String>,
    /// Only items with this label.
    pub label: Option<String>,
    /// How many (default 30, at most 100).
    pub limit: Option<u32>,
}

/// One pull request or issue.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct ItemArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Number on the platform.
    pub number: u64,
    /// Pull requests only: include the unified diff (default false: only its size).
    pub include_diff: Option<bool>,
    /// Diff characters to return when included (default 40000).
    pub max_diff_chars: Option<usize>,
}

/// Discussions of a repository.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct DiscussionsArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Discussion number: with it the discussion and its comments, without it the latest list.
    pub number: Option<u64>,
    /// How many to list (default 20, at most 100).
    pub limit: Option<u32>,
}

/// A wiki page, or the page list.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct WikiArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Page slug from the list: with it the page's Markdown, without it the page list.
    pub page: Option<String>,
    /// Characters of the page to return (default 40000).
    pub max_chars: Option<usize>,
}

/// CI runs of a repository.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct RunsArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Only this branch.
    pub branch: Option<String>,
    /// `queued`, `running`, `success`, `failed` or `cancelled`.
    pub status: Option<String>,
    /// How many (default 20, at most 100).
    pub limit: Option<u32>,
}

/// One CI run.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct RunArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Run id (from `ci_runs`).
    pub run_id: u64,
}

/// The log of one CI job.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct LogArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Job id (from `ci_run`).
    pub job_id: u64,
    /// Only the last N lines (default 150).
    pub tail_lines: Option<usize>,
    /// Only lines containing this text (case-insensitive), with their line numbers.
    pub grep: Option<String>,
}

/// Activity summary.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub(crate) struct PulseArgs {
    /// `host/owner/name`; default: the current checkout.
    pub repo: Option<String>,
    /// Days to look back (default 7).
    pub days: Option<u32>,
}

/// One day of the contribution calendar.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct DayArgs {
    /// `YYYY-MM-DD` (UTC), within the last year. Default: today.
    pub date: Option<String>,
}

/// Words from a tool call, as the shared rules read them; a mistake is the caller's.
fn bad<T>(r: Result<T, String>) -> Result<T, rmcp::ErrorData> {
    r.map_err(invalid)
}

fn clamp(limit: Option<u32>, default: u32) -> u32 {
    limit.unwrap_or(default).clamp(1, 100)
}

#[tool_router(router = read_router, vis = "pub(crate)")]
impl GitkraftServer {
    #[tool(
        description = "The connected accounts: host, platform, your login, what the platform supports, and whether this server may write.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn accounts(&self) -> ToolResult {
        let mut accounts = Vec::new();
        for host in self.hub.hosts() {
            let me = self.hub.me(host).await.map_err(|e| e.to_string());
            let caps = self.hub.backend(host).map(|b| b.capabilities());
            accounts.push(json!({
                "host": host,
                "kind": self.hub.backend(host).map(|b| format!("{:?}", b.kind())),
                "me": me.as_ref().ok(),
                "error": me.as_ref().err(),
                "reviews": caps.map(|c| c.reviews),
                "merge_methods": caps.map(|c| c.merge_methods),
                "notifications": caps.map(|c| c.notifications),
                "checks": caps.map(|c| c.checks),
                "ci": self.hub.ci(host).map(|c| c.system_name()),
            }));
        }
        out(&json!({
            "accounts": accounts,
            "default_repo": self.default_repo.as_ref().map(|r| r.key()),
            "write": self.policy.write,
            "allow_force": self.policy.allow_force,
        }))
    }

    #[tool(
        description = "Pull requests / merge requests across all accounts that relate to you: opened by you (default), assigned to you, or waiting for your review.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn my_pull_requests(&self, Parameters(a): Parameters<MineArgs>) -> ToolResult {
        let st = bad(args::state_filter(a.state.as_deref()))?;
        let who = bad(args::involvement(a.involvement.as_deref()))?;
        match self
            .hub
            .load_section(&args::section("mcp", SectionKind::Prs, Some(who), st))
            .await
        {
            SectionData::Prs(r) => {
                out(&json!({"items": slim(json!(r.items), false), "errors": r.errors}))
            }
            _ => Err(failed("my_pull_requests", "unexpected section data")),
        }
    }

    #[tool(
        description = "Issues across all accounts that relate to you: opened by you (default), assigned to you.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn my_issues(&self, Parameters(a): Parameters<MineArgs>) -> ToolResult {
        let st = bad(args::state_filter(a.state.as_deref()))?;
        let who = bad(args::involvement(a.involvement.as_deref()))?;
        match self
            .hub
            .load_section(&args::section("mcp", SectionKind::Issues, Some(who), st))
            .await
        {
            SectionData::Issues(r) => {
                out(&json!({"items": slim(json!(r.items), false), "errors": r.errors}))
            }
            _ => Err(failed("my_issues", "unexpected section data")),
        }
    }

    #[tool(
        description = "The notification inbox of every account: mentions, review requests, CI failures.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn inbox(&self) -> ToolResult {
        match self
            .hub
            .load_section(&args::section(
                "mcp",
                SectionKind::Inbox,
                None,
                StateFilter::Open,
            ))
            .await
        {
            SectionData::Inbox(r) => out(&json!({"items": r.items, "errors": r.errors})),
            _ => Err(failed("inbox", "unexpected section data")),
        }
    }

    #[tool(
        description = "Pull requests of a repository, newest first, without their descriptions. Filter by state, author, assignee, label.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn list_pull_requests(
        &self,
        Parameters(a): Parameters<ListArgs>,
    ) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let q = ListQuery {
            state: bad(args::state_filter(a.state.as_deref()))?,
            author: a.author,
            assignee: a.assignee,
            label: a.label,
            page: Page {
                page: 1,
                per_page: clamp(a.limit, 30),
            },
        };
        let prs = self
            .backend(&repo)?
            .list_pull_requests(&repo, &q)
            .await
            .map_err(|e| failed("list_pull_requests", e))?;
        out(&json!({"repo": repo.key(), "items": slim(json!(prs), false)}))
    }

    #[tool(
        description = "One pull request in full: description, state, checks, mergeability, reviews and comments. The diff is only included with include_diff (and cut at max_diff_chars).",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn get_pull_request(&self, Parameters(a): Parameters<ItemArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        self.backend(&repo)?;
        let d = self
            .hub
            .pr_detail(&repo, a.number)
            .await
            .map_err(|e| failed("get_pull_request", e))?;
        let max = a.max_diff_chars.unwrap_or(40_000);
        let (diff, cut) = truncate(&d.diff, max);
        out(&slim(
            json!({
                "pull_request": d.pr,
                "reviews": d.reviews,
                "comments": d.comments,
                "diff_chars": d.diff.chars().count(),
                "diff": a.include_diff.unwrap_or(false).then_some(diff),
                "diff_truncated": a.include_diff.unwrap_or(false) && cut,
                "errors": d.errors,
            }),
            true,
        ))
    }

    #[tool(
        description = "Issues of a repository, newest first, without their descriptions. Filter by state, author, assignee, label.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn list_issues(&self, Parameters(a): Parameters<ListArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let q = ListQuery {
            state: bad(args::state_filter(a.state.as_deref()))?,
            author: a.author,
            assignee: a.assignee,
            label: a.label,
            page: Page {
                page: 1,
                per_page: clamp(a.limit, 30),
            },
        };
        let issues = self
            .backend(&repo)?
            .list_issues(&repo, &q)
            .await
            .map_err(|e| failed("list_issues", e))?;
        out(&json!({"repo": repo.key(), "items": slim(json!(issues), false)}))
    }

    #[tool(
        description = "One issue in full, with its comments.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn get_issue(&self, Parameters(a): Parameters<ItemArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let b = self.backend(&repo)?;
        let issue = b
            .issue(&repo, a.number)
            .await
            .map_err(|e| failed("get_issue", e))?;
        let comments = b
            .comments(&repo, kind_of("issue")?, a.number)
            .await
            .map_err(|e| failed("get_issue comments", e))?;
        out(&slim(json!({"issue": issue, "comments": comments}), true))
    }

    #[tool(
        description = "GitHub discussions: the latest ones, or with `number` one discussion with its comments. Other platforms have none.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn discussions(
        &self,
        Parameters(a): Parameters<DiscussionsArgs>,
    ) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let b = self.backend(&repo)?;
        match a.number {
            Some(n) => {
                let d = b
                    .discussion(&repo, n)
                    .await
                    .map_err(|e| failed("discussions", e))?;
                out(&slim(json!({"repo": repo.key(), "discussion": d}), true))
            }
            None => {
                let list = b
                    .discussions(&repo, clamp(a.limit, 20))
                    .await
                    .map_err(|e| failed("discussions", e))?;
                out(&slim(
                    json!({"repo": repo.key(), "discussions": list}),
                    false,
                ))
            }
        }
    }

    #[tool(
        description = "Kanban boards of a repository: GitHub Projects (v2, columns from the Status field) or GitLab issue boards (one column per list label). Gitea has no board API.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn boards(&self, Parameters(a): Parameters<RepoArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let boards = self
            .backend(&repo)?
            .boards(&repo)
            .await
            .map_err(|e| failed("boards", e))?;
        out(&json!({"repo": repo.key(), "boards": boards}))
    }

    #[tool(
        description = "A repository's wiki on GitLab, Gitea, Forgejo or Codeberg: the page list, or with `page` one page's Markdown. GitHub wikis are git repositories (<repo>.wiki.git) and not reachable here.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn wiki(&self, Parameters(a): Parameters<WikiArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let b = self.backend(&repo)?;
        match a.page {
            Some(slug) => {
                let p = b
                    .wiki_page(&repo, &slug)
                    .await
                    .map_err(|e| failed("wiki", e))?;
                let (text, cut) = truncate(&p.content, a.max_chars.unwrap_or(40_000));
                out(&json!({
                    "repo": repo.key(), "title": p.page.title, "slug": p.page.slug,
                    "content": text, "truncated": cut,
                }))
            }
            None => {
                let pages = b.wiki_pages(&repo).await.map_err(|e| failed("wiki", e))?;
                out(&json!({"repo": repo.key(), "pages": pages}))
            }
        }
    }

    #[tool(
        description = "The labels a repository defines.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn list_labels(&self, Parameters(a): Parameters<RepoArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let labels = self
            .backend(&repo)?
            .list_labels(&repo)
            .await
            .map_err(|e| failed("list_labels", e))?;
        out(&json!({"repo": repo.key(), "labels": labels}))
    }

    #[tool(
        description = "CI runs (GitHub / Gitea Actions, GitLab pipelines, Woodpecker), newest first. Filter by branch and status.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn ci_runs(&self, Parameters(a): Parameters<RunsArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let q = RunQuery {
            branch: a.branch,
            status: bad(a.status.as_deref().map(args::status).transpose())?,
            page: 1,
            per_page: clamp(a.limit, 20),
        };
        let runs = self
            .ci(&repo)?
            .list_runs(&repo, &q)
            .await
            .map_err(|e| failed("ci_runs", e))?;
        out(&json!({"repo": repo.key(), "runs": runs}))
    }

    #[tool(
        description = "One CI run with its jobs (ids, status, stage, runner). Use ci_job_log for the output of a job.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn ci_run(&self, Parameters(a): Parameters<RunArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let d = self
            .ci(&repo)?
            .run_detail(&repo, a.run_id)
            .await
            .map_err(|e| failed("ci_run", e))?;
        out(&json!({"run": d.run, "jobs": d.jobs}))
    }

    #[tool(
        description = "The log of a CI job, without colour codes: the last tail_lines lines (default 150), or only the lines containing `grep` (e.g. error).",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn ci_job_log(&self, Parameters(a): Parameters<LogArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let raw = self
            .ci(&repo)?
            .job_log(&repo, a.job_id)
            .await
            .map_err(|e| failed("ci_job_log", e))?;
        let x = args::log_excerpt(&raw, a.tail_lines.unwrap_or(150), a.grep.as_deref(), 200);
        out(&json!({"total_lines": x.total_lines, "lines": x.lines}))
    }

    #[tool(
        description = "The workflows of a repository that can be started by hand (GitHub, Gitea).",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn list_workflows(&self, Parameters(a): Parameters<RepoArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        let w = self
            .ci(&repo)?
            .list_workflows(&repo)
            .await
            .map_err(|e| failed("list_workflows", e))?;
        out(&json!({"repo": repo.key(), "workflows": w}))
    }

    #[tool(
        description = "What you did on one day over every account, like the panel under GitHub's contribution calendar: contributions, commits per repository, pull requests, issues, reviews, comments.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn activity_day(&self, Parameters(a): Parameters<DayArgs>) -> ToolResult {
        let today = chrono::Utc::now().date_naive();
        let date = match a.date.as_deref() {
            None => today,
            Some(t) => t
                .parse::<chrono::NaiveDate>()
                .map_err(|_| invalid("date must be YYYY-MM-DD"))?,
        };
        if date > today || (today - date).num_days() > 370 {
            return Err(invalid(
                "date must be within the last year and not in the future",
            ));
        }
        let r = self.hub.day_activity(date).await;
        let groups: Vec<_> = r
            .activity
            .groups()
            .into_iter()
            .map(|(kind, count, items)| json!({"heading": kind.heading(), "count": count, "items": items}))
            .collect();
        out(&json!({
            "headline": r.activity.headline(),
            "total": r.activity.total,
            "groups": groups,
            "errors": r.errors,
        }))
    }

    #[tool(
        description = "Activity of a repository over the last days: PRs opened, merged, closed, issues, green and red CI runs, most active authors.",
        annotations(read_only_hint = true)
    )]
    pub(crate) async fn pulse(&self, Parameters(a): Parameters<PulseArgs>) -> ToolResult {
        let repo = self.repo(a.repo.as_deref())?;
        self.backend(&repo)?;
        let p = self
            .hub
            .repo_pulse(&repo, a.days.unwrap_or(7).clamp(1, 90))
            .await;
        out(&json!({
            "repo": repo.key(),
            "days": p.days,
            "prs_opened": p.prs_opened,
            "prs_merged": p.prs_merged,
            "prs_closed": p.prs_closed,
            "prs_open_now": p.prs_open_now,
            "issues_opened": p.issues_opened,
            "issues_closed": p.issues_closed,
            "ci_green": p.ci_green,
            "ci_red": p.ci_red,
            "top_authors": p.top_authors,
        }))
    }
}
