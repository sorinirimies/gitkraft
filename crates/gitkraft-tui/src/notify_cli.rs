//! `gitkraft-tui notify …`: run the notification poller headless, and control it.
//!
//! ```text
//! notify status                 bell state of every project
//! notify on|off [REPO]          per-project switch (REPO = host/owner/name, default: this checkout)
//! notify on|off --all           the global switch
//! notify mute DURATION [REPO]   snooze, e.g. 2h; `notify unmute [REPO]` ends it
//! notify test                   show a sample desktop notification
//! notify --once                 poll everything once and exit (cron)
//! notify --daemon               keep polling (launchd / systemd)
//! notify install                write the launchd / systemd user service
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, Utc};
use gitkraft_hub::Hub;
use gitkraft_hub::accounts::{AccountsFile, discover};
use gitkraft_hub::poller::{Engine, run_cycle};
use gitkraft_platforms::notify::{Alert, Notifier, desktop::DesktopNotifier};
use gitkraft_platforms::{CachedClient, RepoId, ReqwestClient};
use gitkraft_settings::dashboard::Dashboard;
use gitkraft_settings::files;
use gitkraft_settings::notify::{
    NotifyConfig, NotifySettings, NotifyStatus, StatusInput, parse_duration, set_enabled, status,
};

/// A parsed `notify` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print the bell state of every project.
    Status,
    /// Turn notifications on or off: `repo` of `None` means this checkout, `all` the global switch.
    Switch {
        /// New state.
        on: bool,
        /// Explicit project key.
        repo: Option<String>,
        /// Global switch.
        all: bool,
    },
    /// Snooze for a duration.
    Mute {
        /// How long.
        for_secs: u64,
        /// Explicit project key.
        repo: Option<String>,
    },
    /// End a snooze.
    Unmute {
        /// Explicit project key.
        repo: Option<String>,
    },
    /// Show a sample notification.
    Test,
    /// Poll once and exit.
    Once,
    /// Poll until killed.
    Daemon,
    /// Write the service file.
    Install,
}

/// Parses the arguments after `notify`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut it = args.iter().map(String::as_str);
    let first = it
        .next()
        .ok_or("usage: notify status|on|off|mute|unmute|test|--once|--daemon|install")?;
    let rest: Vec<&str> = it.collect();
    let repo = |r: &[&str]| {
        r.iter()
            .find(|a| !a.starts_with("--"))
            .map(|s| (*s).to_string())
    };
    match first {
        "status" => Ok(Command::Status),
        "on" | "off" => Ok(Command::Switch {
            on: first == "on",
            all: rest.contains(&"--all"),
            repo: repo(&rest),
        }),
        "mute" => {
            let dur = rest.first().ok_or("mute needs a duration, e.g. 2h")?;
            let secs = parse_duration(dur)
                .ok_or_else(|| format!("bad duration '{dur}' (try 30m, 2h, 1d)"))?
                .as_secs();
            Ok(Command::Mute {
                for_secs: secs,
                repo: repo(&rest[1..]),
            })
        }
        "unmute" => Ok(Command::Unmute { repo: repo(&rest) }),
        "test" => Ok(Command::Test),
        "--once" | "once" => Ok(Command::Once),
        "--daemon" | "daemon" => Ok(Command::Daemon),
        "install" => Ok(Command::Install),
        other => Err(format!("unknown notify command '{other}'")),
    }
}

/// One status row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Project key.
    pub key: String,
    /// Bell state.
    pub status: NotifyStatus,
    /// Poll interval in seconds.
    pub interval: u64,
    /// Unread inbox entries.
    pub unread: usize,
}

/// Text of `notify status`.
pub fn render_status(rows: &[Row], mode: &str, now: DateTime<Utc>) -> String {
    if rows.is_empty() {
        return "no projects are configured; run `gitkraft-tui notify on` inside a checkout\n"
            .into();
    }
    let width = rows.iter().map(|r| r.key.len()).max().unwrap_or(0);
    let mut out = format!(
        "poller: {}\n",
        if mode.is_empty() { "not running" } else { mode }
    );
    for r in rows {
        out.push_str(&format!(
            "{} {:<width$}  every {:>4}s  unread {:>3}  {}\n",
            r.status.glyph(true),
            r.key,
            r.interval,
            r.unread,
            r.status.describe(now)
        ));
    }
    out
}

