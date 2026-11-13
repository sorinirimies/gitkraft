use std::collections::{BTreeMap, BTreeSet};

use gitkraft_settings::notify::NotifyEvent;
use serde::{Deserialize, Serialize};

use super::alert::Alert;
use crate::model::{Issue, ItemState, Notification, NotificationReason, PullRequest};
use crate::platform::RepoId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Seen {
    state: ItemState,
    title: String,
    url: String,
    author: String,
    labels: Vec<String>,
    branch: String,
    mine: bool,
    involved: bool,
}

/// What a repo looked like at the last poll.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RepoSnapshot {
    prs: BTreeMap<u64, Seen>,
    issues: BTreeMap<u64, Seen>,
}

/// Builds a snapshot from fresh lists; `me` is your login on this host.
pub fn snapshot(prs: &[PullRequest], issues: &[Issue], me: &str) -> RepoSnapshot {
    let is_me = |l: &str| !me.is_empty() && l.eq_ignore_ascii_case(me);
    RepoSnapshot {
        prs: prs
            .iter()
            .map(|p| {
                let mine = is_me(&p.author.login) || p.assignees.iter().any(|u| is_me(&u.login));
                (
                    p.number,
                    Seen {
                        state: p.state,
                        title: p.title.clone(),
                        url: p.url.clone(),
                        author: p.author.login.clone(),
                        labels: p.labels.iter().map(|l| l.name.clone()).collect(),
                        branch: p.target_branch.clone(),
                        mine,
                        involved: p.reviewers.iter().any(|u| is_me(&u.login)),
                    },
                )
            })
            .collect(),
        issues: issues
            .iter()
            .map(|i| {
                let mine = is_me(&i.author.login) || i.assignees.iter().any(|u| is_me(&u.login));
                (
                    i.number,
                    Seen {
                        state: i.state,
                        title: i.title.clone(),
                        url: i.url.clone(),
                        author: i.author.login.clone(),
                        labels: i.labels.iter().map(|l| l.name.clone()).collect(),
                        branch: String::new(),
                        mine,
                        involved: false,
                    },
                )
            })
            .collect(),
    }
}

fn alert(repo: &RepoId, event: NotifyEvent, number: u64, s: &Seen) -> Alert {
    Alert {
        repo: repo.clone(),
        event,
        number: Some(number),
        title: s.title.clone(),
        url: s.url.clone(),
        branch: (!s.branch.is_empty()).then(|| s.branch.clone()),
        labels: s.labels.clone(),
        author: Some(s.author.clone()),
        mine: s.mine,
        involved: s.involved,
    }
}

/// Alerts for what changed between two snapshots. The first poll of a repo
/// (`prev == None`) only seeds the snapshot and raises nothing.
pub fn diff_snapshots(
    repo: &RepoId,
    prev: Option<&RepoSnapshot>,
    cur: &RepoSnapshot,
) -> Vec<Alert> {
    let Some(prev) = prev else { return Vec::new() };
    let mut out = Vec::new();
    for (n, now) in &cur.prs {
        match prev.prs.get(n) {
            None if now.state == ItemState::Open => {
                out.push(alert(repo, NotifyEvent::PrOpened, *n, now))
            }
            Some(before) if before.state == ItemState::Open => match now.state {
                ItemState::Merged => out.push(alert(repo, NotifyEvent::PrMerged, *n, now)),
                ItemState::Closed => out.push(alert(repo, NotifyEvent::PrClosed, *n, now)),
                ItemState::Open => {}
            },
            _ => {}
        }
    }
    for (n, now) in &cur.issues {
        if !prev.issues.contains_key(n) && now.state == ItemState::Open {
            out.push(alert(repo, NotifyEvent::IssueOpened, *n, now));
        }
    }
    out
}

