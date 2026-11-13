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

/// Where the server reads and writes; all `None` in tests.
#[derive(Debug, Clone, Default)]
pub struct Paths {
    /// `dashboard.toml`.
    pub dashboard: Option<PathBuf>,
    /// `notifications.toml`.
    pub notify: Option<PathBuf>,
    /// Directory of the poller's state files.
    pub use_state_dir: bool,
}

impl Paths {
    /// The real files of the current user.
    pub fn user() -> Self {
        Self {
            dashboard: files::dashboard_path().ok(),
            notify: files::notify_config_path().ok(),
            use_state_dir: true,
        }
    }
}

struct Inner {
    hub: Hub<DynClient>,
    dashboard: RwLock<Dashboard>,
    paths: Paths,
    theme: usize,
    cache: Mutex<HashMap<usize, (Instant, SectionData)>>,
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
        Self {
            inner: Arc::new(Inner {
                hub,
                dashboard: RwLock::new(dashboard),
                paths,
                theme,
                cache: Mutex::new(HashMap::new()),
            }),
        }
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
}
