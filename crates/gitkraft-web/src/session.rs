//! What the server shares between requests: the hub, the dashboard and a short-lived cache.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use gitkraft_hub::poller::Engine;
use gitkraft_hub::{Hub, SectionData};
use gitkraft_platforms::DynClient;
use gitkraft_settings::dashboard::{Dashboard, Section};
use gitkraft_settings::files;
use gitkraft_settings::notify::NotifyConfig;
use gitkraft_settings::theme::{AppTheme, theme_by_index};

/// How long a loaded section is reused, so a page reload does not hit the platforms again.
const CACHE_FOR: Duration = Duration::from_secs(20);

/// 128 random bits as hex, from the operating system's seed of `RandomState`.
fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    (0..2)
        .map(|_| {
            let h = std::collections::hash_map::RandomState::new().build_hasher();
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// Where the server reads and writes; all `None` in tests.
#[derive(Debug, Clone, Default)]
pub struct Paths {
    /// `dashboard.toml`.
    pub dashboard: Option<PathBuf>,
    /// `notifications.toml`.
    pub notify: Option<PathBuf>,
    /// Directory of the poller's state files.
    pub use_state_dir: bool,
    /// The `~/.ssh` directory the SSH key page may change.
    pub ssh_dir: Option<PathBuf>,
    /// Local checkouts a pull request may be put in a worktree of (the one the server started in, plus `--checkout`).
    pub checkouts: Vec<PathBuf>,
    /// Where `ai-policy.toml` and `agent-watch.toml` live (default: the user's config directory).
    pub config_dir: Option<PathBuf>,
    /// Where agent-watch reports are filed; `None` keeps no reports (and the Agents page empty).
    pub agents_store: Option<PathBuf>,
}

impl Paths {
    /// The real files of the current user.
    pub fn user() -> Self {
        Self {
            dashboard: files::dashboard_path().ok(),
            notify: files::notify_config_path().ok(),
            use_state_dir: true,
            ssh_dir: Some(gitkraft_hub::keys::ssh_dir()),
            checkouts: vec![PathBuf::from(".")],
            config_dir: gitkraft_hub::commit_ai::global_dir(),
            agents_store: gitkraft_hub::agent_watch::Store::default_dir(),
        }
    }
}

struct Inner {
    hub: Hub<DynClient>,
    dashboard: RwLock<Dashboard>,
    paths: Paths,
    theme: usize,
    token: String,
    cache: Mutex<HashMap<usize, (Instant, SectionData)>>,
    /// The remotes of the first checkout (GitHub, Codeberg, Gitea, …).
    project: gitkraft_hub::project::Project,
    /// The Agents page: jobs, reports and the repository list.
    agents: Mutex<gitkraft_hub::agent_watch::AgentsState>,
}

/// Shared server state.
#[derive(Clone)]
pub struct Session {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("hub", &self.inner.hub)
            .finish()
    }
}

impl Session {
    /// A session over `hub`.
    pub fn new(hub: Hub<DynClient>, dashboard: Dashboard, paths: Paths, theme: usize) -> Self {
        let project = paths
            .checkouts
            .first()
            .map(|d| gitkraft_hub::project::Project::of_dir(d))
            .unwrap_or_default();
        let agents = match (&paths.config_dir, &paths.agents_store) {
            (Some(c), Some(s)) => {
                gitkraft_hub::agent_watch::AgentsState::in_dirs(c.clone(), s.clone())
            }
            (Some(c), None) => gitkraft_hub::agent_watch::AgentsState {
                dir: Some(c.clone()),
                ..Default::default()
            },
            _ => gitkraft_hub::agent_watch::AgentsState::default(),
        };
        Self {
            inner: Arc::new(Inner {
                hub,
                dashboard: RwLock::new(dashboard),
                paths,
                theme,
                token: new_token(),
                cache: Mutex::new(HashMap::new()),
                project,
                agents: Mutex::new(agents),
            }),
        }
    }

    /// The secret every page sends with an action. A page of another site cannot know it, so it cannot
    /// make this server act (cross-site request forgery).
    pub fn token(&self) -> &str {
        &self.inner.token
    }

    /// `~/.ssh` when this session may change it.
    pub fn ssh_dir(&self) -> Option<&std::path::Path> {
        self.inner.paths.ssh_dir.as_deref()
    }

    /// The local checkout of `repo`, if one of the known directories has a remote that points at it.
    pub fn checkout_of(&self, repo: &gitkraft_platforms::RepoId) -> Option<PathBuf> {
        let tabs: Vec<(Option<PathBuf>, Vec<String>)> = self
            .inner
            .paths
            .checkouts
            .iter()
            .map(|p| {
                (
                    p.canonicalize().ok(),
                    gitkraft_hub::accounts::remote_urls(p),
                )
            })
            .collect();
        gitkraft_hub::checkout::local_checkouts(&tabs)
            .into_iter()
            .find(|(r, _)| r == repo)
            .map(|(_, p)| p)
    }

    /// The AI policy: which hosts allow which drafts. Off until the user ran `ai-allow`.
    ///
    /// # Errors
    /// The policy file is unreadable.
    pub fn ai_policy(&self) -> Result<gitkraft_hub::review_ai::Policy, String> {
        match &self.inner.paths.config_dir {
            Some(d) => gitkraft_hub::review_ai::Policy::load(d),
            None => Ok(gitkraft_hub::review_ai::Policy::default()),
        }
    }

    /// The remotes of the checkout the server runs in.
    pub fn project(&self) -> &gitkraft_hub::project::Project {
        &self.inner.project
    }

    /// Where each remote stands against the checked-out branch (read from git on every call, so a
    /// fetch shows up without a restart).
    pub fn sync(&self) -> Vec<gitkraft_hub::project::RemoteSync> {
        self.inner
            .paths
            .checkouts
            .first()
            .map(|d| gitkraft_hub::project::sync_status(d, &self.inner.project))
            .unwrap_or_default()
    }

    /// The hub.
    pub fn hub(&self) -> &Hub<DynClient> {
        &self.inner.hub
    }

    /// A copy of the dashboard.
    pub fn dashboard(&self) -> Dashboard {
        self.inner
            .dashboard
            .read()
            .map(|d| d.clone())
            .unwrap_or_default()
    }

    /// The section at `index` (index into `dashboard.sections`).
    pub fn section(&self, index: usize) -> Option<Section> {
        self.inner
            .dashboard
            .read()
            .ok()?
            .sections
            .get(index)
            .cloned()
    }

    /// The saved colour theme.
    pub fn theme(&self) -> AppTheme {
        theme_by_index(self.inner.theme)
    }

    /// A section's data, from the cache while it is fresh.
    pub async fn section_data(&self, index: usize) -> Option<SectionData> {
        if let Some((at, data)) = self.inner.cache.lock().ok()?.get(&index)
            && at.elapsed() < CACHE_FOR
        {
            return Some(data.clone());
        }
        let section = self.section(index)?;
        let data = self.inner.hub.load_section(&section).await;
        if let Ok(mut cache) = self.inner.cache.lock() {
            cache.insert(index, (Instant::now(), data.clone()));
        }
        Some(data)
    }

    /// Forgets cached sections, after an action changed something.
    pub fn invalidate(&self) {
        if let Ok(mut cache) = self.inner.cache.lock() {
            cache.clear();
        }
    }

    /// The notification config, or the defaults when there is none.
    pub fn notify_config(&self) -> NotifyConfig {
        self.inner
            .paths
            .notify
            .as_deref()
            .and_then(|p| files::read_optional(p).ok().flatten())
            .and_then(|t| NotifyConfig::parse(&t).ok())
            .unwrap_or_default()
    }

    /// Path of `notifications.toml`, if this session may write it.
    pub fn notify_path(&self) -> Option<&std::path::Path> {
        self.inner.paths.notify.as_deref()
    }

    /// The poller's saved state (bell, inbox); empty when this session has none.
    pub fn engine(&self) -> Engine {
        if self.inner.paths.use_state_dir {
            Engine::load()
        } else {
            Engine::default()
        }
    }

    /// Persists the dashboard after an edit.
    pub fn save_dashboard(&self) -> Result<(), String> {
        let Some(path) = &self.inner.paths.dashboard else {
            return Ok(());
        };
        let text = self.dashboard().to_toml()?;
        files::write_atomic(path, &text).map_err(|e| e.to_string())
    }

    /// Reads or changes the Agents state; never held across an await.
    pub fn with_agents<R>(
        &self,
        f: impl FnOnce(&mut gitkraft_hub::agent_watch::AgentsState) -> R,
    ) -> R {
        match self.inner.agents.lock() {
            Ok(mut a) => f(&mut a),
            Err(p) => f(&mut p.into_inner()),
        }
    }

    /// The repositories every account can see, listed once and kept until `reload`.
    pub async fn agent_repos(&self, reload: bool) {
        let need = self.with_agents(|a| {
            if reload {
                a.repos = None;
            }
            a.repos.is_none() && !a.loading_repos
        });
        if !need {
            return;
        }
        self.with_agents(|a| a.loading_repos = true);
        let (repos, errors) = gitkraft_hub::agent_watch::scan::universe(&self.inner.hub).await;
        self.with_agents(|a| {
            a.loading_repos = false;
            a.repos = Some(repos);
            a.repo_errors = errors;
        });
    }

    /// Runs the named job now: scan, agent, report, and (for the real session) the inbox and a
    /// system notification. `quiet` is the scheduler asking.
    ///
    /// # Errors
    /// No such job, one already running, another process holds the lock, or the report cannot be written.
    pub async fn run_agent(&self, name: &str) -> Result<String, String> {
        let (job, store, dir, known) = self.with_agents(|a| {
            if let Some(r) = &a.running {
                return Err(format!("{r} is still running"));
            }
            let job = a.cfg.job(name).cloned().ok_or("no such job")?;
            let store = a.store.clone().ok_or("this session keeps no reports")?;
            let dir = a
                .dir
                .clone()
                .ok_or("this session has no config directory")?;
            a.running = Some(name.to_string());
            Ok((job, store, dir, a.repos.clone()))
        })?;
        let out =
            gitkraft_hub::agent_watch::app::run_in_app(&self.inner.hub, &job, known, &store, &dir)
                .await;
        self.with_agents(|a| {
            a.running = None;
            a.reload();
        });
        let out = out?;
        if self.inner.paths.use_state_dir {
            let mut e = Engine::load();
            gitkraft_hub::agent_watch::announce(
                &out,
                &mut e.inbox,
                &gitkraft_platforms::notify::desktop::DesktopNotifier,
                chrono::Utc::now(),
            );
            let _ = e.save();
        }
        Ok(format!("{name}: {}", out.report.line()))
    }

    /// The name of a job that is owed a run now, if the scheduler may start one.
    pub fn agent_owed(&self) -> Option<String> {
        self.with_agents(|a| {
            a.reload();
            if !a.auto || a.running.is_some() {
                return None;
            }
            a.owed(chrono::Local::now().naive_local())
        })
    }

    /// Whether this session is the user's real one (reports and notifications go to disk).
    pub fn is_live(&self) -> bool {
        self.inner.paths.use_state_dir
    }
}