/// The service definition for `os` (`macos`, `linux`), running `exe notify --daemon`.
pub fn service_unit(os: &str, exe: &str) -> Option<(String, String)> {
    match os {
        "macos" => Some((
            "Library/LaunchAgents/io.github.sorinirimies.gitkraft.notify.plist".into(),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>io.github.sorinirimies.gitkraft.notify</string>\n  <key>ProgramArguments</key>\n  <array><string>{exe}</string><string>notify</string><string>--daemon</string></array>\n  <key>RunAtLoad</key><true/>\n  <key>KeepAlive</key><true/>\n</dict>\n</plist>\n"
            ),
        )),
        "linux" => Some((
            ".config/systemd/user/gitkraft-notify.service".into(),
            format!(
                "[Unit]\nDescription=GitKraft notification poller\n\n[Service]\nExecStart={exe} notify --daemon\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n"
            ),
        )),
        _ => None,
    }
}

/// Runs a command; returns the text to print.
pub fn run(cmd: Command) -> Result<String> {
    match cmd {
        Command::Status => status_text(),
        Command::Switch { on, repo, all } => {
            let key = if all { None } else { Some(resolve_repo(repo)?) };
            let path = files::notify_config_path()?;
            let text = files::read_optional(&path)?.unwrap_or_default();
            let edited = set_enabled(&text, key.as_deref(), on).map_err(|e| anyhow!(e))?;
            files::write_atomic(&path, &edited)?;
            Ok(format!(
                "notifications {} for {}\n",
                if on { "on" } else { "off" },
                key.as_deref().unwrap_or("all projects")
            ))
        }
        Command::Mute { for_secs, repo } => {
            let key = resolve_repo(repo)?;
            let mut engine = Engine::load();
            let until = Utc::now()
                + chrono::Duration::seconds(i64::try_from(for_secs).unwrap_or(i64::MAX / 2));
            engine.state.mute(Some(&key), until);
            engine.save()?;
            Ok(format!(
                "{key} snoozed until {}\n",
                until.with_timezone(&Local).format("%H:%M")
            ))
        }
        Command::Unmute { repo } => {
            let key = resolve_repo(repo)?;
            let mut engine = Engine::load();
            engine.state.unmute(Some(&key));
            engine.save()?;
            Ok(format!("{key} snooze ended\n"))
        }
        Command::Test => {
            DesktopNotifier.show(&sample_alert(), false);
            Ok("sent a test notification\n".into())
        }
        Command::Install => install(),
        Command::Once => {
            let report = poll_forever(true)?;
            Ok(report)
        }
        Command::Daemon => poll_forever(false),
    }
}

fn sample_alert() -> Alert {
    Alert {
        repo: RepoId::new("github.com", "sorinirimies", "gitkraft"),
        event: gitkraft_settings::notify::NotifyEvent::ReviewRequested,
        number: Some(1),
        title: "Test notification from GitKraft".into(),
        url: String::new(),
        branch: None,
        labels: Vec::new(),
        author: None,
        mine: true,
        involved: true,
    }
}

/// The project key for an explicit argument or this checkout's `origin`.
fn resolve_repo(explicit: Option<String>) -> Result<String> {
    if let Some(key) = explicit {
        return RepoId::from_key(&key)
            .map(|r| r.key())
            .ok_or_else(|| anyhow!("'{key}' is not host/owner/name"));
    }
    let repo = git2::Repository::discover(".")
        .context("not inside a git checkout; pass host/owner/name")?;
    let remotes = gitkraft_core::features::remotes::list_remotes(&repo)?;
    let url = remotes
        .iter()
        .find(|r| r.name == "origin")
        .or_else(|| remotes.first())
        .and_then(|r| r.url.clone())
        .ok_or_else(|| anyhow!("this checkout has no remote with a URL"))?;
    let url = url.as_str();
    gitkraft_platforms::parse_remote_url(url)
        .map(|r| r.key())
        .ok_or_else(|| anyhow!("cannot read a host/owner/name from '{url}'"))
}

