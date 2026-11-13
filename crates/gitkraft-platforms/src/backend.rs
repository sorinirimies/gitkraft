//! One interface over every platform. [`Backend`] is what the front-ends use.

use crate::account::Account;
use crate::api::Api;
use crate::error::Result;
use crate::github_like::GitHubLike;
use crate::gitlab::{GitLab, ItemKind};
use crate::http::HttpClient;
use crate::model::{
    ActivityDay, Capabilities, Comment, Issue, ListQuery, MergeMethod, Notification, PullRequest,
    Review, ReviewVerdict, StateFilter, User,
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
    pub async fn search_pull_requests(&self, who: Involvement, state: StateFilter, me: &str) -> Result<Vec<PullRequest>> {
        match self {
            Self::GitHubLike(b) => b.search_pull_requests(who, state, me).await,
            Self::GitLab(b) => b.search_pull_requests(who, state, me).await,
        }
    }

    /// Issues across every repo you can see, by relation to you.
    pub async fn search_issues(&self, who: Involvement, state: StateFilter, me: &str) -> Result<Vec<Issue>> {
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

    /// Merges a pull request.
    pub async fn merge(&self, repo: &RepoId, number: u64, method: MergeMethod) -> Result<()> {
        match self {
            Self::GitHubLike(b) => b.merge(repo, number, method).await,
            Self::GitLab(b) => b.merge(repo, number, method).await,
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
    pub async fn contributions(&self, user: &str) -> Result<Vec<ActivityDay>> {
        match self {
            Self::GitHubLike(b) => b.contributions(user).await,
            Self::GitLab(b) => b.contributions(user).await,
        }
    }
}
