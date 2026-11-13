//! Where the new config and state files live, and how they are written.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::persistence::settings_dir;

/// `notifications.toml`: layered notification settings.
pub fn notify_config_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("notifications.toml"))
}

/// `dashboard.toml`: the dashboard sections.
pub fn dashboard_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("dashboard.toml"))
}

/// `accounts.toml`: self-hosted platforms.
pub fn accounts_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("accounts.toml"))
}

/// Directory for state that is not config: poll snapshots, inbox, poller status.
pub fn state_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .context("could not determine data directory")?;
    Ok(base.join("gitkraft"))
}

/// `<state>/notify-state.json`.
pub fn notify_state_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("notify-state.json"))
}

/// `<state>/ci-memory.json`: last CI result per workflow and branch.
pub fn ci_memory_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("ci-memory.json"))
}

/// `<state>/notify-status.json`, written by the poller, read by the UIs.
pub fn notify_status_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("notify-status.json"))
}

/// `<state>/inbox.json`.
pub fn inbox_path() -> Result<PathBuf> {
    Ok(state_dir()?.join("inbox.json"))
}

/// Reads a text file; a missing file is `Ok(None)`.
pub fn read_optional(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

/// Writes `content` atomically (temp file in the same directory, then rename).
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    use std::io::Write as _;
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    let mut tmp =
        tempfile::NamedTempFile::new_in(parent).context("failed to create temporary file")?;
    tmp.write_all(content.as_bytes())
        .context("failed to write temporary file")?;
    tmp.persist(path)
        .map_err(|e| anyhow::anyhow!("failed to persist {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_is_atomic_and_creates_parents() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("a").join("b.toml");
        write_atomic(&path, "x = 1").expect("write");
        write_atomic(&path, "x = 2").expect("overwrite");
        assert_eq!(
            read_optional(&path).expect("read").as_deref(),
            Some("x = 2")
        );
        assert_eq!(
            std::fs::read_dir(path.parent().expect("parent"))
                .expect("dir")
                .count(),
            1,
            "no temp files left"
        );
    }

    #[test]
    fn missing_file_is_none_not_an_error() {
        let dir = tempfile::tempdir().expect("tmp");
        assert_eq!(read_optional(&dir.path().join("nope")).expect("read"), None);
    }

    #[test]
    fn paths_end_in_the_expected_names() {
        for (p, name) in [
            (notify_config_path(), "notifications.toml"),
            (dashboard_path(), "dashboard.toml"),
            (notify_state_path(), "notify-state.json"),
            (inbox_path(), "inbox.json"),
        ] {
            if let Ok(p) = p {
                assert_eq!(p.file_name().and_then(|n| n.to_str()), Some(name));
            }
        }
    }
}
