//! One interface over every platform. [`Backend`] is what the front-ends use.

use crate::account::Account;
use crate::api::Api;
use crate::error::{PlatformError, Result};
use crate::github_like::GitHubLike;
use crate::gitlab::{GitLab, ItemKind};
use crate::http::HttpClient;
use crate::model::{
    ActivityDay, Board, Capabilities, Comment, DayActivity, Discussion, DiscussionDetail, Issue,
    ListQuery, MergeMethod, MergeOptions, MergeOutcome, Notification, PullRequest, Review,
    ReviewVerdict, StateFilter, UpdateStyle, User, WikiContent, WikiPage,
};
use crate::platform::{PlatformKind, RepoId};
use gitkraft_settings::dashboard::Involvement;

/// The adapter for one [`Account`].
#[derive(Debug)]
pub enum Backend<C> {
    /// GitHub, GitHub Enterprise, Gitea, Forgejo, Codeberg.
    GitHubLike(GitHubLike<C>),
    /// GitLab.
    GitLab(GitLab<C>),
}

impl<C: HttpClient> Backend<C> {
    /// Builds the adapter for `account` on top of `client`.
    pub fn new(account: &Account, client: C) -> Self {
        let api = Api::new(
            client,
            account.kind,
            account.api_base(),
            account.token.clone(),
        );
        match account.kind {
            PlatformKind::GitLab => Self::GitLab(GitLab::new(api, account.host.clone())),
            PlatformKind::GitHub | PlatformKind::Gitea => {
                Self::GitHubLike(GitHubLike::new(api, account.host.clone()))
            }
        }
    }

    fn api(&self) -> &Api<C> {
        match self {
            Self::GitHubLike(b) => &b.api,
            Self::GitLab(b) => &b.api,
        }
    }

    /// The platform kind of this account.
    pub fn kind(&self) -> PlatformKind {
        self.api().kind
    }

    /// SSH keys registered on the account.
    pub async fn ssh_keys(&self) -> Result<Vec<crate::model::SshKey>> {
        crate::ssh::remote::list(self.api()).await
    }

    /// Registers a public key (`<type> <base64>`) on the account.
    pub async fn add_ssh_key(&self, title: &str, key: &str) -> Result<crate::model::SshKey> {
        crate::ssh::remote::add(self.api(), title, key).await
    }

    /// Removes an SSH key from the account.
    pub async fn delete_ssh_key(&self, id: &str) -> Result<()> {
        crate::ssh::remote::delete(self.api(), id).await
    }

    /// What this platform supports.
    pub fn capabilities(&self) -> Capabilities {
        match self {
            Self::GitHubLike(b) => b.capabilities(),
            Self::GitLab(b) => b.capabilities(),
        }
    }

    /// The authenticated user.
    pub async fn current_user(&self) -> Result<User> {
        match self {
            Self::GitHubLike(b) => b.current_user().await,
            Self::GitLab(b) => b.current_user().await,
        }
    }

    /// Lists pull requests.
    pub async fn list_pull_requests(
        &self,
        repo: &RepoId,
        q: &ListQuery,
    ) -> Result<Vec<PullRequest>> {
        match self {
            Self::GitHubLike(b) => b.list_pull_requests(repo, q).await,
            Self::GitLab(b) => b.list_pull_requests(repo, q).await,
        }
    }

