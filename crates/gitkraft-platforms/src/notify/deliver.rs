use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveTime, Utc};
use gitkraft_settings::notify::Effective;

use super::alert::Alert;
use super::inbox::Inbox;

/// Shows an alert to the user. The real one is [`desktop::DesktopNotifier`](super);
/// tests use [`RecordingNotifier`].
pub trait Notifier {
    /// Shows `alert`. Failures are the notifier's to swallow: the inbox still has the alert.
    fn show(&self, alert: &Alert, sound: bool);
}

/// Remembers what it was asked to show.
#[derive(Debug, Clone, Default)]
pub struct RecordingNotifier {
    shown: Arc<Mutex<Vec<Alert>>>,
}

impl RecordingNotifier {
    /// Alerts shown so far.
    pub fn shown(&self) -> Vec<Alert> {
        self.shown.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Notifier for RecordingNotifier {
    fn show(&self, alert: &Alert, _sound: bool) {
        if let Ok(mut s) = self.shown.lock() {
            s.push(alert.clone());
        }
    }
}

/// Filters `alerts` through `eff`, then sends the survivors to the inbox and,
/// unless quiet hours or a snooze apply, to the desktop. Returns how many passed.
pub fn deliver(
    alerts: Vec<Alert>,
    eff: &Effective,
    muted_until: Option<DateTime<Utc>>,
    local_time: NaiveTime,
    now: DateTime<Utc>,
    notifier: &dyn Notifier,
    inbox: &mut Inbox,
) -> usize {
    let snoozed = muted_until.is_some_and(|u| u > now);
    let quiet = eff.quiet_hours.is_some_and(|q| q.contains(local_time));
    let mut passed = 0;
    for alert in alerts.into_iter().filter(|a| eff.accepts(&a.candidate())) {
        passed += 1;
        if eff.desktop && !snoozed && !quiet {
            notifier.show(&alert, eff.sound);
        }
        if eff.inbox {
            inbox.push(alert, now);
        }
    }
    passed
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_settings::notify::{NotifyConfig, NotifyEvent};

    use crate::platform::RepoId;

    fn alert(event: NotifyEvent) -> Alert {
        Alert {
            repo: RepoId::new("h", "o", "r"),
            event,
            number: Some(1),
            title: "t".into(),
            url: "u".into(),
            branch: None,
            labels: vec![],
            author: None,
            mine: true,
            involved: true,
        }
    }

    fn eff(extra: &str) -> Effective {
        let text = format!(
            "[defaults]\nenabled = true\nscope = \"all\"\nevents = [\"pr_opened\"]\n{extra}"
        );
        NotifyConfig::parse(&text)
            .expect("valid")
            .resolve("h/o/r", None)
    }

    fn t(h: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, 0, 0).expect("time")
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("ts")
    }

    fn run(e: &Effective, muted: Option<DateTime<Utc>>, hour: u32) -> (usize, usize, usize) {
        let n = RecordingNotifier::default();
        let mut inbox = Inbox::default();
        let passed = deliver(
            vec![
                alert(NotifyEvent::PrOpened),
                alert(NotifyEvent::IssueOpened),
            ],
            e,
            muted,
            t(hour),
            now(),
            &n,
            &mut inbox,
        );
        (passed, n.shown().len(), inbox.entries().len())
    }

    #[test]
    fn filtered_alerts_reach_desktop_and_inbox() {
        assert_eq!(run(&eff(""), None, 12), (1, 1, 1));
    }

    #[test]
    fn quiet_hours_and_snooze_silence_desktop_but_keep_inbox() {
        assert_eq!(
            run(&eff("quiet_hours = \"22:00-08:00\""), None, 23),
            (1, 0, 1)
        );
        assert_eq!(
            run(&eff(""), Some(now() + chrono::Duration::hours(1)), 12),
            (1, 0, 1)
        );
        assert_eq!(
            run(&eff(""), Some(now() - chrono::Duration::hours(1)), 12),
            (1, 1, 1),
            "expired snooze"
        );
    }

    #[test]
    fn delivery_channels_are_switchable() {
        assert_eq!(run(&eff("desktop = false"), None, 12), (1, 0, 1));
        assert_eq!(run(&eff("inbox = false"), None, 12), (1, 1, 0));
        let off = Effective {
            enabled: false,
            ..eff("")
        };
        assert_eq!(run(&off, None, 12), (0, 0, 0));
    }
}
