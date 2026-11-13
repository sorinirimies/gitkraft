//! Real desktop notifications through `notify-rust`.

use super::alert::Alert;
use super::deliver::Notifier;

/// Shows alerts in the system notification centre.
#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopNotifier;

impl Notifier for DesktopNotifier {
    fn show(&self, alert: &Alert, sound: bool) {
        let mut n = notify_rust::Notification::new();
        n.appname("GitKraft")
            .summary(&alert.headline())
            .body(&format!("{}\n{}", alert.title, alert.deep_link()));
        if sound {
            n.sound_name("default");
        }
        // A missing notification daemon must not break polling; the inbox still has the alert.
        if let Err(e) = n.show() {
            tracing::warn!("desktop notification failed: {e}");
        }
    }
}
