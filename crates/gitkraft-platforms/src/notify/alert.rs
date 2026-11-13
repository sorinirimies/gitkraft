use gitkraft_settings::notify::{Candidate, NotifyEvent};
use serde::{Deserialize, Serialize};

use crate::platform::RepoId;

/// Something worth telling the user about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alert {
    /// Project it concerns.
    pub repo: RepoId,
    /// What happened.
    pub event: NotifyEvent,
    /// PR / issue number, when there is one.
    pub number: Option<u64>,
    /// Headline.
    pub title: String,
    /// Web URL, also used as the deep link target.
    pub url: String,
    /// Branch (CI events, PR target).
    pub branch: Option<String>,
    /// Labels on the item.
    pub labels: Vec<String>,
    /// Who caused it.
    pub author: Option<String>,
    /// The item is yours.
    pub mine: bool,
    /// You take part in it.
    pub involved: bool,
}

impl Alert {
    /// The filter input for this alert.
    pub fn candidate(&self) -> Candidate {
        Candidate {
            event: Some(self.event),
            branch: self.branch.clone(),
            paths: Vec::new(),
            labels: self.labels.clone(),
            author: self.author.clone(),
            mine: self.mine,
            involved: self.involved,
        }
    }

    /// Short text for a desktop notification title.
    pub fn headline(&self) -> String {
        let what = match self.event {
            NotifyEvent::ReviewRequested => "Review requested",
            NotifyEvent::Mention => "Mentioned",
            NotifyEvent::PrOpened => "New PR",
            NotifyEvent::PrMerged => "PR merged",
            NotifyEvent::PrClosed => "PR closed",
            NotifyEvent::IssueOpened => "New issue",
            NotifyEvent::Comment => "New comment",
            NotifyEvent::CiFailed => "CI failed",
            NotifyEvent::CiRecovered => "CI recovered",
            NotifyEvent::ReleasePublished => "New release",
        };
        format!("{what} · {}", self.repo.full_name())
    }

    /// Deep link that opens the item in GitKraft.
    pub fn deep_link(&self) -> String {
        let kind = match self.event {
            NotifyEvent::IssueOpened => "issue",
            NotifyEvent::CiFailed | NotifyEvent::CiRecovered => "ci",
            _ => "pr",
        };
        match self.number {
            Some(n) => format!("gitkraft://{}/{kind}/{n}", self.repo.key()),
            None => format!("gitkraft://{}", self.repo.key()),
        }
    }
}
