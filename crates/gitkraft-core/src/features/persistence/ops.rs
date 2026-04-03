//! Persistence operations — load, save, and query application settings
//! backed by a redb embedded key-value database.

use super::types::{AppSettings, RepoHistoryEntry};
use anyhow::{Context, Result};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};

const SETTINGS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("settings");
const RECENT_REPOS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("recent_repos");

/// Get the settings directory (~/.config/gitkraft/ or platform equivalent).
pub fn settings_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine config directory")?;
    Ok(base.join("gitkraft"))
}

/// Full path to the database file.
fn db_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("gitkraft.redb"))
}

/// Open or create the database.
fn open_db() -> Result<Database> {
    let dir = settings_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create settings directory {}", dir.display()))?;
    let path = db_path()?;
    Database::create(&path)
        .with_context(|| format!("failed to open database at {}", path.display()))
}

/// Load settings from the database. Returns default settings if the database
/// doesn't exist or any table is missing.
pub fn load_settings() -> Result<AppSettings> {
    let db = match open_db() {
        Ok(db) => db,
        Err(_) => return Ok(AppSettings::default()),
    };

    let read_txn = db.begin_read()?;
    let mut settings = AppSettings::default();

    // Read scalar settings
    if let Ok(table) = read_txn.open_table(SETTINGS_TABLE) {
        if let Ok(Some(val)) = table.get("last_repo") {
            settings.last_repo = Some(PathBuf::from(val.value()));
        }
        if let Ok(Some(val)) = table.get("theme_name") {
            settings.theme_name = Some(val.value().to_string());
        }
        if let Ok(Some(val)) = table.get("layout") {
            if let Ok(layout) = serde_json::from_str::<super::types::LayoutSettings>(val.value()) {
                settings.layout = Some(layout);
            }
        }
    }

    // Read recent repos
    if let Ok(table) = read_txn.open_table(RECENT_REPOS_TABLE) {
        let mut entries: Vec<RepoHistoryEntry> = Vec::new();
        if let Ok(iter) = table.iter() {
            for (_key, value) in iter.flatten() {
                if let Ok(entry) = serde_json::from_slice::<RepoHistoryEntry>(value.value()) {
                    entries.push(entry);
                }
            }
        }
        // Sort by last_opened descending (most recent first)
        entries.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        settings.recent_repos = entries;
    }

    Ok(settings)
}

/// Save settings to the database. Creates the database and tables if needed.
pub fn save_settings(settings: &AppSettings) -> Result<()> {
    let db = open_db()?;
    let write_txn = db.begin_write()?;

    // Write scalar settings
    {
        let mut table = write_txn.open_table(SETTINGS_TABLE)?;
        if let Some(ref path) = settings.last_repo {
            table.insert("last_repo", path.to_string_lossy().as_ref())?;
        }
        if let Some(ref theme) = settings.theme_name {
            table.insert("theme_name", theme.as_str())?;
        }
        if let Some(ref layout) = settings.layout {
            let layout_json =
                serde_json::to_string(layout).context("failed to serialize layout settings")?;
            table.insert("layout", layout_json.as_str())?;
        }
    }

    // Write recent repos — clear existing entries then rewrite
    {
        let mut table = write_txn.open_table(RECENT_REPOS_TABLE)?;

        // Collect existing keys so we can remove them
        let existing_keys: Vec<String> = {
            let iter = table.iter()?;
            iter.filter_map(|e| e.ok().map(|(k, _)| k.value().to_string()))
                .collect()
        };
        for key in &existing_keys {
            table.remove(key.as_str())?;
        }

        // Insert current entries (path string is the key)
        for entry in &settings.recent_repos {
            let key = entry.path.to_string_lossy();
            let value =
                serde_json::to_vec(entry).context("failed to serialize repo history entry")?;
            table.insert(key.as_ref(), value.as_slice())?;
        }
    }

    write_txn.commit()?;
    Ok(())
}

/// Convenience: record that a repo was opened (updates history + last_repo).
pub fn record_repo_opened(path: &Path) -> Result<()> {
    let mut settings = load_settings()?;
    settings.add_recent_repo(path.to_path_buf());
    save_settings(&settings)
}

/// Convenience: get the last opened repo path.
pub fn get_last_repo() -> Result<Option<PathBuf>> {
    let settings = load_settings()?;
    Ok(settings.last_repo)
}

/// Convenience: save theme preference.
pub fn save_theme(theme_name: &str) -> Result<()> {
    let mut settings = load_settings()?;
    settings.theme_name = Some(theme_name.to_string());
    save_settings(&settings)
}

/// Convenience: get saved theme name.
pub fn get_saved_theme() -> Result<Option<String>> {
    let settings = load_settings()?;
    Ok(settings.theme_name)
}

/// Convenience: save layout preferences.
pub fn save_layout(layout: &super::types::LayoutSettings) -> Result<()> {
    let mut settings = load_settings()?;
    settings.layout = Some(layout.clone());
    save_settings(&settings)
}

/// Convenience: get saved layout preferences.
pub fn get_saved_layout() -> Result<Option<super::types::LayoutSettings>> {
    let settings = load_settings()?;
    Ok(settings.layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_recent_deduplicates() {
        let mut settings = AppSettings::default();
        settings.add_recent_repo(PathBuf::from("/tmp/repo1"));
        settings.add_recent_repo(PathBuf::from("/tmp/repo2"));
        settings.add_recent_repo(PathBuf::from("/tmp/repo1"));
        assert_eq!(settings.recent_repos.len(), 2);
        assert_eq!(settings.recent_repos[0].path, PathBuf::from("/tmp/repo1"));
    }

    #[test]
    fn add_recent_respects_max() {
        let mut settings = AppSettings {
            max_recent: 3,
            ..Default::default()
        };
        for i in 0..5 {
            settings.add_recent_repo(PathBuf::from(format!("/tmp/repo{i}")));
        }
        assert_eq!(settings.recent_repos.len(), 3);
    }

    #[test]
    fn settings_round_trip() {
        // Test the serde round-trip of RepoHistoryEntry (the encoding used inside redb values)
        let mut settings = AppSettings::default();
        settings.add_recent_repo(PathBuf::from("/tmp/repo1"));
        settings.add_recent_repo(PathBuf::from("/tmp/repo2"));
        settings.theme_name = Some("Dark".to_string());

        let entry = &settings.recent_repos[0];
        let bytes = serde_json::to_vec(entry).unwrap();
        let decoded: RepoHistoryEntry = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.path, entry.path);
        assert_eq!(decoded.display_name, entry.display_name);
    }
}