    /// Pull requests across every repo you can see, by relation to you. `me` is your login.
    pub async fn search_pull_requests(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<PullRequest>> {
        match self {
            Self::GitHubLike(b) => b.search_pull_requests(who, state, me).await,
            Self::GitLab(b) => b.search_pull_requests(who, state, me).await,
        }
    }

    /// Issues across every repo you can see, by relation to you.
    pub async fn search_issues(
        &self,
        who: Involvement,
        state: StateFilter,
        me: &str,
    ) -> Result<Vec<Issue>> {
        match self {
            Self::GitHubLike(b) => b.search_issues(who, state, me).await,
            Self::GitLab(b) => b.search_issues(who, state, me).await,
        }
    }

    /// One pull request, with its check state.
    pub async fn pull_request(&self, repo: &RepoId, number: u64) -> Result<PullRequest> {
        match self {
            Self::GitHubLike(b) => b.pull_request(repo, number).await,
            Self::GitLab(b) => b.pull_request(repo, number).await,
        }
    }

    /// The unified diff of a pull request.
    pub async fn pull_request_diff(&self, repo: &RepoId, number: u64) -> Result<String> {
        match self {
            Self::GitHubLike(b) => b.pull_request_diff(repo, number).await,
            Self::GitLab(b) => b.pull_request_diff(repo, number).await,
        }
    }

    /// Lists issues (pull requests excluded).
    pub async fn list_issues(&self, repo: &RepoId, q: &ListQuery) -> Result<Vec<Issue>> {
        match self {
            Self::GitHubLike(b) => b.list_issues(repo, q).await,
            Self::GitLab(b) => b.list_issues(repo, q).await,
        }
    }

    /// One issue.
    pub async fn issue(&self, repo: &RepoId, number: u64) -> Result<Issue> {
        match self {
            Self::GitHubLike(b) => b.issue(repo, number).await,
            Self::GitLab(b) => b.issue(repo, number).await,
        }
    }

    /// Comments on a PR or issue, oldest first.
    pub async fn comments(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
    ) -> Result<Vec<Comment>> {
        match self {
            Self::GitHubLike(b) => b.comments(repo, number).await,
            Self::GitLab(b) => b.comments(repo, kind, number).await,
        }
    }

    /// Reviews on a pull request.
    pub async fn reviews(&self, repo: &RepoId, number: u64) -> Result<Vec<Review>> {
        match self {
            Self::GitHubLike(b) => b.reviews(repo, number).await,
            Self::GitLab(b) => b.reviews(repo, number).await,
        }
    }

    /// Posts a comment.
    pub async fn post_comment(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
        body: &str,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.post_comment(repo, number, body).await,
            Self::GitLab(b) => b.post_comment(repo, kind, number, body).await,
        }
    }

    /// The labels the project has.
    pub async fn list_labels(&self, repo: &RepoId) -> Result<Vec<String>> {
        match self {
            Self::GitHubLike(b) => b.list_labels(repo).await,
            Self::GitLab(b) => b.list_labels(repo).await,
        }
    }

