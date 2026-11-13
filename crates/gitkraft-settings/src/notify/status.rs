use std::time::Duration;

use chrono::{DateTime, Utc};

/// What the bell shows for one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyStatus {
    /// Off for this project.
    Off,
    /// On and polling; `last_poll` is the last good poll.
    On {
        /// Last successful poll, if any yet.
        last_poll: Option<DateTime<Utc>>,
    },
    /// Muted until the given time.
    Snoozed {
        /// When it turns back on.
        until: DateTime<Utc>,
    },
    /// On, but the last poll failed.
    Error {
        /// Short reason (auth, rate limit, network…).
        reason: String,
    },
    /// On, but nothing is polling (no daemon, no app running the poller).
    NoPoller,
}

impl NotifyStatus {
    /// Bell glyph; the text alternatives keep it readable without colour or emoji.
    pub fn glyph(&self, ascii: bool) -> &'static str {
        match (self, ascii) {
            (Self::Off, false) => "🔕",
            (Self::Off, true) => "[ ]",
            (Self::On { .. }, false) => "🔔",
            (Self::On { .. }, true) => "[*]",
            (Self::Snoozed { .. }, false) => "⏸",
            (Self::Snoozed { .. }, true) => "[z]",
            (Self::Error { .. }, false) => "⚠",
            (Self::Error { .. }, true) => "[!]",
            (Self::NoPoller, false) => "🔔?",
            (Self::NoPoller, true) => "[?]",
        }
    }

    /// One-line description for a tooltip or status bar.
    pub fn describe(&self, now: DateTime<Utc>) -> String {
        match self {
            Self::Off => "notifications off".into(),
            Self::On { last_poll: Some(t) } => {
                format!("on, last poll {}s ago", (now - *t).num_seconds().max(0))
            }
            Self::On { last_poll: None } => "on, waiting for first poll".into(),
            Self::Snoozed { until } => {
                let mins = (*until - now).num_minutes().max(0);
                format!("snoozed, {}h {:02}m left", mins / 60, mins % 60)
            }
            Self::Error { reason } => format!("polling failed: {reason}"),
            Self::NoPoller => {
                "on, but no poller is running (start the daemon or keep the app open)".into()
            }
        }
    }
}

/// Everything [`status`] needs; all of it comes from the config and the poller's status file.
#[derive(Debug, Clone, Default)]
pub struct StatusInput {
    /// Effective `enabled`.
    pub enabled: bool,
    /// Muted until, from the notify state.
    pub muted_until: Option<DateTime<Utc>>,
    /// Last successful poll of this project.
    pub last_poll: Option<DateTime<Utc>>,
    /// Error of the last poll, if it failed.
    pub last_error: Option<String>,
    /// Last heartbeat of any poller (daemon or in-app).
    pub poller_heartbeat: Option<DateTime<Utc>>,
    /// The project's poll interval.
    pub interval: Duration,
}

/// The bell state. A heartbeat older than three intervals counts as no poller.
pub fn status(i: &StatusInput, now: DateTime<Utc>) -> NotifyStatus {
    if !i.enabled {
        return NotifyStatus::Off;
    }
    if let Some(until) = i.muted_until
        && until > now
    {
        return NotifyStatus::Snoozed { until };
    }
    let grace =
        chrono::Duration::from_std(i.interval * 3).unwrap_or(chrono::Duration::seconds(180));
    let alive = i.poller_heartbeat.is_some_and(|h| now - h <= grace);
    if !alive {
        return NotifyStatus::NoPoller;
    }
    if let Some(reason) = &i.last_error {
        return NotifyStatus::Error {
            reason: reason.clone(),
        };
    }
    NotifyStatus::On {
        last_poll: i.last_poll,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + s, 0).expect("ts")
    }

    fn input() -> StatusInput {
        StatusInput {
            enabled: true,
            poller_heartbeat: Some(at(0)),
            interval: Duration::from_secs(60),
            ..StatusInput::default()
        }
    }

    #[test]
    fn off_beats_everything() {
        let i = StatusInput {
            enabled: false,
            last_error: Some("x".into()),
            ..input()
        };
        assert_eq!(status(&i, at(0)), NotifyStatus::Off);
    }

    #[test]
    fn snooze_expires() {
        let i = StatusInput {
            muted_until: Some(at(100)),
            ..input()
        };
        assert!(matches!(status(&i, at(50)), NotifyStatus::Snoozed { .. }));
        assert!(matches!(status(&i, at(100)), NotifyStatus::On { .. }));
    }

    #[test]
    fn stale_heartbeat_means_no_poller() {
        assert!(matches!(status(&input(), at(180)), NotifyStatus::On { .. }));
        assert_eq!(status(&input(), at(181)), NotifyStatus::NoPoller);
        let none = StatusInput {
            poller_heartbeat: None,
            ..input()
        };
        assert_eq!(status(&none, at(0)), NotifyStatus::NoPoller);
    }

    #[test]
    fn error_and_ok_states() {
        let e = StatusInput {
            last_error: Some("rate limited".into()),
            ..input()
        };
        assert_eq!(
            status(&e, at(10)),
            NotifyStatus::Error {
                reason: "rate limited".into()
            }
        );
        let ok = StatusInput {
            last_poll: Some(at(5)),
            ..input()
        };
        assert_eq!(
            status(&ok, at(10)),
            NotifyStatus::On {
                last_poll: Some(at(5))
            }
        );
    }

    #[test]
    fn glyphs_and_descriptions_differ_per_state() {
        let all = [
            NotifyStatus::Off,
            NotifyStatus::On { last_poll: None },
            NotifyStatus::Snoozed { until: at(3600) },
            NotifyStatus::Error { reason: "x".into() },
            NotifyStatus::NoPoller,
        ];
        let ascii: std::collections::HashSet<_> = all.iter().map(|s| s.glyph(true)).collect();
        assert_eq!(ascii.len(), all.len());
        assert!(all[2].describe(at(0)).contains("1h 00m"));
    }
}