fn load_config() -> NotifyConfig {
    files::notify_config_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
        .and_then(|t| NotifyConfig::parse(&t).ok())
        .unwrap_or_default()
}

fn status_text() -> Result<String> {
    let config = load_config();
    let engine = Engine::load();
    let mut keys: Vec<String> = config.projects.keys().cloned().collect();
    keys.extend(engine.status.repos.keys().cloned());
    keys.sort();
    keys.dedup();
    let now = Utc::now();
    let rows: Vec<Row> = keys
        .into_iter()
        .map(|key| {
            let eff = config.resolve(&key, None);
            let st = engine.status.repos.get(&key).cloned().unwrap_or_default();
            let status = status(
                &StatusInput {
                    enabled: eff.enabled,
                    muted_until: engine.state.muted_until(&key),
                    last_poll: st.last_poll,
                    last_error: st.last_error,
                    poller_heartbeat: engine.status.heartbeat,
                    interval: eff.poll_interval,
                },
                now,
            );
            Row {
                unread: engine.inbox.unread_for(&key),
                interval: eff.poll_interval.as_secs(),
                key,
                status,
            }
        })
        .collect();
    let alive = engine
        .status
        .heartbeat
        .is_some_and(|h| now - h < chrono::Duration::minutes(5));
    Ok(render_status(
        &rows,
        if alive { &engine.status.mode } else { "" },
        now,
    ))
}

fn install() -> Result<String> {
    let exe = std::env::current_exe().context("cannot find this executable")?;
    let exe = exe.to_string_lossy();
    let os = std::env::consts::OS;
    let Some((rel, body)) = service_unit(os, &exe) else {
        return Ok(format!(
            "run `{exe} notify --daemon` at login (on Windows: schtasks /Create /SC ONLOGON /TN GitKraftNotify /TR \"{exe} notify --daemon\")\n"
        ));
    };
    let path = dirs::home_dir().context("no home directory")?.join(rel);
    files::write_atomic(&path, &body)?;
    let hint = if os == "macos" {
        format!("load it with: launchctl load {}", path.display())
    } else {
        "enable it with: systemctl --user enable --now gitkraft-notify".to_string()
    };
    Ok(format!("wrote {}\n{hint}\n", path.display()))
}

/// Watched projects: every configured project and every repo named in the dashboard.
fn watched(config: &NotifyConfig) -> Vec<RepoId> {
    let mut repos: Vec<RepoId> = config
        .projects
        .keys()
        .filter_map(|k| RepoId::from_key(k))
        .collect();
    let dashboard = files::dashboard_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
        .and_then(|t| Dashboard::parse(&t).ok())
        .unwrap_or_default();
    for key in dashboard.sections.iter().flat_map(|s| s.repos.iter()) {
        if let Some(r) = RepoId::from_key(key)
            && !repos.contains(&r)
        {
            repos.push(r);
        }
    }
    repos
}

