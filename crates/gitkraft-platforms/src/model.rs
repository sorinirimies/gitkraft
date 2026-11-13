//! Normalized types: the UI never sees a GitHub, GitLab or Gitea type.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::RepoId;

/// A person on a platform.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct User {
    /// Login / username.
    pub login: String,
    /// Avatar image URL, when the platform gives one.
    pub avatar_url: Option<String>,
}

/// A label with its hex colour (no leading `#`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Label {
    /// Label name.
    pub name: String,
    /// Hex colour, e.g. `d73a4a`.
    pub color: String,
}

/// Open / closed / merged, the same on every platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ItemState {
    /// Open.
    #[default]
    Open,
    /// Closed without merging.
    Closed,
    /// Merged (pull requests only).
    Merged,
}

/// Which items a list call returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateFilter {
    /// Open items.
    #[default]
    Open,
    /// Closed (and merged) items.
    Closed,
    /// Everything.
    All,
}

/// Result of the combined checks on a PR head, when the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChecksState {
    /// Still running or queued.
    Pending,
    /// All passed.
    Success,
    /// At least one failed.
    Failure,
}

/// A pull request (GitLab: merge request).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PullRequest {
    /// Repository it belongs to.
    pub repo: RepoId,
    /// Number (GitLab: `iid`).
    pub number: u64,
    /// Title.
    pub title: String,
    /// Markdown body.
    pub body: String,
    /// Author.
    pub author: User,
    /// State.
    pub state: ItemState,
    /// Draft / work in progress.
    pub draft: bool,
    /// Branch the change comes from.
    pub source_branch: String,
    /// Branch it merges into.
    pub target_branch: String,
    /// Labels.
    pub labels: Vec<Label>,
    /// Assignees.
    pub assignees: Vec<User>,
    /// Requested reviewers.
    pub reviewers: Vec<User>,
    /// Comment count.
    pub comments: u32,
    /// Added lines, when known.
    pub additions: Option<u32>,
    /// Removed lines, when known.
    pub deletions: Option<u32>,
    /// `Some(false)` when the PR has conflicts.
    pub mergeable: Option<bool>,
    /// Combined check state, when known.
    pub checks: Option<ChecksState>,
    /// Creation time.
    pub created_at: Option<DateTime<Utc>>,
    /// Last update time.
    pub updated_at: Option<DateTime<Utc>>,
    /// Web URL (for "open in browser" and copy link).
    pub url: String,
}

/// An issue.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Issue {
    /// Repository it belongs to.
    pub repo: RepoId,
    /// Number (GitLab: `iid`).
    pub number: u64,
    /// Title.
    pub title: String,
    /// Markdown body.
    pub body: String,
    /// Author.
    pub author: User,
    /// Open or closed.
    pub state: ItemState,
    /// Labels.
    pub labels: Vec<Label>,
    /// Assignees.
    pub assignees: Vec<User>,
    /// Comment count.
    pub comments: u32,
    /// Milestone title.
    pub milestone: Option<String>,
    /// Creation time.
    pub created_at: Option<DateTime<Utc>>,
    /// Last update time.
    pub updated_at: Option<DateTime<Utc>>,
    /// Web URL.
    pub url: String,
}

/// A comment on a PR or issue.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Comment {
    /// Platform id.
    pub id: u64,
    /// Author.
    pub author: User,
    /// Markdown body.
    pub body: String,
    /// Creation time.
    pub created_at: Option<DateTime<Utc>>,
}

/// What a reviewer decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewVerdict {
    /// Approve.
    Approve,
    /// Request changes.
    RequestChanges,
    /// Comment only.
    Comment,
}

/// A submitted review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// Reviewer.
    pub author: User,
    /// Decision (`None` for pending or dismissed reviews).
    pub verdict: Option<ReviewVerdict>,
    /// Review body.
    pub body: String,
    /// Submission time.
    pub submitted_at: Option<DateTime<Utc>>,
}

/// How to merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeMethod {
    /// Merge commit.
    #[default]
    Merge,
    /// Squash.
    Squash,
    /// Rebase.
    Rebase,
}

/// What a notification is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubjectKind {
    /// A pull request.
    PullRequest,
    /// An issue.
    Issue,
    /// A commit.
    Commit,
    /// A release.
    Release,
    /// Anything else (CI run, discussion…).
    Other,
}

/// Why you were notified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationReason {
    /// Your review was requested.
    ReviewRequested,
    /// You were mentioned.
    Mention,
    /// You were assigned.
    Assigned,
    /// Someone commented on something you take part in.
    Comment,
    /// A CI run failed.
    CiActivity,
    /// You watch the repo.
    Subscribed,
    /// Anything else.
    Other,
}

/// One inbox entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    /// Platform id (opaque).
    pub id: String,
    /// Repository, when known.
    pub repo: Option<RepoId>,
    /// Subject kind.
    pub kind: SubjectKind,
    /// Subject number (PR / issue), when known.
    pub number: Option<u64>,
    /// Subject title.
    pub title: String,
    /// Why you got it.
    pub reason: NotificationReason,
    /// Not read yet.
    pub unread: bool,
    /// Last update.
    pub updated_at: Option<DateTime<Utc>>,
    /// Web URL.
    pub url: String,
}

/// Contributions on one day, for the calendar heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityDay {
    /// The day (UTC).
    pub date: NaiveDate,
    /// Contribution count.
    pub count: u32,
}

/// Paging for list calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// 1-based page.
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
        }
    }
}

/// What the platform offers; the UI hides controls that are `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    /// Squash / rebase merge methods.
    pub merge_methods: bool,
    /// Submit approve / request-changes reviews.
    pub reviews: bool,
    /// Notification inbox.
    pub notifications: bool,
    /// Contribution calendar.
    pub contributions: bool,
    /// Combined check state on PRs.
    pub checks: bool,
}

/// A filter for PR and issue lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListQuery {
    /// State filter.
    pub state: StateFilter,
    /// Only items by this author.
    pub author: Option<String>,
    /// Only items assigned to this user.
    pub assignee: Option<String>,
    /// Only items with this label.
    pub label: Option<String>,
    /// Paging.
    pub page: Page,
}
