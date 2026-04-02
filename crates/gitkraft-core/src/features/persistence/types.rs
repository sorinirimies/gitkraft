use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoHistoryEntry {
    pub path: PathBuf,
    pub display_name: String,
    pub last_opened: DateTime<Utc>,
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
