//! Notifications: turn poll results into alerts, filter them by the project's
//! settings, show them and keep them in an inbox.
//!
//! The pure parts (snapshot diffing, delivery rules, status file) live here and
//! are tested with a fake clock and a fake notifier; only [`desktop`] touches
//! the real desktop.

mod alert;
mod deliver;
mod diff;
mod inbox;
mod poll;
mod state;

#[cfg(feature = "desktop")]
pub mod desktop;

pub use alert::Alert;
pub use deliver::{Notifier, RecordingNotifier, deliver};
pub use diff::{RepoSnapshot, diff_snapshots, inbox_alerts, snapshot};
pub use inbox::{Inbox, InboxEntry};
pub use poll::{poll_inbox, poll_repo};
pub use state::{NotifyState, PollerStatus, RepoPollStatus, is_due};
