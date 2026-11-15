//! `gitkraft-tui agents …`: scheduled agents that check the CI of the projects you pick, on any
//! forge, and tell you when something is red. Config is `agent-watch.toml`, reports live under the
//! data directory, findings go to GitKraft's inbox and the system notification centre.
//!
//! ```text
//! agents list                        jobs, schedule, next run, last result
//! agents add NAME --schedule "mon-fri 07:30" --harness pi --visibility public --host github.com
//! agents edit NAME …                 change any of those
//! agents repos [NAME]                which repositories a job would watch (or everything visible)
//! agents run NAME [--dry-run]        run now; --dry-run only scans and prints
//! agents run-due                     run what is owed now (what the service calls every minute)
//! agents report NAME [--json]        the latest report
//! agents install | uninstall         the per-minute service (launchd, systemd)
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use chrono::Local;
use clap::{Args, Parser, Subcommand};
use gitkraft_hub::agent_watch::Deps;
use gitkraft_hub::agent_watch::harness::{Exec, SystemExec};
use gitkraft_hub::agent_watch::{
    AgentWatch, HarnessKind, Job, Store, Visibility, local, scan::universe,
};
use gitkraft_hub::review_ai::Policy;
use gitkraft_hub::{Hub, ai::Remote};
use gitkraft_platforms::DynClient;
use gitkraft_platforms::notify::{Notifier, desktop::DesktopNotifier};
use gitkraft_settings::files;

/// The `agents` command line.
#[derive(Debug, Parser)]
#[command(
    name = "gitkraft-tui agents",
    about = "Scheduled agents that watch your CI"
)]
pub struct AgentsCli {
    /// What to do.
    #[command(subcommand)]
    pub cmd: AgentsCmd,
}

/// The settings of a job; every one optional so `edit` changes only what is named.
#[derive(Debug, Args, Clone, Default)]
pub struct JobArgs {
    /// When: `daily 08:00`, `mon-fri 07:30`, `weekly`, `every 6h`.
    #[arg(long)]
    pub schedule: Option<String>,
    /// Who looks at red CI: builtin, pi, copilot, vscode or zed.
    #[arg(long)]
    pub harness: Option<String>,
    /// The model for the harness (`provider/id` for pi).
    #[arg(long)]
    pub model: Option<String>,
    /// Replace the built-in instructions.
    #[arg(long)]
    pub prompt: Option<String>,
    /// all, public or private.
    #[arg(long)]
    pub visibility: Option<String>,
    /// Only this host (repeat for several).
    #[arg(long = "host")]
    pub hosts: Vec<String>,
    /// Only repositories matching this glob on `host/owner/name` (repeat).
    #[arg(long = "include")]
    pub include: Vec<String>,
    /// Never these (repeat).
    #[arg(long = "exclude")]
    pub exclude: Vec<String>,
    /// Forget the include and exclude globs (before the ones given here are added).
    #[arg(long)]
    pub clear_globs: bool,
    /// Watch forks too (`--forks`, or `--forks false` to stop).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub forks: Option<bool>,
    /// Watch archived repositories too (`--archived`, or `--archived false`).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub archived: Option<bool>,
    /// An entry in GitKraft's inbox per finding (`--inbox false` for none).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub inbox: Option<bool>,
    /// A system notification per run (`--system false` for none).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    pub system: Option<bool>,
}

/// A job's name.
#[derive(Debug, Args, Clone)]
pub struct Named {
    /// The job.
    pub name: String,
}

