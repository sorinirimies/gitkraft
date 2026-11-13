//! The hub: one backend per account, and the calls the front-ends make on them.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use chrono::NaiveDate;
use futures::future::join_all;
use gitkraft_ci::{CiBackend, Run, RunQuery};
use gitkraft_platforms::model::{
    ActivityDay, Comment, Issue, ItemState, ListQuery, Notification, PullRequest, Review,
    StateFilter,
};
use gitkraft_platforms::{Account, Backend, HttpClient, PlatformError, RepoId};
use gitkraft_settings::dashboard::{Involvement, Section, SectionKind, SectionState};

/// Everything one section shows, plus the accounts that failed (a section can be partly filled).
#[derive(Debug, Clone, PartialEq)]
pub struct SectionResult<T> {
    /// Items, newest activity first.
    pub items: Vec<T>,
    /// One line per failed account or repo.
    pub errors: Vec<String>,
}

impl<T> Default for SectionResult<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// The data of one dashboard section.
#[derive(Debug, Clone, PartialEq)]
pub enum SectionData {
    /// Pull requests.
    Prs(SectionResult<PullRequest>),
    /// Issues.
    Issues(SectionResult<Issue>),
    /// CI runs.
    Runs(SectionResult<Run>),
    /// Notifications.
    Inbox(SectionResult<Notification>),
}

impl SectionData {
    /// Number of items.
    pub fn len(&self) -> usize {
        match self {
            Self::Prs(r) => r.items.len(),
            Self::Issues(r) => r.items.len(),
            Self::Runs(r) => r.items.len(),
            Self::Inbox(r) => r.items.len(),
        }
    }

    /// No items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Errors collected while loading.
    pub fn errors(&self) -> &[String] {
        match self {
            Self::Prs(r) => &r.errors,
            Self::Issues(r) => &r.errors,
            Self::Runs(r) => &r.errors,
            Self::Inbox(r) => &r.errors,
        }
    }
}

/// A pull request with everything the detail view shows.
#[derive(Debug, Clone, PartialEq)]
pub struct PrDetail {
    /// The PR.
    pub pr: PullRequest,
    /// Comments, oldest first.
    pub comments: Vec<Comment>,
    /// Reviews.
    pub reviews: Vec<Review>,
    /// Unified diff; empty when it could not be fetched.
    pub diff: String,
    /// Why parts are missing.
    pub errors: Vec<String>,
}

struct Entry<C> {
    account: Account,
    backend: Backend<C>,
    ci: CiBackend<C>,
    me: Mutex<Option<String>>,
}

/// One backend per account.
pub struct Hub<C> {
    entries: Vec<Entry<C>>,
}

impl<C> std::fmt::Debug for Hub<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hosts: Vec<_> = self
            .entries
            .iter()
            .map(|e| e.account.host.as_str())
            .collect();
        f.debug_struct("Hub").field("accounts", &hosts).finish()
    }
}

fn state_filter(s: SectionState) -> StateFilter {
    match s {
        SectionState::Open => StateFilter::Open,
        SectionState::Closed => StateFilter::Closed,
        SectionState::All => StateFilter::All,
    }
}

fn describe(e: &PlatformError, what: &str) -> String {
    format!("{what}: {e}")
}

impl<C: HttpClient + Clone> Hub<C> {
    /// Builds a backend per account, all sharing `client`.
    pub fn new(accounts: Vec<Account>, client: &C) -> Self {
        let entries = accounts
            .into_iter()
            .map(|account| Entry {
                backend: Backend::new(&account, client.clone()),
                ci: CiBackend::new(&account, client.clone()),
                me: Mutex::new(account.me.clone()),
                account,
            })
            .collect();
        Self { entries }
    }

