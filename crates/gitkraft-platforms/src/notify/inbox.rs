use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::alert::Alert;

/// Most entries kept; older ones fall off.
const CAP: usize = 500;

/// One inbox row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxEntry {
    /// The alert.
    pub alert: Alert,
    /// When it arrived.
    pub at: DateTime<Utc>,
    /// Marked read / done.
    pub read: bool,
}

/// The in-app inbox: newest first, capped.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Inbox {
    entries: Vec<InboxEntry>,
}

impl Inbox {
    /// Adds an alert at the top. The same item and event is not added twice while unread.
    pub fn push(&mut self, alert: Alert, at: DateTime<Utc>) {
        let dup = self.entries.iter().any(|e| {
            !e.read
                && e.alert.repo == alert.repo
                && e.alert.event == alert.event
                && e.alert.number == alert.number
        });
        if dup {
            return;
        }
        self.entries.insert(
            0,
            InboxEntry {
                alert,
                at,
                read: false,
            },
        );
        self.entries.truncate(CAP);
    }

    /// Entries, newest first.
    pub fn entries(&self) -> &[InboxEntry] {
        &self.entries
    }

    /// Unread count.
    pub fn unread(&self) -> usize {
        self.entries.iter().filter(|e| !e.read).count()
    }

    /// Unread count for one project (the number next to its bell).
    pub fn unread_for(&self, repo_key: &str) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.read && e.alert.repo.key() == repo_key)
            .count()
    }

    /// Marks entry `index` read. Out of range is a no-op.
    pub fn mark_read(&mut self, index: usize) {
        if let Some(e) = self.entries.get_mut(index) {
            e.read = true;
        }
    }

    /// Marks everything read.
    pub fn mark_all_read(&mut self) {
        self.entries.iter_mut().for_each(|e| e.read = true);
    }

    /// Drops read entries.
    pub fn clear_read(&mut self) {
        self.entries.retain(|e| !e.read);
    }

    /// Parses a saved inbox; garbage gives an empty one.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// Serializes for saving.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::RepoId;
    use gitkraft_settings::notify::NotifyEvent;

    fn alert(n: u64) -> Alert {
        Alert {
            repo: RepoId::new("h", "o", "r"),
            event: NotifyEvent::PrOpened,
            number: Some(n),
            title: "t".into(),
            url: "u".into(),
            branch: None,
            labels: vec![],
            author: None,
            mine: false,
            involved: false,
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("ts")
    }

    #[test]
    fn newest_first_and_deduplicated_while_unread() {
        let mut i = Inbox::default();
        i.push(alert(1), now());
        i.push(alert(2), now());
        i.push(alert(1), now());
        assert_eq!(
            i.entries()
                .iter()
                .map(|e| e.alert.number)
                .collect::<Vec<_>>(),
            vec![Some(2), Some(1)]
        );
        i.mark_read(1);
        i.push(alert(1), now());
        assert_eq!(i.entries().len(), 3, "a read entry may come again");
    }

    #[test]
    fn counts_read_state_and_clear() {
        let mut i = Inbox::default();
        i.push(alert(1), now());
        i.push(alert(2), now());
        assert_eq!(
            (i.unread(), i.unread_for("h/o/r"), i.unread_for("h/o/x")),
            (2, 2, 0)
        );
        i.mark_read(99);
        assert_eq!(i.unread(), 2);
        i.mark_read(0);
        assert_eq!(i.unread(), 1);
        i.clear_read();
        assert_eq!(i.entries().len(), 1);
        i.mark_all_read();
        assert_eq!(i.unread(), 0);
    }

    #[test]
    fn capped_and_json_round_trips() {
        let mut i = Inbox::default();
        for n in 0..(CAP as u64 + 20) {
            i.push(alert(n), now());
        }
        assert_eq!(i.entries().len(), CAP);
        assert_eq!(Inbox::from_json(&i.to_json()), i);
        assert_eq!(Inbox::from_json("garbage"), Inbox::default());
    }
}