    /// Adds and removes labels (by name) on a PR or issue.
    pub async fn edit_labels(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
        add: &[String],
        remove: &[String],
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.edit_labels(repo, number, add, remove).await,
            Self::GitLab(b) => b.edit_labels(repo, kind, number, add, remove).await,
        }
    }

    /// Closes (`open == false`) or reopens a PR or issue.
    pub async fn set_open(
        &self,
        repo: &RepoId,
        kind: ItemKind,
        number: u64,
        open: bool,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.set_open(repo, number, open).await,
            Self::GitLab(b) => b.set_open(repo, kind, number, open).await,
        }
    }

    /// Merges a pull request right now.
    pub async fn merge(&self, repo: &RepoId, number: u64, method: MergeMethod) -> Result<()> {
        self.merge_with(repo, number, &method.into())
            .await
            .map(|_| ())
    }

    /// Merges, schedules (auto-merge) or queues a pull request, optionally forcing past branch rules.
    pub async fn merge_with(
        &self,
        repo: &RepoId,
        number: u64,
        options: &MergeOptions,
    ) -> Result<MergeOutcome> {
        if let Some(problem) = options.problem() {
            return Err(PlatformError::Unsupported(problem));
        }
        match self {
            Self::GitHubLike(b) => b.merge_with(repo, number, options).await,
            Self::GitLab(b) => b.merge_with(repo, number, options).await,
        }
    }

    /// Brings the pull request's branch up to date with its target.
    pub async fn update_branch(
        &self,
        repo: &RepoId,
        number: u64,
        style: UpdateStyle,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.update_branch(repo, number, style).await,
            Self::GitLab(b) => b.update_branch(repo, number, style).await,
        }
    }

    /// Cancels a scheduled auto-merge or takes the pull request out of the merge queue.
    pub async fn cancel_auto_merge(&self, repo: &RepoId, number: u64) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.cancel_auto_merge(repo, number).await,
            Self::GitLab(b) => b.cancel_auto_merge(repo, number).await,
        }
    }

    /// Submits a review.
    pub async fn submit_review(
        &self,
        repo: &RepoId,
        number: u64,
        verdict: ReviewVerdict,
        body: &str,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.submit_review(repo, number, verdict, body).await,
            Self::GitLab(b) => b.submit_review(repo, number, verdict, body).await,
        }
    }

    /// Comments on one line of a file in a pull request (a removed line is on the old side).
    pub async fn comment_on_line(
        &self,
        repo: &RepoId,
        number: u64,
        at: &crate::diffpos::Anchor,
        body: &str,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.comment_on_line(repo, number, at, body).await,
            Self::GitLab(b) => b.comment_on_line(repo, number, at, body).await,
        }
    }

    /// The notification inbox.
    pub async fn notifications(&self) -> Result<Vec<Notification>> {
        match self {
            Self::GitHubLike(b) => b.notifications().await,
            Self::GitLab(b) => b.notifications().await,
        }
    }

    /// Marks one notification read / done.
    pub async fn mark_read(&self, id: &str) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.mark_read(id).await,
            Self::GitLab(b) => b.mark_read(id).await,
        }
    }

    /// Daily contribution counts for `user`, oldest first.
    /// What `user` did on `date`, for the panel under the contribution calendar.
    pub async fn day_activity(&self, user: &str, date: chrono::NaiveDate) -> Result<DayActivity> {
        match self {
            Self::GitHubLike(b) => b.day_activity(user, date).await,
            Self::GitLab(b) => b.day_activity(user, date).await,
        }
    }

    /// The repository wiki's pages (GitLab, Gitea family).
    pub async fn wiki_pages(&self, repo: &RepoId) -> Result<Vec<WikiPage>> {
        match self {
            Self::GitHubLike(b) => b.wiki_pages(repo).await,
            Self::GitLab(b) => b.wiki_pages(repo).await,
        }
    }

    /// One wiki page with its content.
    pub async fn wiki_page(&self, repo: &RepoId, slug: &str) -> Result<WikiContent> {
        match self {
            Self::GitHubLike(b) => b.wiki_page(repo, slug).await,
            Self::GitLab(b) => b.wiki_page(repo, slug).await,
        }
    }

    /// Latest discussions of a repository (GitHub only).
    pub async fn discussions(&self, repo: &RepoId, first: u32) -> Result<Vec<Discussion>> {
        match self {
            Self::GitHubLike(b) => b.discussions(repo, first).await,
            Self::GitLab(_) => Err(PlatformError::Unsupported(
                "discussions exist on GitHub only",
            )),
        }
    }

    /// One discussion with its comments (GitHub only).
    pub async fn discussion(&self, repo: &RepoId, number: u64) -> Result<DiscussionDetail> {
        match self {
            Self::GitHubLike(b) => b.discussion(repo, number).await,
            Self::GitLab(_) => Err(PlatformError::Unsupported(
                "discussions exist on GitHub only",
            )),
        }
    }

    /// Comments on a discussion (GitHub only).
    pub async fn comment_on_discussion(
        &self,
        repo: &RepoId,
        number: u64,
        body: &str,
    ) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.comment_on_discussion(repo, number, body).await,
            Self::GitLab(_) => Err(PlatformError::Unsupported(
                "discussions exist on GitHub only",
            )),
        }
    }

    /// The kanban boards of a repository: GitHub Projects (v2), GitLab issue boards.
    pub async fn boards(&self, repo: &RepoId) -> Result<Vec<Board>> {
        match self {
            Self::GitHubLike(b) => b.boards(repo).await,
            Self::GitLab(b) => b.boards(repo).await,
        }
    }

    /// Contributions per day over the last year.
    pub async fn contributions(&self, user: &str) -> Result<Vec<ActivityDay>> {
        match self {
            Self::GitHubLike(b) => b.contributions(user).await,
            Self::GitLab(b) => b.contributions(user).await,
        }
    }
}
