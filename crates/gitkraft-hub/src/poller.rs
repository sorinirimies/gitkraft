//! The notification poller: one cycle polls every due project, raises alerts
//! and delivers them. The daemon and the TUI both call [`run_cycle`].

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, NaiveTime, Utc};
use gitkraft_ci::RunQuery;
use gitkraft_ci::events::{CiMemory, alerts as ci_alerts, detect};
use gitkraft_platforms::notify::{
    Alert, Inbox, Notifier, NotifyState, PollerStatus, deliver, is_due, poll_inbox, poll_repo,
};
use gitkraft_platforms::{HttpClient, RepoId};
use gitkraft_settings::files;
use gitkraft_settings::notify::{NotifyConfig, NotifySettings};

use crate::Hub;

/// Everything the poller remembers; saved between runs.
#[derive(Debug, Clone, Default)]
pub struct Engine {
    /// Snapshots, seen inbox ids, snoozes.
    pub state: NotifyState,
    /// Last CI result per workflow and branch, by project key.
    pub ci: BTreeMap<String, CiMemory>,
    /// Bell state for the UIs.
    pub status: PollerStatus,
    /// The in-app inbox.
    pub inbox: Inbox,
    last_attempt: BTreeMap<String, DateTime<Utc>>,
    last_inbox_attempt: BTreeMap<String, DateTime<Utc>>,
}

/// What one cycle did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CycleReport {
    /// Projects polled.
    pub polled: usize,
    /// Alerts that passed the filters.
    pub alerts: usize,
    /// Projects whose poll failed.
    pub failed: usize,
}

impl Engine {
    /// Loads the saved state from the default state directory; missing or broken files start fresh.
    pub fn load() -> Self {
        let read = |p: anyhow::Result<std::path::PathBuf>| {
            p.ok().and_then(|p| files::read_optional(&p).ok().flatten())
        };
        Self {
            state: read(files::notify_state_path())
                .map(|t| NotifyState::from_json(&t))
                .unwrap_or_default(),
            ci: read(files::ci_memory_path())
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default(),
            status: read(files::notify_status_path())
                .map(|t| PollerStatus::from_json(&t))
                .unwrap_or_default(),
            inbox: read(files::inbox_path())
                .map(|t| Inbox::from_json(&t))
                .unwrap_or_default(),
            ..Self::default()
        }
    }

    /// Saves everything to the default state directory.
    pub fn save(&self) -> anyhow::Result<()> {
        files::write_atomic(&files::notify_state_path()?, &self.state.to_json())?;
        files::write_atomic(&files::ci_memory_path()?, &serde_json::to_string(&self.ci)?)?;
        files::write_atomic(&files::notify_status_path()?, &self.status.to_json())?;
        files::write_atomic(&files::inbox_path()?, &self.inbox.to_json())?;
        Ok(())
    }

    /// Saves under `dir` (tests).
    pub fn save_to(&self, dir: &Path) -> anyhow::Result<()> {
        files::write_atomic(&dir.join("state.json"), &self.state.to_json())?;
        files::write_atomic(&dir.join("inbox.json"), &self.inbox.to_json())?;
        Ok(())
    }
}

