use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoHistoryEntry {
    pub path: PathBuf,
    pub display_name: String,
    pub last_opened: DateTime<Utc>,
}

/// Persisted pane layout dimensions so the UI restores exactly as the user left it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub sidebar_width: Option<f32>,
    pub commit_log_width: Option<f32>,
    pub staging_height: Option<f32>,
    pub diff_file_list_width: Option<f32>,
    pub sidebar_expanded: Option<bool>,
    /// UI scale factor (1.0 = 100%, 0.8 = 80%, 1.2 = 120%).
    #[serde(default)]
    pub ui_scale: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Last opened repository path.
    pub last_repo: Option<PathBuf>,
    /// Recently opened repositories, most recent first.
    pub recent_repos: Vec<RepoHistoryEntry>,
    /// Theme name (GUI) or theme index (TUI).
    pub theme_name: Option<String>,
    /// Max recent repos to store.
    #[serde(default = "default_max_recent")]
    pub max_recent: usize,
    /// Persisted pane layout dimensions.
    #[serde(default)]
    pub layout: Option<LayoutSettings>,
    /// Persisted editor name.
    #[serde(default)]
    pub editor_name: Option<String>,
    /// Paths of all tabs open when the app was last closed, in tab order.
    #[serde(default)]
    pub open_tabs: Vec<PathBuf>,
    /// Index of the active tab within open_tabs.
    #[serde(default)]
    pub active_tab_index: usize,
}

fn default_max_recent() -> usize {
    20
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            last_repo: None,
            recent_repos: Vec::new(),
            theme_name: None,
            max_recent: default_max_recent(),
            layout: None,
            editor_name: None,
            open_tabs: Vec::new(),
            active_tab_index: 0,
        }
    }
}

impl AppSettings {
    /// Add a repo to the recent list, moving it to the front if it already exists.
    /// Also sets it as the last_repo.
    pub fn add_recent_repo(&mut self, path: PathBuf) {
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Remove existing entry for this path
        self.recent_repos.retain(|e| e.path != path);

        // Insert at front
        self.recent_repos.insert(
            0,
            RepoHistoryEntry {
                path: path.clone(),
                display_name,
                last_opened: Utc::now(),
            },
        );

        // Trim to max
        if self.recent_repos.len() > self.max_recent {
            self.recent_repos.truncate(self.max_recent);
        }

        self.last_repo = Some(path);
    }
}
