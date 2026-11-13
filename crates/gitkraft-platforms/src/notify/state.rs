use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::diff::RepoSnapshot;

/// What the poller remembers between runs, so restarts do not repeat or spam.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotifyState {
    /// Last snapshot per `host/owner/name`.
    pub snapshots: BTreeMap<String, RepoSnapshot>,
    /// Inbox ids already seen per host.
    pub seen_notifications: BTreeMap<String, BTreeSet<String>>,
    /// Snooze per project key (or `*` for everything).
    pub muted_until: BTreeMap<String, DateTime<Utc>>,
}

impl NotifyState {
    /// Parses saved state; garbage gives an empty state (the next poll re-seeds).
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// Serializes for saving.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Snooze in effect for `repo_key`: the later of the project's and the global one.
    pub fn muted_until(&self, repo_key: &str) -> Option<DateTime<Utc>> {
        self.muted_until
            .get(repo_key)
            .into_iter()
            .chain(self.muted_until.get("*"))
            .max()
            .copied()
    }

    /// Snoozes `repo_key` (`None`: everything) until `until`.
    pub fn mute(&mut self, repo_key: Option<&str>, until: DateTime<Utc>) {
        self.muted_until
            .insert(repo_key.unwrap_or("*").to_string(), until);
    }

    /// Ends a snooze.
    pub fn unmute(&mut self, repo_key: Option<&str>) {
        self.muted_until.remove(repo_key.unwrap_or("*"));
    }
}

/// Per-project poll result, for the bell.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RepoPollStatus {
    /// Last good poll.
    pub last_poll: Option<DateTime<Utc>>,
    /// Error of the last attempt, cleared by the next good poll.
    pub last_error: Option<String>,
}

/// Written by whatever runs the poller; read by the UIs to draw the bell.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PollerStatus {
    /// Last sign of life of any poller (daemon or in-app).
    pub heartbeat: Option<DateTime<Utc>>,
    /// `daemon` or `in-app`, shown in the status bar.
    pub mode: String,
    /// Status per project key.
    pub repos: BTreeMap<String, RepoPollStatus>,
}

impl PollerStatus {
    /// Records a successful poll.
    pub fn ok(&mut self, repo_key: &str, now: DateTime<Utc>) {
        self.heartbeat = Some(now);
        self.repos.insert(
            repo_key.to_string(),
            RepoPollStatus {
                last_poll: Some(now),
                last_error: None,
            },
        );
    }

    /// Records a failed poll, keeping the last good time.
    pub fn failed(&mut self, repo_key: &str, reason: String, now: DateTime<Utc>) {
        self.heartbeat = Some(now);
        self.repos
            .entry(repo_key.to_string())
            .or_default()
            .last_error = Some(reason);
    }

    /// Parses a saved status file.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// Serializes for saving.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Whether a project is due for its next poll.
pub fn is_due(last_attempt: Option<DateTime<Utc>>, interval: Duration, now: DateTime<Utc>) -> bool {
    match last_attempt {
        None => true,
        Some(t) => chrono::Duration::from_std(interval).is_ok_and(|i| now - t >= i),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + s, 0).expect("ts")
    }

    #[test]
    fn due_when_never_polled_or_interval_passed() {
        let i = Duration::from_secs(60);
        assert!(is_due(None, i, at(0)));
        assert!(!is_due(Some(at(0)), i, at(59)));
        assert!(is_due(Some(at(0)), i, at(60)));
    }

    #[test]
    fn mute_takes_the_later_of_project_and_global() {
        let mut s = NotifyState::default();
        assert_eq!(s.muted_until("h/o/r"), None);
        s.mute(Some("h/o/r"), at(100));
        s.mute(None, at(500));
        assert_eq!(s.muted_until("h/o/r"), Some(at(500)));
        assert_eq!(s.muted_until("h/o/x"), Some(at(500)));
        s.unmute(None);
        assert_eq!(s.muted_until("h/o/r"), Some(at(100)));
        assert_eq!(s.muted_until("h/o/x"), None);
    }

    #[test]
    fn failed_poll_keeps_last_good_time_and_ok_clears_error() {
        let mut p = PollerStatus::default();
        p.ok("a", at(0));
        p.failed("a", "rate limited".into(), at(60));
        let r = &p.repos["a"];
        assert_eq!(
            (r.last_poll, r.last_error.as_deref()),
            (Some(at(0)), Some("rate limited"))
        );
        assert_eq!(p.heartbeat, Some(at(60)));
        p.ok("a", at(120));
        assert_eq!(p.repos["a"].last_error, None);
        p.failed("new", "x".into(), at(130));
        assert_eq!(p.repos["new"].last_poll, None);
    }

    #[test]
    fn json_round_trip_and_garbage() {
        let mut s = NotifyState::default();
        s.mute(None, at(1));
        s.seen_notifications
            .entry("h".into())
            .or_default()
            .insert("1".into());
        assert_eq!(NotifyState::from_json(&s.to_json()), s);
        assert_eq!(NotifyState::from_json("{"), NotifyState::default());
        assert_eq!(PollerStatus::from_json("x"), PollerStatus::default());
    }
}