/// Polls every due, enabled project in `repos` once. Never panics and never stops on one failure:
/// each project's error lands in `engine.status` and the others still run.
#[allow(
    clippy::too_many_arguments,
    reason = "one cycle needs the hub, state, config, clock and notifier; a struct would only rename them"
)]
pub async fn run_cycle<C: HttpClient + Clone>(
    hub: &Hub<C>,
    engine: &mut Engine,
    repos: &[RepoId],
    config: &NotifyConfig,
    locals: &BTreeMap<String, NotifySettings>,
    now: DateTime<Utc>,
    local_time: NaiveTime,
    notifier: &dyn Notifier,
) -> CycleReport {
    let mut report = CycleReport::default();
    engine.status.heartbeat = Some(now);
    let mut inbox_hosts: BTreeMap<String, std::time::Duration> = BTreeMap::new();

    for repo in repos {
        let key = repo.key();
        let eff = config.resolve(&key, locals.get(&key));
        if !eff.enabled {
            continue;
        }
        inbox_hosts
            .entry(repo.host.clone())
            .and_modify(|d| *d = (*d).min(eff.poll_interval))
            .or_insert(eff.poll_interval);
        if !is_due(
            engine.last_attempt.get(&key).copied(),
            eff.poll_interval,
            now,
        ) {
            continue;
        }
        engine.last_attempt.insert(key.clone(), now);
        report.polled += 1;
        let (Some(backend), Some(ci)) = (hub.backend(&repo.host), hub.ci(&repo.host)) else {
            engine
                .status
                .failed(&key, "no account for this host".into(), now);
            report.failed += 1;
            continue;
        };
        let me = match hub.me(&repo.host).await {
            Ok(me) => me,
            Err(e) => {
                engine.status.failed(&key, e.to_string(), now);
                report.failed += 1;
                continue;
            }
        };
        let mut alerts: Vec<Alert> = Vec::new();
        let mut error = None;
        match poll_repo(backend, repo, &me, &mut engine.state).await {
            Ok(a) => alerts.extend(a),
            Err(e) => error = Some(e.to_string()),
        }
        match ci.list_runs(repo, &RunQuery::default()).await {
            Ok(runs) => {
                let memory = engine.ci.entry(key.clone()).or_default();
                let events = detect(memory, &runs);
                alerts.extend(ci_alerts(repo, &events, &me));
            }
            // Repos without CI answer 404; that is not an error worth showing.
            Err(gitkraft_platforms::PlatformError::NotFound(_)) => {}
            Err(e) => error = error.or(Some(e.to_string())),
        }
        match error {
            Some(e) => {
                engine.status.failed(&key, e, now);
                report.failed += 1;
            }
            None => engine.status.ok(&key, now),
        }
        let muted = engine.state.muted_until(&key);
        report.alerts += deliver(
            alerts,
            &eff,
            muted,
            local_time,
            now,
            notifier,
            &mut engine.inbox,
        );
    }

    // Account-wide inbox: review requests, mentions and CI activity, once per host.
    for (host, interval) in inbox_hosts {
        if !is_due(engine.last_inbox_attempt.get(&host).copied(), interval, now) {
            continue;
        }
        engine.last_inbox_attempt.insert(host.clone(), now);
        let Some(backend) = hub.backend(&host) else {
            continue;
        };
        let Ok(alerts) = poll_inbox(backend, &host, &mut engine.state).await else {
            continue;
        };
        let mut by_repo: BTreeMap<String, Vec<Alert>> = BTreeMap::new();
        for a in alerts {
            by_repo.entry(a.repo.key()).or_default().push(a);
        }
        for (key, alerts) in by_repo {
            let eff = config.resolve(&key, locals.get(&key));
            let muted = engine.state.muted_until(&key);
            report.alerts += deliver(
                alerts,
                &eff,
                muted,
                local_time,
                now,
                notifier,
                &mut engine.inbox,
            );
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::notify::RecordingNotifier;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, PlatformKind};

    fn at(s: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + s, 0).expect("ts")
    }

    fn noon() -> NaiveTime {
        NaiveTime::from_hms_opt(12, 0, 0).expect("time")
    }

    fn setup(fake: &FakeClient) -> Hub<FakeClient> {
        let mut a = Account::new(PlatformKind::GitHub, "github.com");
        a.me = Some("me".into());
        Hub::new(vec![a], fake)
    }

    fn config(extra: &str) -> NotifyConfig {
        NotifyConfig::parse(&format!("[defaults]\nenabled = true\nscope = \"all\"\npoll_interval = \"60s\"\nevents = [\"pr_opened\", \"ci_failed\"]\n{extra}")).expect("valid")
    }

    fn repo() -> RepoId {
        RepoId::new("github.com", "o", "r")
    }

    const PULLS_1: &str =
        r#"[{"number":1,"title":"a","state":"open","user":{"login":"x"},"html_url":"u1"}]"#;
    const PULLS_2: &str = r#"[{"number":1,"title":"a","state":"open","user":{"login":"x"},"html_url":"u1"},{"number":2,"title":"b","state":"open","user":{"login":"y"},"html_url":"u2"}]"#;
    const NO_RUNS: &str = r#"{"workflow_runs":[]}"#;

    fn api(fake: &FakeClient, pulls: &[&str]) {
        fake.push_json("https://api.github.com/repos/o/r/issues", "[]");
        fake.push_json("https://api.github.com/repos/o/r/actions/runs", NO_RUNS);
        fake.push_json("https://api.github.com/notifications", "[]");
        for p in pulls {
            fake.push_json("https://api.github.com/repos/o/r/pulls", p);
        }
    }

    #[tokio::test]
    async fn first_cycle_seeds_second_alerts_and_the_interval_is_respected() {
        let fake = FakeClient::new();
        api(&fake, &[PULLS_1, PULLS_2]);
        let hub = setup(&fake);
        let mut engine = Engine::default();
        let notifier = RecordingNotifier::default();
        let cfg = config("");
        let locals = BTreeMap::new();

        let r1 = run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &locals,
            at(0),
            noon(),
            &notifier,
        )
        .await;
        assert_eq!((r1.polled, r1.alerts, r1.failed), (1, 0, 0));
        assert!(engine.status.repos["github.com/o/r"].last_poll.is_some());

        let early = run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &locals,
            at(30),
            noon(),
            &notifier,
        )
        .await;
        assert_eq!(early.polled, 0, "not due yet");

        let r2 = run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &locals,
            at(61),
            noon(),
            &notifier,
        )
        .await;
        assert_eq!((r2.polled, r2.alerts), (1, 1));
        assert_eq!(notifier.shown().len(), 1);
        assert_eq!(engine.inbox.unread(), 1);
        assert_eq!(engine.status.heartbeat, Some(at(61)));
    }

    #[tokio::test]
    async fn disabled_projects_are_not_polled() {
        let fake = FakeClient::new();
        api(&fake, &[PULLS_1]);
        let hub = setup(&fake);
        let mut engine = Engine::default();
        let cfg =
            NotifyConfig::parse("[projects.\"github.com/o/r\"]\nenabled = false\n").expect("valid");
        let r = run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &BTreeMap::new(),
            at(0),
            noon(),
            &RecordingNotifier::default(),
        )
        .await;
        assert_eq!(r.polled, 0);
        assert!(fake.requests().is_empty());
        assert!(engine.status.repos.is_empty());
    }

    #[tokio::test]
    async fn a_failing_project_is_recorded_and_does_not_stop_the_others() {
        let fake = FakeClient::new();
        fake.push_status("https://api.github.com/repos/o/bad/pulls", 500, "boom");
        fake.push_json("https://api.github.com/repos/o/bad/issues", "[]");
        fake.push_json("https://api.github.com/repos/o/bad/actions/runs", NO_RUNS);
        api(&fake, &[PULLS_1]);
        let hub = setup(&fake);
        let mut engine = Engine::default();
        let bad = RepoId::new("github.com", "o", "bad");
        let r = run_cycle(
            &hub,
            &mut engine,
            &[bad, repo()],
            &config(""),
            &BTreeMap::new(),
            at(0),
            noon(),
            &RecordingNotifier::default(),
        )
        .await;
        assert_eq!((r.polled, r.failed), (2, 1));
        assert!(
            engine.status.repos["github.com/o/bad"]
                .last_error
                .as_deref()
                .is_some_and(|e| e.contains("500"))
        );
        assert_eq!(engine.status.repos["github.com/o/r"].last_error, None);
    }

    #[tokio::test]
    async fn ci_failure_raises_an_alert_after_the_seed_run() {
        let fake = FakeClient::new();
        fake.push_json("https://api.github.com/repos/o/r/pulls", "[]");
        fake.push_json("https://api.github.com/repos/o/r/issues", "[]");
        fake.push_json("https://api.github.com/notifications", "[]");
        let run = |id: u64, c: &str| {
            format!(
                r#"{{"id":{id},"name":"CI","status":"completed","conclusion":"{c}","head_branch":"master","actor":{{"login":"x"}}}}"#
            )
        };
        fake.push_json(
            "https://api.github.com/repos/o/r/actions/runs",
            &format!(r#"{{"workflow_runs":[{}]}}"#, run(1, "success")),
        );
        fake.push_json(
            "https://api.github.com/repos/o/r/actions/runs",
            &format!(
                r#"{{"workflow_runs":[{},{}]}}"#,
                run(2, "failure"),
                run(1, "success")
            ),
        );
        let hub = setup(&fake);
        let mut engine = Engine::default();
        let n = RecordingNotifier::default();
        let cfg = config("");
        run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &BTreeMap::new(),
            at(0),
            noon(),
            &n,
        )
        .await;
        let r = run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &BTreeMap::new(),
            at(61),
            noon(),
            &n,
        )
        .await;
        assert_eq!(r.alerts, 1);
        assert_eq!(
            n.shown()[0].event,
            gitkraft_settings::notify::NotifyEvent::CiFailed
        );
    }

    #[tokio::test]
    async fn snooze_keeps_the_desktop_quiet_but_fills_the_inbox() {
        let fake = FakeClient::new();
        api(&fake, &[PULLS_1, PULLS_2]);
        let hub = setup(&fake);
        let mut engine = Engine::default();
        engine.state.mute(None, at(10_000));
        let n = RecordingNotifier::default();
        let cfg = config("");
        run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &BTreeMap::new(),
            at(0),
            noon(),
            &n,
        )
        .await;
        run_cycle(
            &hub,
            &mut engine,
            &[repo()],
            &cfg,
            &BTreeMap::new(),
            at(61),
            noon(),
            &n,
        )
        .await;
        assert!(n.shown().is_empty());
        assert_eq!(engine.inbox.unread(), 1);
    }

    #[test]
    fn saved_files_round_trip_through_a_directory() {
        let dir = tempfile::tempdir().expect("tmp");
        let mut e = Engine::default();
        e.state.mute(None, at(5));
        e.save_to(dir.path()).expect("save");
        let text = std::fs::read_to_string(dir.path().join("state.json")).expect("read");
        assert_eq!(NotifyState::from_json(&text), e.state);
    }
}