    /// Hosts of the accounts, in order.
    pub fn hosts(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|e| e.account.host.as_str())
            .collect()
    }

    /// No accounts.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn entry(&self, host: &str) -> Option<&Entry<C>> {
        self.entries
            .iter()
            .find(|e| e.account.host.eq_ignore_ascii_case(host))
    }

    /// The backend for `host`.
    pub fn backend(&self, host: &str) -> Option<&Backend<C>> {
        self.entry(host).map(|e| &e.backend)
    }

    /// The CI backend for `host`.
    pub fn ci(&self, host: &str) -> Option<&CiBackend<C>> {
        self.entry(host).map(|e| &e.ci)
    }

    /// Your login on an account, asking the platform the first time.
    pub async fn me(&self, host: &str) -> Result<String, PlatformError> {
        let entry = self
            .entry(host)
            .ok_or_else(|| PlatformError::NotFound(format!("no account for {host}")))?;
        if let Some(me) = entry.me.lock().ok().and_then(|m| m.clone()) {
            return Ok(me);
        }
        let user = entry.backend.current_user().await?;
        if let Ok(mut m) = entry.me.lock() {
            *m = Some(user.login.clone());
        }
        Ok(user.login)
    }

    /// Loads one section.
    pub async fn load_section(&self, section: &Section) -> SectionData {
        match section.kind {
            SectionKind::Prs => SectionData::Prs(self.load_prs(section).await),
            SectionKind::Issues => SectionData::Issues(self.load_issues(section).await),
            SectionKind::Ci => SectionData::Runs(self.load_runs(section).await),
            SectionKind::Inbox => SectionData::Inbox(self.load_inbox().await),
        }
    }

    async fn load_prs(&self, s: &Section) -> SectionResult<PullRequest> {
        let state = state_filter(s.state);
        let jobs = if s.repos.is_empty() {
            let who = s.involvement.unwrap_or(Involvement::Authored);
            let calls = self.entries.iter().map(|e| async move {
                let host = e.account.host.clone();
                let me = self.me(&host).await.map_err(|err| describe(&err, &host))?;
                e.backend
                    .search_pull_requests(who, state, &me)
                    .await
                    .map_err(|err| describe(&err, &host))
            });
            join_all(calls).await
        } else {
            let calls =
                s.repos
                    .iter()
                    .filter_map(|key| RepoId::from_key(key))
                    .map(|repo| async move {
                        let backend = self
                            .backend(&repo.host)
                            .ok_or_else(|| format!("{}: no account", repo.host))?;
                        let q = ListQuery {
                            state,
                            label: s.label.clone(),
                            ..ListQuery::default()
                        };
                        let mut prs = backend
                            .list_pull_requests(&repo, &q)
                            .await
                            .map_err(|err| describe(&err, &repo.key()))?;
                        if let Some(who) = s.involvement {
                            let me = self
                                .me(&repo.host)
                                .await
                                .map_err(|err| describe(&err, &repo.host))?;
                            prs.retain(|p| pr_matches(p, who, &me));
                        }
                        Ok(prs)
                    });
            join_all(calls).await
        };
        collect(jobs, |p: &PullRequest| p.updated_at)
    }

    async fn load_issues(&self, s: &Section) -> SectionResult<Issue> {
        let state = state_filter(s.state);
        let jobs = if s.repos.is_empty() {
            let who = s.involvement.unwrap_or(Involvement::Assigned);
            let calls = self.entries.iter().map(|e| async move {
                let host = e.account.host.clone();
                let me = self.me(&host).await.map_err(|err| describe(&err, &host))?;
                e.backend
                    .search_issues(who, state, &me)
                    .await
                    .map_err(|err| describe(&err, &host))
            });
            join_all(calls).await
        } else {
            let calls =
                s.repos
                    .iter()
                    .filter_map(|key| RepoId::from_key(key))
                    .map(|repo| async move {
                        let backend = self
                            .backend(&repo.host)
                            .ok_or_else(|| format!("{}: no account", repo.host))?;
                        let q = ListQuery {
                            state,
                            label: s.label.clone(),
                            ..ListQuery::default()
                        };
                        let mut issues = backend
                            .list_issues(&repo, &q)
                            .await
                            .map_err(|err| describe(&err, &repo.key()))?;
                        if let Some(who) = s.involvement {
                            let me = self
                                .me(&repo.host)
                                .await
                                .map_err(|err| describe(&err, &repo.host))?;
                            issues.retain(|i| match who {
                                Involvement::Authored => i.author.login.eq_ignore_ascii_case(&me),
                                Involvement::Assigned => i
                                    .assignees
                                    .iter()
                                    .any(|u| u.login.eq_ignore_ascii_case(&me)),
                                Involvement::ReviewRequested => false,
                            });
                        }
                        Ok(issues)
                    });
            join_all(calls).await
        };
        collect(jobs, |i: &Issue| i.updated_at)
    }

    async fn load_runs(&self, s: &Section) -> SectionResult<Run> {
        let calls = s
            .repos
            .iter()
            .filter_map(|key| RepoId::from_key(key))
            .map(|repo| async move {
                let ci = self
                    .ci(&repo.host)
                    .ok_or_else(|| format!("{}: no account", repo.host))?;
                let q = RunQuery {
                    branch: s.branch.clone(),
                    ..RunQuery::default()
                };
                ci.list_runs(&repo, &q)
                    .await
                    .map_err(|err| describe(&err, &repo.key()))
            });
        collect(join_all(calls).await, |r: &Run| r.created_at)
    }

    async fn load_inbox(&self) -> SectionResult<Notification> {
        let calls = self.entries.iter().map(|e| async move {
            e.backend
                .notifications()
                .await
                .map_err(|err| describe(&err, &e.account.host))
        });
        collect(join_all(calls).await, |n: &Notification| n.updated_at)
    }

    /// A PR with comments, reviews and diff. Missing parts become `errors`, not a failure.
    pub async fn pr_detail(&self, repo: &RepoId, number: u64) -> Result<PrDetail, PlatformError> {
        let backend = self
            .backend(&repo.host)
            .ok_or_else(|| PlatformError::NotFound(format!("no account for {}", repo.host)))?;
        let pr = backend.pull_request(repo, number).await?;
        let (comments, reviews, diff) = futures::join!(
            backend.comments(repo, gitkraft_platforms::ItemKind::PullRequest, number),
            backend.reviews(repo, number),
            backend.pull_request_diff(repo, number),
        );
        let errors: Vec<String> = [
            comments.as_ref().err().map(|e| describe(e, "comments")),
            reviews.as_ref().err().map(|e| describe(e, "reviews")),
            diff.as_ref().err().map(|e| describe(e, "diff")),
        ]
        .into_iter()
        .flatten()
        .collect();
        Ok(PrDetail {
            pr,
            comments: comments.unwrap_or_default(),
            reviews: reviews.unwrap_or_default(),
            diff: diff.unwrap_or_default(),
            errors,
        })
    }

    /// Daily contributions summed over every account, oldest first, plus per-account errors.
    pub async fn contributions(&self) -> SectionResult<ActivityDay> {
        let calls = self.entries.iter().map(|e| async move {
            let host = e.account.host.clone();
            let me = self.me(&host).await.map_err(|err| describe(&err, &host))?;
            e.backend
                .contributions(&me)
                .await
                .map_err(|err| describe(&err, &host))
        });
        let mut sum: BTreeMap<NaiveDate, u32> = BTreeMap::new();
        let mut errors = Vec::new();
        for r in join_all(calls).await {
            match r {
                Ok(days) => days
                    .into_iter()
                    .for_each(|d| *sum.entry(d.date).or_default() += d.count),
                Err(e) => errors.push(e),
            }
        }
        SectionResult {
            items: sum
                .into_iter()
                .map(|(date, count)| ActivityDay { date, count })
                .collect(),
            errors,
        }
    }
}

fn pr_matches(p: &PullRequest, who: Involvement, me: &str) -> bool {
    let is_me = |l: &str| l.eq_ignore_ascii_case(me);
    match who {
        Involvement::Authored => is_me(&p.author.login),
        Involvement::Assigned => p.assignees.iter().any(|u| is_me(&u.login)),
        Involvement::ReviewRequested => p.reviewers.iter().any(|u| is_me(&u.login)),
    }
}

/// Merges per-source results, newest first by `key` (items without a time go last).
fn collect<T, K: Ord>(
    results: Vec<Result<Vec<T>, String>>,
    key: impl Fn(&T) -> Option<K>,
) -> SectionResult<T> {
    let mut out = SectionResult::default();
    for r in results {
        match r {
            Ok(items) => out.items.extend(items),
            Err(e) => out.errors.push(e),
        }
    }
    out.items.sort_by(|a, b| key(b).cmp(&key(a)));
    out
}

/// Whether an item is still open, for counting badges.
pub fn is_open(state: ItemState) -> bool {
    state == ItemState::Open
}

/// Shares one client between several hubs.
pub fn shared<C>(client: C) -> Arc<C> {
    Arc::new(client)
}
