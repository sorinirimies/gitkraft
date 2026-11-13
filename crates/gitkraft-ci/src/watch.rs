//! How often to poll CI: fast while something runs, slow when everything is idle.

use std::time::Duration;

use crate::model::Run;

/// Polling interval for a list of runs. Pass `visible = false` when the CI view is
/// not on screen: the poll slows down instead of stopping, so alerts still arrive.
pub fn poll_interval(runs: &[Run], visible: bool) -> Duration {
    let running = runs
        .iter()
        .any(|r| r.status == crate::model::Status::Running);
    let queued = runs.iter().any(|r| {
        matches!(
            r.status,
            crate::model::Status::Queued | crate::model::Status::Waiting
        )
    });
    let secs = match (running, queued, visible) {
        (true, _, true) => 3,
        (true, _, false) => 15,
        (false, true, true) => 10,
        (false, true, false) => 30,
        (false, false, true) => 60,
        (false, false, false) => 300,
    };
    Duration::from_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Status;

    fn runs(s: &[Status]) -> Vec<Run> {
        s.iter()
            .map(|&status| Run {
                status,
                ..Run::default()
            })
            .collect()
    }

    #[test]
    fn faster_when_active_slower_when_hidden() {
        assert_eq!(
            poll_interval(&runs(&[Status::Success, Status::Running]), true),
            Duration::from_secs(3)
        );
        assert_eq!(
            poll_interval(&runs(&[Status::Queued]), true),
            Duration::from_secs(10)
        );
        assert_eq!(
            poll_interval(&runs(&[Status::Success]), true),
            Duration::from_secs(60)
        );
        assert_eq!(
            poll_interval(&runs(&[Status::Running]), false),
            Duration::from_secs(15)
        );
        assert_eq!(poll_interval(&[], false), Duration::from_secs(300));
    }
}