/// Alerts for unread inbox entries not seen before. `seen` is updated with
/// every id in `current`. An empty `seen` seeds without raising alerts.
pub fn inbox_alerts(
    seen: &mut BTreeSet<String>,
    current: &[Notification],
    first_poll: bool,
) -> Vec<Alert> {
    let mut out = Vec::new();
    for n in current {
        let is_new = seen.insert(n.id.clone());
        if first_poll || !is_new || !n.unread {
            continue;
        }
        let (Some(repo), event) = (
            n.repo.clone(),
            match n.reason {
                NotificationReason::ReviewRequested => Some(NotifyEvent::ReviewRequested),
                NotificationReason::Mention => Some(NotifyEvent::Mention),
                NotificationReason::Comment => Some(NotifyEvent::Comment),
                NotificationReason::CiActivity => Some(NotifyEvent::CiFailed),
                _ => None,
            },
        ) else {
            continue;
        };
        let Some(event) = event else { continue };
        out.push(Alert {
            repo,
            event,
            number: n.number,
            title: n.title.clone(),
            url: n.url.clone(),
            branch: None,
            labels: Vec::new(),
            author: None,
            mine: false,
            involved: true,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Label, User};

    fn repo() -> RepoId {
        RepoId::new("github.com", "o", "r")
    }

    fn pr(n: u64, state: ItemState, author: &str) -> PullRequest {
        PullRequest {
            number: n,
            title: format!("pr {n}"),
            state,
            author: User {
                login: author.into(),
                avatar_url: None,
            },
            target_branch: "master".into(),
            labels: vec![Label {
                name: "bug".into(),
                color: String::new(),
            }],
            ..PullRequest::default()
        }
    }

    fn issue(n: u64, state: ItemState) -> Issue {
        Issue {
            number: n,
            title: format!("issue {n}"),
            state,
            ..Issue::default()
        }
    }

    #[test]
    fn first_poll_seeds_without_alerts() {
        let cur = snapshot(
            &[pr(1, ItemState::Open, "a")],
            &[issue(2, ItemState::Open)],
            "me",
        );
        assert!(diff_snapshots(&repo(), None, &cur).is_empty());
    }

    #[test]
    fn new_open_items_alert_closed_ones_do_not() {
        let prev = snapshot(&[pr(1, ItemState::Open, "a")], &[], "me");
        let cur = snapshot(
            &[
                pr(1, ItemState::Open, "a"),
                pr(2, ItemState::Open, "b"),
                pr(3, ItemState::Closed, "c"),
            ],
            &[issue(9, ItemState::Open), issue(8, ItemState::Closed)],
            "me",
        );
        let alerts = diff_snapshots(&repo(), Some(&prev), &cur);
        let kinds: Vec<_> = alerts.iter().map(|a| (a.event, a.number)).collect();
        assert_eq!(
            kinds,
            vec![
                (NotifyEvent::PrOpened, Some(2)),
                (NotifyEvent::IssueOpened, Some(9))
            ]
        );
        assert_eq!(alerts[0].labels, ["bug"]);
        assert_eq!(alerts[0].branch.as_deref(), Some("master"));
    }

    #[test]
    fn state_transitions_of_known_prs() {
        let prev = snapshot(
            &[
                pr(1, ItemState::Open, "a"),
                pr(2, ItemState::Open, "a"),
                pr(3, ItemState::Closed, "a"),
            ],
            &[],
            "me",
        );
        let cur = snapshot(
            &[
                pr(1, ItemState::Merged, "a"),
                pr(2, ItemState::Closed, "a"),
                pr(3, ItemState::Open, "a"),
            ],
            &[],
            "me",
        );
        let kinds: Vec<_> = diff_snapshots(&repo(), Some(&prev), &cur)
            .into_iter()
            .map(|a| a.event)
            .collect();
        assert_eq!(kinds, vec![NotifyEvent::PrMerged, NotifyEvent::PrClosed]);
    }

    #[test]
    fn mine_and_involved_come_from_me() {
        let mut p = pr(1, ItemState::Open, "Me");
        p.reviewers = vec![User {
            login: "me".into(),
            avatar_url: None,
        }];
        let s = snapshot(&[p, pr(2, ItemState::Open, "x")], &[], "me");
        let cur = diff_snapshots(&repo(), Some(&RepoSnapshot::default()), &s);
        assert!(cur[0].mine && cur[0].involved);
        assert!(!cur[1].mine && !cur[1].involved);
        let anon = snapshot(&[pr(1, ItemState::Open, "")], &[], "");
        assert!(
            !anon.prs[&1].mine,
            "empty login never matches an empty `me`"
        );
    }

    fn note(id: &str, reason: NotificationReason, unread: bool) -> Notification {
        Notification {
            id: id.into(),
            repo: Some(repo()),
            kind: crate::model::SubjectKind::PullRequest,
            number: Some(7),
            title: "t".into(),
            reason,
            unread,
            updated_at: None,
            url: "u".into(),
        }
    }

    #[test]
    fn inbox_alerts_only_for_new_unread_known_reasons() {
        let mut seen = BTreeSet::new();
        let first = [note("1", NotificationReason::Mention, true)];
        assert!(inbox_alerts(&mut seen, &first, true).is_empty(), "seed");
        let next = [
            note("1", NotificationReason::Mention, true),
            note("2", NotificationReason::ReviewRequested, true),
            note("3", NotificationReason::Subscribed, true),
            note("4", NotificationReason::Mention, false),
            note("5", NotificationReason::CiActivity, true),
        ];
        let alerts = inbox_alerts(&mut seen, &next, false);
        assert_eq!(
            alerts.iter().map(|a| a.event).collect::<Vec<_>>(),
            vec![NotifyEvent::ReviewRequested, NotifyEvent::CiFailed]
        );
        assert_eq!(seen.len(), 5);
        assert!(
            inbox_alerts(&mut seen, &next, false).is_empty(),
            "no repeats"
        );
    }
}