fn poll_forever(once: bool) -> Result<String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("cannot start the runtime")?;
    rt.block_on(async move {
        let config = load_config();
        let repos = watched(&config);
        let file = files::accounts_path()
            .ok()
            .and_then(|p| files::read_optional(&p).ok().flatten())
            .map(|t| AccountsFile::parse(&t).map_err(|e| anyhow!("accounts.toml: {e}")))
            .transpose()?
            .unwrap_or_default();
        let urls: Vec<String> = Vec::new();
        let hosts: Vec<String> = repos
            .iter()
            .map(|r| format!("https://{}/{}/{}", r.host, r.owner, r.name))
            .collect();
        let accounts = discover(
            &[urls, hosts].concat(),
            &file,
            gitkraft_platforms::auth::resolve_token,
        );
        if accounts.is_empty() {
            return Err(anyhow!(
                "no accounts: set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or log in with `gh`"
            ));
        }
        let client = std::sync::Arc::new(CachedClient::new(
            ReqwestClient::new().map_err(|e| anyhow!(e.to_string()))?,
        ));
        let hub = Hub::new(accounts, &client);
        let mut engine = Engine::load();
        engine.status.mode = "daemon".into();
        let notifier = DesktopNotifier;
        let mut total = 0usize;
        loop {
            let config = load_config();
            let repos = watched(&config);
            let report = run_cycle(
                &hub,
                &mut engine,
                &repos,
                &config,
                &BTreeMap::<String, NotifySettings>::new(),
                Utc::now(),
                Local::now().time(),
                &notifier,
            )
            .await;
            total += report.alerts;
            engine.save()?;
            if once {
                return Ok(format!(
                    "polled {} project(s), {} alert(s), {} failed\n",
                    report.polled, total, report.failed
                ));
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
}

/// The directory the status files live in (for messages).
pub fn state_dir() -> Option<PathBuf> {
    files::state_dir().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Command, String> {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(p(&["status"]), Ok(Command::Status));
        assert_eq!(
            p(&["on"]),
            Ok(Command::Switch {
                on: true,
                repo: None,
                all: false
            })
        );
        assert_eq!(
            p(&["off", "github.com/o/r"]),
            Ok(Command::Switch {
                on: false,
                repo: Some("github.com/o/r".into()),
                all: false
            })
        );
        assert_eq!(
            p(&["on", "--all"]),
            Ok(Command::Switch {
                on: true,
                repo: None,
                all: true
            })
        );
        assert_eq!(
            p(&["mute", "2h"]),
            Ok(Command::Mute {
                for_secs: 7200,
                repo: None
            })
        );
        assert_eq!(
            p(&["mute", "30m", "codeberg.org/a/b"]),
            Ok(Command::Mute {
                for_secs: 1800,
                repo: Some("codeberg.org/a/b".into())
            })
        );
        assert_eq!(p(&["unmute"]), Ok(Command::Unmute { repo: None }));
        assert_eq!(p(&["test"]), Ok(Command::Test));
        assert_eq!(p(&["--once"]), Ok(Command::Once));
        assert_eq!(p(&["--daemon"]), Ok(Command::Daemon));
        assert_eq!(p(&["install"]), Ok(Command::Install));
    }

    #[test]
    fn bad_input_gets_an_explanation() {
        assert!(p(&[]).expect_err("usage").starts_with("usage"));
        assert!(p(&["mute"]).expect_err("duration").contains("duration"));
        assert!(
            p(&["mute", "soon"])
                .expect_err("bad")
                .contains("bad duration")
        );
        assert!(p(&["dance"]).expect_err("unknown").contains("unknown"));
    }

    #[test]
    fn status_table_lists_every_state_and_handles_empty() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).expect("ts");
        assert!(render_status(&[], "", now).contains("no projects"));
        let rows = vec![
            Row {
                key: "github.com/o/app".into(),
                status: NotifyStatus::On {
                    last_poll: Some(now),
                },
                interval: 60,
                unread: 2,
            },
            Row {
                key: "codeberg.org/a/b".into(),
                status: NotifyStatus::Off,
                interval: 60,
                unread: 0,
            },
            Row {
                key: "gitlab.com/g/p".into(),
                status: NotifyStatus::NoPoller,
                interval: 30,
                unread: 0,
            },
        ];
        let text = render_status(&rows, "daemon", now);
        assert!(text.starts_with("poller: daemon"));
        assert!(text.contains("[*] github.com/o/app") && text.contains("unread   2"));
        assert!(text.contains("[ ] codeberg.org/a/b") && text.contains("notifications off"));
        assert!(text.contains("no poller is running"));
        assert!(render_status(&rows, "", now).starts_with("poller: not running"));
    }

    #[test]
    fn service_files_run_the_daemon() {
        let (path, body) = service_unit("macos", "/usr/local/bin/gitkraft-tui").expect("macos");
        assert!(
            path.ends_with(".plist")
                && body.contains("<string>--daemon</string>")
                && body.contains("KeepAlive")
        );
        let (path, body) = service_unit("linux", "/usr/bin/gitkraft-tui").expect("linux");
        assert!(
            path.contains("systemd/user")
                && body.contains("ExecStart=/usr/bin/gitkraft-tui notify --daemon")
        );
        assert!(service_unit("windows", "x").is_none());
    }

    #[test]
    fn explicit_repos_are_validated_without_touching_git() {
        assert_eq!(
            resolve_repo(Some("GitHub.com/o/r".into())).expect("ok"),
            "github.com/o/r"
        );
        assert!(resolve_repo(Some("nope".into())).is_err());
    }
}