/// The commands.
#[derive(Debug, Subcommand)]
pub enum AgentsCmd {
    /// Jobs with schedule, next run and last result.
    List {
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Create a job.
    Add {
        /// Name of the job.
        name: String,
        /// Its settings.
        #[command(flatten)]
        args: JobArgs,
    },
    /// Change a job; lists given here replace the old ones.
    Edit {
        /// Name of the job.
        name: String,
        /// What to change.
        #[command(flatten)]
        args: JobArgs,
    },
    /// Delete a job (its reports stay).
    Rm(Named),
    /// Switch a job on.
    Enable(Named),
    /// Switch a job off.
    Disable(Named),
    /// The repositories a job watches, or everything the accounts can see.
    Repos {
        /// The job (default: everything visible).
        name: Option<String>,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run a job now.
    Run {
        /// The job.
        name: String,
        /// Scan and print, without an agent, a report or a notification.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run every job that is owed a run (the service calls this every minute).
    RunDue,
    /// The recent reports of a job, newest first.
    Reports {
        /// The job.
        name: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// A report of a job (marks it seen): the latest unless `--nth` says which.
    Report {
        /// The job.
        name: String,
        /// 0 is the latest, 1 the one before.
        #[arg(long, default_value_t = 0)]
        nth: usize,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Write and load the per-minute service.
    Install,
    /// Remove the service.
    Uninstall,
}

/// The service files for `os`: relative path under the home directory and content.
#[must_use]
pub fn service_files(os: &str, exe: &str) -> Vec<(String, String)> {
    match os {
        "macos" => vec![(
            "Library/LaunchAgents/io.github.sorinirimies.gitkraft.agents.plist".into(),
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>io.github.sorinirimies.gitkraft.agents</string>\n  <key>ProgramArguments</key>\n  <array><string>{exe}</string><string>agents</string><string>run-due</string></array>\n  <key>StartInterval</key><integer>60</integer>\n  <key>RunAtLoad</key><true/>\n</dict>\n</plist>\n"
            ),
        )],
        "linux" => vec![
            (
                ".config/systemd/user/gitkraft-agents.service".into(),
                format!("[Unit]\nDescription=GitKraft agent-watch tick\n\n[Service]\nType=oneshot\nExecStart={exe} agents run-due\n"),
            ),
            (
                ".config/systemd/user/gitkraft-agents.timer".into(),
                "[Unit]\nDescription=GitKraft agent-watch every minute\n\n[Timer]\nOnBootSec=1min\nOnUnitActiveSec=1min\n\n[Install]\nWantedBy=timers.target\n".into(),
            ),
        ],
        _ => Vec::new(),
    }
}

/// Applies the named settings to `job`.
///
/// # Errors
/// An unknown harness or visibility word.
pub fn apply(a: &JobArgs, job: &mut Job) -> Result<()> {
    if let Some(s) = &a.schedule {
        job.schedule = s.clone();
    }
    if let Some(h) = &a.harness {
        job.harness = HarnessKind::parse(h)
            .ok_or_else(|| anyhow!("unknown harness `{h}`: builtin, pi, copilot, vscode or zed"))?;
    }
    if let Some(v) = &a.visibility {
        job.projects.visibility =
            Visibility::parse(v).ok_or_else(|| anyhow!("visibility is all, public or private"))?;
    }
    if a.model.is_some() {
        job.model = a.model.clone().filter(|m| !m.is_empty());
    }
    if a.prompt.is_some() {
        job.prompt = a.prompt.clone().filter(|m| !m.is_empty());
    }
    if a.clear_globs {
        job.projects.include.clear();
        job.projects.exclude.clear();
    }
    if !a.hosts.is_empty() {
        job.projects.hosts = a.hosts.clone();
    }
    if !a.include.is_empty() {
        job.projects.include = a.include.clone();
    }
    if !a.exclude.is_empty() {
        job.projects.exclude = a.exclude.clone();
    }
    if let Some(v) = a.forks {
        job.projects.forks = v;
    }
    if let Some(v) = a.archived {
        job.projects.archived = v;
    }
    if let Some(v) = a.inbox {
        job.notify.inbox = v;
    }
    if let Some(v) = a.system {
        job.notify.system = v;
    }
    Ok(())
}

fn config_dir() -> Result<PathBuf> {
    gitkraft_hub::commit_ai::global_dir().ok_or_else(|| anyhow!("no config directory"))
}

fn store() -> Result<Store> {
    Store::default_dir()
        .map(Store::new)
        .ok_or_else(|| anyhow!("no data directory"))
}

/// The text of `agents reports`: one line per report, numbered for `agents report --nth`.
#[must_use]
pub fn render_reports(all: &[gitkraft_hub::agent_watch::Report]) -> String {
    if all.is_empty() {
        return "no reports yet\n".into();
    }
    all.iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{i:>2} {} {}{}\n",
                r.started
                    .with_timezone(&chrono::Local)
                    .format("%a %d %b %H:%M"),
                if r.seen || !r.needs_you() { "" } else { "● " },
                r.line()
            )
        })
        .collect()
}

/// The text of `agents list`.
#[must_use]
pub fn render_list(cfg: &AgentWatch, store: &Store, now: chrono::NaiveDateTime) -> String {
    if cfg.jobs.is_empty() {
        return "no jobs yet: gitkraft-tui agents add NAME --schedule \"daily 08:00\"\n".into();
    }
    let mut out = String::new();
    for j in &cfg.jobs {
        let last = store.latest(&j.name);
        let next = j
            .parsed_schedule()
            .ok()
            .and_then(|s| s.next(now, last.as_ref().map(|r| local(r.started))))
            .map_or("-".to_string(), |t| t.format("%a %H:%M").to_string());
        out.push_str(&format!(
            "{} {:<14} {:<16} {:<8} next {:<10} {}\n   {}\n",
            if j.enabled { "●" } else { "○" },
            j.name,
            j.schedule,
            j.harness.name(),
            next,
            last.as_ref().map_or("never ran".to_string(), |r| format!(
                "{} {}",
                if r.seen || !r.needs_you() { "" } else { "● " },
                r.line()
            )),
            j.projects.describe()
        ));
    }
    out
}

async fn hub() -> Result<Hub<DynClient>> {
    let cwd = std::env::current_dir()?;
    let accounts =
        tokio::task::spawn_blocking(move || gitkraft_hub::accounts::discover_for(&cwd, &[]))
            .await?
            .map_err(|e| anyhow!(e))?;
    let client = DynClient::new(Arc::new(gitkraft_platforms::CachedClient::new(
        gitkraft_platforms::ReqwestClient::new().map_err(|e| anyhow!(e.to_string()))?,
    )));
    Ok(Hub::new(accounts, &client))
}

/// Runs `job` and files the report; the shared tail of `run` and `run-due`.
async fn execute(
    hub: &Hub<DynClient>,
    job: &Job,
    all: &[gitkraft_platforms::model::RepoInfo],
    store: &Store,
    exec: Arc<dyn Exec>,
    notifier: &dyn Notifier,
) -> Result<String> {
    let policy = Policy::load(&config_dir()?).map_err(|e| anyhow!(e))?;
    let llm: Option<Remote> = if job.harness == HarnessKind::Builtin {
        crate::ai_cli::real_llm().ok()
    } else {
        None
    };
    let deps = Deps {
        llm: llm.as_ref(),
        exec,
        policy: &policy,
        scratch: std::env::temp_dir(),
    };
    let out = gitkraft_hub::agent_watch::execute(
        hub,
        job,
        all,
        &deps,
        store,
        &files::inbox_path()?,
        notifier,
    )
    .await
    .map_err(|e| anyhow!(e))?;
    Ok(format!("{}: {}\n", job.name, out.report.line()))
}

/// Runs a command; returns the text to print.
///
/// # Errors
/// Bad arguments, a missing job, or a failed run.
pub async fn run(cli: AgentsCli) -> Result<String> {
    let dir = config_dir()?;
    let mut cfg = AgentWatch::load(&dir).map_err(|e| anyhow!(e))?;
    let now = Local::now().naive_local();
    match cli.cmd {
        AgentsCmd::List { json } => {
            let s = store()?;
            if json {
                let rows: Vec<_> = cfg
                    .jobs
                    .iter()
                    .map(|j| serde_json::json!({"job": j, "last": s.latest(&j.name)}))
                    .collect();
                return Ok(serde_json::to_string_pretty(&rows)? + "\n");
            }
            Ok(render_list(&cfg, &s, now))
        }
        AgentsCmd::Add { name, args } => {
            let mut job = Job::new(&name, args.schedule.as_deref().unwrap_or("daily 08:00"));
            apply(&args, &mut job)?;
            cfg.add(job).map_err(|e| anyhow!(e))?;
            cfg.save(&dir).map_err(|e| anyhow!(e))?;
            Ok(format!(
                "added {name}; `gitkraft-tui agents repos {name}` shows what it watches, `agents install` starts the service\n"
            ))
        }
        AgentsCmd::Edit { name, args } => {
            let job = cfg
                .job_mut(&name)
                .ok_or_else(|| anyhow!("no job `{name}`"))?;
            apply(&args, job)?;
            cfg.save(&dir).map_err(|e| anyhow!(e))?;
            Ok(format!("changed {name}\n"))
        }
        AgentsCmd::Rm(n) => {
            if !cfg.remove(&n.name) {
                bail!("no job `{}`", n.name);
            }
            cfg.save(&dir).map_err(|e| anyhow!(e))?;
            Ok(format!("removed {}\n", n.name))
        }
        AgentsCmd::Enable(n) => switch(&mut cfg, &dir, &n.name, true),
        AgentsCmd::Disable(n) => switch(&mut cfg, &dir, &n.name, false),
        AgentsCmd::Repos { name, json } => {
            let hub = hub().await?;
            let (all, errors) = universe(&hub).await;
            let picked: Vec<_> = match &name {
                Some(n) => cfg
                    .job(n)
                    .ok_or_else(|| anyhow!("no job `{n}`"))?
                    .projects
                    .pick(&all),
                None => all.iter().collect(),
            };
            if json {
                return Ok(serde_json::to_string_pretty(&picked)? + "\n");
            }
            let mut out = String::new();
            for i in &picked {
                out.push_str(&format!(
                    "{}  {}{}\n",
                    i.repo.key(),
                    if i.private { "private" } else { "public" },
                    if i.fork { " fork" } else { "" }
                ));
            }
            for e in errors {
                out.push_str(&format!("! {e}\n"));
            }
            out.push_str(&format!("{} of {} repositories\n", picked.len(), all.len()));
            Ok(out)
        }
        AgentsCmd::Run { name, dry_run } => {
            let job = cfg
                .job(&name)
                .ok_or_else(|| anyhow!("no job `{name}`"))?
                .clone();
            let hub = hub().await?;
            let (all, errors) = universe(&hub).await;
            if dry_run {
                let picked = job.projects.pick(&all);
                let s = gitkraft_hub::agent_watch::scan::scan(&hub, &picked).await;
                let mut out = format!(
                    "{} repos, {} workflows, {} red\n",
                    s.repos,
                    s.workflows,
                    s.findings.len()
                );
                for f in &s.findings {
                    out.push_str(&format!("  ✘ {} {}\n", f.headline(), f.url));
                }
                for e in s.errors.iter().chain(&errors) {
                    out.push_str(&format!("  ! {e}\n"));
                }
                return Ok(out);
            }
            let _lock = store()?
                .try_lock()
                .ok_or_else(|| anyhow!("another run is in progress"))?;
            execute(
                &hub,
                &job,
                &all,
                &store()?,
                Arc::new(SystemExec),
                &DesktopNotifier,
            )
            .await
        }
        AgentsCmd::RunDue => {
            let s = store()?;
            let due: Vec<Job> = gitkraft_hub::agent_watch::due(&cfg, &s, now)
                .into_iter()
                .cloned()
                .collect();
            if due.is_empty() {
                return Ok(String::new());
            }
            let Some(_lock) = s.try_lock() else {
                return Ok(String::new());
            };
            let hub = hub().await?;
            let (all, _) = universe(&hub).await;
            let mut out = String::new();
            for j in &due {
                out.push_str(
                    &execute(&hub, j, &all, &s, Arc::new(SystemExec), &DesktopNotifier).await?,
                );
            }
            Ok(out)
        }
        AgentsCmd::Reports { name, json } => {
            let all = store()?.reports(&name);
            if json {
                return Ok(serde_json::to_string_pretty(&all)? + "\n");
            }
            Ok(render_reports(&all))
        }
        AgentsCmd::Report { name, nth, json } => {
            let s = store()?;
            let r = s
                .reports(&name)
                .into_iter()
                .nth(nth)
                .ok_or_else(|| anyhow!("`{name}` has no report number {nth}"))?;
            s.mark_seen(&name).map_err(|e| anyhow!(e))?;
            if json {
                return Ok(serde_json::to_string_pretty(&r)? + "\n");
            }
            Ok(r.markdown())
        }
        AgentsCmd::Install => install(),
        AgentsCmd::Uninstall => uninstall(),
    }
}

fn switch(cfg: &mut AgentWatch, dir: &std::path::Path, name: &str, on: bool) -> Result<String> {
    let job = cfg
        .job_mut(name)
        .ok_or_else(|| anyhow!("no job `{name}`"))?;
    job.enabled = on;
    cfg.save(dir).map_err(|e| anyhow!(e))?;
    Ok(format!(
        "{} {name}\n",
        if on { "enabled" } else { "disabled" }
    ))
}

fn install() -> Result<String> {
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let files = service_files(os, &exe);
    if files.is_empty() {
        return Ok(format!(
            "run `{exe} agents run-due` every minute (Windows: schtasks /Create /SC MINUTE /TN GitKraftAgents /TR \"{exe} agents run-due\")\n"
        ));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home directory"))?;
    let mut out = String::new();
    for (rel, body) in &files {
        let p = home.join(rel);
        files::write_atomic(&p, body)?;
        out.push_str(&format!("wrote {}\n", p.display()));
    }
    out.push_str(&if os == "macos" {
        format!(
            "load it with: launchctl load {}\n",
            home.join(&files[0].0).display()
        )
    } else {
        "enable it with: systemctl --user enable --now gitkraft-agents.timer\n".to_string()
    });
    Ok(out)
}

fn uninstall() -> Result<String> {
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let home = dirs::home_dir().ok_or_else(|| anyhow!("no home directory"))?;
    let mut out = String::new();
    for (rel, _) in service_files(std::env::consts::OS, &exe) {
        let p = home.join(rel);
        if p.exists() {
            std::fs::remove_file(&p)?;
            out.push_str(&format!("removed {}\n", p.display()));
        }
    }
    if out.is_empty() {
        out.push_str("no service files\n");
    }
    out.push_str("unload it first with launchctl unload / systemctl --user disable --now gitkraft-agents.timer if it is running\n");
    Ok(out)
}

/// Parses `args` (what follows `agents`).
///
/// # Errors
/// clap's usage message.
pub fn parse(args: &[String]) -> Result<AgentsCli, String> {
    AgentsCli::try_parse_from(std::iter::once("agents".to_string()).chain(args.iter().cloned()))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> AgentsCli {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()).expect("parse")
    }

    #[test]
    fn add_takes_every_setting_and_apply_fills_the_job() {
        let AgentsCmd::Add { name, args } = p(&[
            "add",
            "pub",
            "--schedule",
            "mon-fri 07:30",
            "--harness",
            "pi",
            "--model",
            "m/x",
            "--visibility",
            "public",
            "--host",
            "github.com",
            "--host",
            "codeberg.org",
            "--include",
            "github.com/me/*",
            "--exclude",
            "**/old",
            "--system",
            "false",
            "--forks",
        ])
        .cmd
        else {
            panic!("add")
        };
        let mut job = Job::new(&name, "daily 08:00");
        apply(&args, &mut job).expect("apply");
        assert_eq!(job.schedule, "mon-fri 07:30");
        assert_eq!(job.harness, HarnessKind::Pi);
        assert_eq!(job.projects.visibility, Visibility::Public);
        assert_eq!(job.projects.hosts, ["github.com", "codeberg.org"]);
        assert!(job.notify.inbox && !job.notify.system && job.projects.forks);
    }

    #[test]
    fn switches_turn_on_and_off_and_globs_can_be_cleared() {
        let mut job = Job::new("j", "daily 08:00");
        job.projects.include = vec!["a/*".into()];
        job.projects.forks = true;
        let AgentsCmd::Edit { args, .. } = p(&[
            "edit",
            "j",
            "--forks",
            "false",
            "--archived",
            "--inbox",
            "false",
            "--clear-globs",
            "--include",
            "b/*",
        ])
        .cmd
        else {
            panic!("edit")
        };
        apply(&args, &mut job).expect("apply");
        assert!(!job.projects.forks && job.projects.archived && !job.notify.inbox);
        assert_eq!(job.projects.include, ["b/*"]);
    }

    #[test]
    fn reports_are_listed_newest_first_with_a_number() {
        assert_eq!(render_reports(&[]), "no reports yet\n");
        let AgentsCmd::Report { nth, .. } = p(&["report", "j", "--nth", "2"]).cmd else {
            panic!("report")
        };
        assert_eq!(nth, 2);
    }

    #[test]
    fn an_edit_changes_only_what_is_named_and_unknown_words_are_refused() {
        let mut job = Job::new("j", "daily 08:00");
        job.projects.hosts = vec!["git.lan".into()];
        let AgentsCmd::Edit { args, .. } = p(&["edit", "j", "--harness", "zed"]).cmd else {
            panic!("edit")
        };
        apply(&args, &mut job).expect("apply");
        assert_eq!(
            (job.harness, job.projects.hosts.as_slice()),
            (HarnessKind::Zed, &["git.lan".to_string()][..])
        );
        let AgentsCmd::Edit { args, .. } = p(&["edit", "j", "--harness", "emacs"]).cmd else {
            panic!("edit")
        };
        assert!(apply(&args, &mut job).is_err_and(|e| e.to_string().contains("unknown harness")));
        let AgentsCmd::Edit { args, .. } = p(&["edit", "j", "--visibility", "secret"]).cmd else {
            panic!("edit")
        };
        assert!(apply(&args, &mut job).is_err());
    }

    #[test]
    fn the_service_ticks_every_minute_on_each_os() {
        let mac = service_files("macos", "/bin/gk");
        assert!(mac[0].0.ends_with("gitkraft.agents.plist"));
        assert!(
            mac[0].1.contains("<string>run-due</string>")
                && mac[0].1.contains("<integer>60</integer>")
        );
        let linux = service_files("linux", "/bin/gk");
        assert_eq!(linux.len(), 2);
        assert!(
            linux[0].1.contains("ExecStart=/bin/gk agents run-due")
                && linux[1].1.contains("OnUnitActiveSec=1min")
        );
        assert!(service_files("windows", "x").is_empty());
    }

    #[test]
    fn the_list_shows_state_next_run_and_what_a_job_watches() {
        let tmp = tempfile::tempdir().expect("tmp");
        let store = Store::new(tmp.path());
        assert!(
            render_list(&AgentWatch::default(), &store, Local::now().naive_local())
                .contains("no jobs yet")
        );
        let mut cfg = AgentWatch::default();
        let mut j = Job::new("pub", "mon-fri 07:30");
        j.projects.visibility = Visibility::Public;
        j.enabled = false;
        cfg.add(j).expect("add");
        let now =
            chrono::NaiveDateTime::parse_from_str("2026-03-06 09:00", "%Y-%m-%d %H:%M").expect("t");
        let text = render_list(&cfg, &store, now);
        assert!(
            text.contains("○ pub") && text.contains("next Mon 07:30") && text.contains("never ran"),
            "{text}"
        );
        assert!(text.contains("all forges · public"), "{text}");
    }
}
