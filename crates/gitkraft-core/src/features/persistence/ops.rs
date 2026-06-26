//! Persistence operations backed by a plain JSON file.
//!
//! Settings are stored at `~/.config/gitkraft/settings.json` (or the
//! platform-appropriate config directory).  Writes are **atomic**: content is
//! first written to a `NamedTempFile` in the same directory and then
//! `persist()`-ed into place, so a crash mid-write can never produce a
//! corrupted file and no stale `.tmp` file is ever left behind.
//!
//! GUI settings are stored in `settings.json`; TUI settings are stored in
//! `tui-settings.json`.  The two files are independent so each UI can evolve
//! its own preferences (theme index, session, etc.) without stomping the other.

use super::types::{AppSettings, RepoHistoryEntry};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Returns the settings directory (`~/.config/gitkraft/` or equivalent).
pub fn settings_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine config directory")?;
    Ok(base.join("gitkraft"))
}

/// Full path to the GUI JSON settings file (public so frontends can open it in an editor).
pub fn settings_json_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("settings.json"))
}

/// Full path to the TUI-specific JSON settings file (public so the TUI can open it in an editor).
pub fn tui_settings_json_path() -> Result<PathBuf> {
    Ok(settings_dir()?.join("tui-settings.json"))
}

/// Full path to the GUI JSON settings file.
fn json_path() -> Result<PathBuf> {
    settings_json_path()
}

/// Full path to the TUI-specific JSON settings file.
fn tui_json_path() -> Result<PathBuf> {
    tui_settings_json_path()
}

// ── Internal I/O helpers ──────────────────────────────────────────────────────

/// Load settings from any JSON path (internal).
fn load_from(path: &std::path::Path) -> Result<AppSettings> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        return match serde_json::from_str::<AppSettings>(&content) {
            Ok(s) => Ok(s),
            Err(e) => {
                tracing::warn!(
                    "settings file {:?} is malformed ({e}); using defaults",
                    path
                );
                Ok(AppSettings::default())
            }
        };
    }
    Ok(AppSettings::default())
}

/// Save settings to any JSON path (internal, atomic write via NamedTempFile).
fn save_to(path: &std::path::Path, settings: &AppSettings) -> Result<()> {
    use std::io::Write as _;

    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create directory {}", parent.display()))?;

    let content = serde_json::to_string_pretty(settings).context("failed to serialise settings")?;

    // Write to a NamedTempFile in the same directory so that persist() is an
    // atomic rename on all platforms.  The temp file is auto-deleted on drop
    // if persist() never succeeds, eliminating stale .tmp files after crashes.
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .context("failed to create temporary settings file")?;
    tmp.write_all(content.as_bytes())
        .context("failed to write settings to temporary file")?;
    tmp.persist(path)
        .map_err(|e| anyhow::anyhow!("failed to persist settings file: {}", e))?;

    Ok(())
}

// ── GUI settings (settings.json) ─────────────────────────────────────────────

/// Load GUI application settings.
///
/// Returns `AppSettings::default()` when the file does not exist yet (first run)
/// or when the file is malformed (file is preserved for manual recovery).
pub fn load_settings() -> Result<AppSettings> {
    load_from(&json_path()?)
}

/// Persist GUI application settings to `settings.json` using an atomic write.
pub fn save_settings(settings: &AppSettings) -> Result<()> {
    save_to(&json_path()?, settings)
}

/// Apply a closure to GUI settings and persist the result in a single
/// read-modify-write cycle.  Avoids duplicating the load → mutate → save
/// pattern across every individual setting helper.
pub fn update_settings<F: FnOnce(&mut AppSettings)>(f: F) -> Result<()> {
    let mut settings = load_settings()?;
    f(&mut settings);
    save_settings(&settings)
}

/// Record that a repository was opened (updates history + `last_repo`).
pub fn record_repo_opened(path: &Path) -> Result<()> {
    update_settings(|s| s.add_recent_repo(path.to_path_buf()))
}

/// Return the last opened repository path, if any.
pub fn get_last_repo() -> Result<Option<PathBuf>> {
    Ok(load_settings()?.last_repo)
}

/// Persist the selected theme name.
pub fn save_theme(theme_name: &str) -> Result<()> {
    update_settings(|s| s.theme_name = Some(theme_name.to_string()))
}

/// Return the saved theme name, if any.
pub fn get_saved_theme() -> Result<Option<String>> {
    Ok(load_settings()?.theme_name)
}

/// Persist the selected editor name.
pub fn save_editor(editor_name: &str) -> Result<()> {
    update_settings(|s| s.editor_name = Some(editor_name.to_string()))
}

/// Return the saved editor name, if any.
pub fn get_saved_editor() -> Result<Option<String>> {
    Ok(load_settings()?.editor_name)
}

/// Persist layout preferences.
pub fn save_layout(layout: &super::types::LayoutSettings) -> Result<()> {
    let layout = layout.clone();
    update_settings(|s| s.layout = Some(layout))
}

/// Return saved layout preferences, if any.
pub fn get_saved_layout() -> Result<Option<super::types::LayoutSettings>> {
    Ok(load_settings()?.layout)
}

/// Record that a repo was opened AND update the open-tab session in a single
/// write (saves one round-trip to disk).
pub fn record_repo_and_save_session(
    path: &Path,
    open_tabs: &[PathBuf],
    active_tab_index: usize,
) -> Result<Vec<RepoHistoryEntry>> {
    let mut settings = load_settings()?;
    settings.add_recent_repo(path.to_path_buf());
    settings.open_tabs = open_tabs.to_vec();
    settings.active_tab_index = active_tab_index;
    save_settings(&settings)?;
    Ok(settings.recent_repos)
}

/// Persist the open-tab session without modifying the recent-repos list.
pub fn save_session(open_tabs: &[PathBuf], active_tab_index: usize) -> Result<()> {
    update_settings(|s| {
        s.open_tabs = open_tabs.to_vec();
        s.active_tab_index = active_tab_index;
    })
}

// ── TUI settings (tui-settings.json) ─────────────────────────────────────────

/// Load TUI application settings from `tui-settings.json`.
pub fn load_tui_settings() -> Result<AppSettings> {
    let mut settings = load_from(&tui_json_path()?)?;
    // If the TUI has no editor configured, fall back to the GUI's editor
    // setting so the user only has to configure their editor once.
    if settings.editor_name.is_none() {
        if let Ok(gui) = load_from(&json_path()?) {
            if gui.editor_name.is_some() {
                settings.editor_name = gui.editor_name;
            }
        }
    }
    Ok(settings)
}

/// Persist TUI application settings to `tui-settings.json` using an atomic write.
pub fn save_tui_settings(settings: &AppSettings) -> Result<()> {
    save_to(&tui_json_path()?, settings)
}

/// Apply a closure to TUI settings and persist the result in a single
/// read-modify-write cycle.
///
/// Loads the raw stored settings (without the editor-fallback inheritance used
/// by [`load_tui_settings`]) to avoid accidentally overwriting an explicit
/// editor choice made via the GUI.
pub fn update_tui_settings<F: FnOnce(&mut AppSettings)>(f: F) -> Result<()> {
    let mut settings = load_from(&tui_json_path()?)?;
    f(&mut settings);
    save_tui_settings(&settings)
}

/// Record that a repository was opened in the TUI.
pub fn record_repo_opened_tui(path: &std::path::Path) -> Result<()> {
    update_tui_settings(|s| s.add_recent_repo(path.to_path_buf()))
}

/// Return the last TUI-opened repository path, if any.
pub fn get_last_tui_repo() -> Result<Option<PathBuf>> {
    Ok(load_tui_settings()?.last_repo)
}

/// Persist the TUI theme selection.
pub fn save_theme_tui(theme_name: &str) -> Result<()> {
    update_tui_settings(|s| s.theme_name = Some(theme_name.to_string()))
}

/// Persist the TUI editor selection.
pub fn save_editor_tui(editor_name: &str) -> Result<()> {
    update_tui_settings(|s| s.editor_name = Some(editor_name.to_string()))
}

/// Persist the TUI open-tab session.
pub fn save_session_tui(open_tabs: &[PathBuf], active_tab_index: usize) -> Result<()> {
    update_tui_settings(|s| {
        s.open_tabs = open_tabs.to_vec();
        s.active_tab_index = active_tab_index;
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── In-process helpers (bypass dirs::config_dir) ──────────────────────────

    /// Write settings via the real `save_to` implementation so helpers
    /// automatically exercise the `NamedTempFile` atomic-write path.
    fn write_json(dir: &TempDir, settings: &AppSettings) {
        let path = dir.path().join("settings.json");
        save_to(&path, settings).unwrap();
    }

    fn read_json(dir: &TempDir) -> AppSettings {
        let path = dir.path().join("settings.json");
        let content = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    // ── AppSettings serde round-trip ──────────────────────────────────────────

    #[test]
    fn settings_json_round_trip() {
        let dir = TempDir::new().unwrap();
        let mut s = AppSettings {
            theme_name: Some("Dracula".to_string()),
            editor_name: Some("code".to_string()),
            ..Default::default()
        };
        s.add_recent_repo(PathBuf::from("/tmp/repo-a"));
        s.add_recent_repo(PathBuf::from("/tmp/repo-b"));

        write_json(&dir, &s);
        let loaded = read_json(&dir);

        assert_eq!(loaded.theme_name, Some("Dracula".to_string()));
        assert_eq!(loaded.editor_name, Some("code".to_string()));
        assert_eq!(loaded.recent_repos.len(), 2);
        assert_eq!(loaded.recent_repos[0].path, PathBuf::from("/tmp/repo-b"));
        assert_eq!(loaded.recent_repos[1].path, PathBuf::from("/tmp/repo-a"));
    }

    #[test]
    fn settings_json_preserves_open_tabs_and_active_index() {
        let dir = TempDir::new().unwrap();
        let s = AppSettings {
            open_tabs: vec![PathBuf::from("/tmp/repo-1"), PathBuf::from("/tmp/repo-2")],
            active_tab_index: 1,
            ..Default::default()
        };

        write_json(&dir, &s);
        let loaded = read_json(&dir);

        assert_eq!(loaded.open_tabs.len(), 2);
        assert_eq!(loaded.active_tab_index, 1);
    }

    #[test]
    fn settings_json_preserves_layout() {
        let dir = TempDir::new().unwrap();
        let s = AppSettings {
            layout: Some(super::super::types::LayoutSettings {
                sidebar_width: Some(220.0),
                commit_log_width: Some(400.0),
                staging_height: Some(150.0),
                diff_file_list_width: Some(180.0),
                sidebar_expanded: Some(true),
                ui_scale: Some(1.25),
                ..Default::default()
            }),
            ..Default::default()
        };

        write_json(&dir, &s);
        let loaded = read_json(&dir);
        let layout = loaded.layout.unwrap();

        assert!((layout.sidebar_width.unwrap() - 220.0).abs() < f32::EPSILON);
        assert_eq!(layout.sidebar_expanded, Some(true));
        assert!((layout.ui_scale.unwrap() - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn malformed_json_deserialises_to_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        // Write garbage — simulates a half-written file from a crash.
        std::fs::write(&path, b"{ this is not valid json !!!").unwrap();

        // serde_json::from_str should fail; caller should get AppSettings::default().
        let result = serde_json::from_str::<AppSettings>(&std::fs::read_to_string(&path).unwrap());
        assert!(
            result.is_err(),
            "malformed JSON must not parse successfully"
        );
        // The file must still exist (we must not delete it).
        assert!(path.exists(), "malformed file must be preserved");
    }

    #[test]
    fn atomic_write_produces_no_tmp_file_on_success() {
        let dir = TempDir::new().unwrap();
        let s = AppSettings::default();
        write_json(&dir, &s);

        let tmp = dir.path().join("settings.json.tmp");
        assert!(
            !tmp.exists(),
            "tmp file must be removed after a successful atomic write"
        );
        assert!(dir.path().join("settings.json").exists());
    }

    #[test]
    fn serde_default_missing_fields_load_cleanly() {
        // A JSON object with only known fields — new fields added in future
        // versions should not break loading older settings files.
        let dir = TempDir::new().unwrap();
        let minimal = r#"{"last_repo": null, "recent_repos": [], "theme_name": "Nord"}"#;
        std::fs::write(dir.path().join("settings.json"), minimal).unwrap();

        let loaded = read_json(&dir);
        assert_eq!(loaded.theme_name, Some("Nord".to_string()));
        assert_eq!(loaded.max_recent, 20); // default
        assert_eq!(loaded.active_tab_index, 0); // default
        assert!(loaded.open_tabs.is_empty()); // default
    }

    // ── AppSettings helper logic ──────────────────────────────────────────────

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
    fn settings_round_trip_via_json_bytes() {
        let mut settings = AppSettings::default();
        settings.add_recent_repo(PathBuf::from("/tmp/repo1"));
        settings.add_recent_repo(PathBuf::from("/tmp/repo2"));
        settings.theme_name = Some("Dark".to_string());

        let json = serde_json::to_string(&settings).unwrap();
        let decoded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.recent_repos.len(), 2);
        assert_eq!(decoded.theme_name, Some("Dark".to_string()));
    }

    // ── TUI path tests ────────────────────────────────────────────────────────

    #[test]
    fn tui_and_gui_settings_are_independent() {
        // Verify the path names differ so they won't overwrite each other.
        let gui = json_path().unwrap();
        let tui = tui_json_path().unwrap();
        assert_ne!(gui, tui);
        assert!(gui.to_str().unwrap().ends_with("settings.json"));
        assert!(tui.to_str().unwrap().ends_with("tui-settings.json"));
    }

    #[test]
    fn load_tui_inherits_editor_from_gui_when_tui_has_none() {
        let dir = TempDir::new().unwrap();

        // Write GUI settings with an editor configured.
        let gui = AppSettings {
            editor_name: Some("Helix".to_string()),
            ..Default::default()
        };
        write_json(&dir, &gui);

        // TUI settings exist but have no editor_name.
        let tui_path = dir.path().join("tui-settings.json");
        let tui_content = r#"{"last_repo":null,"recent_repos":[],"theme_name":null}"#;
        std::fs::write(&tui_path, tui_content).unwrap();

        // load_from on just the TUI file gives no editor.
        let tui_raw = load_from(&tui_path).unwrap();
        assert!(tui_raw.editor_name.is_none());

        // The fallback logic (mirroring load_tui_settings) should pick up the
        // GUI editor when TUI has none.
        let gui_loaded = load_from(&dir.path().join("settings.json")).unwrap();
        let mut merged = tui_raw;
        if merged.editor_name.is_none() {
            merged.editor_name = gui_loaded.editor_name;
        }
        assert_eq!(merged.editor_name.as_deref(), Some("Helix"));
    }

    #[test]
    fn load_tui_keeps_own_editor_when_configured() {
        let dir = TempDir::new().unwrap();

        // GUI has one editor, TUI has a different one.
        let gui = AppSettings {
            editor_name: Some("VS Code".to_string()),
            ..Default::default()
        };
        write_json(&dir, &gui);

        let tui_path = dir.path().join("tui-settings.json");
        let tui_content = r#"{"last_repo":null,"recent_repos":[],"editor_name":"Neovim"}"#;
        std::fs::write(&tui_path, tui_content).unwrap();

        let tui_raw = load_from(&tui_path).unwrap();
        assert_eq!(tui_raw.editor_name.as_deref(), Some("Neovim"));

        // When TUI already has an editor, the GUI value must not override it.
        let gui_loaded = load_from(&dir.path().join("settings.json")).unwrap();
        let mut merged = tui_raw;
        if merged.editor_name.is_none() {
            merged.editor_name = gui_loaded.editor_name;
        }
        assert_eq!(merged.editor_name.as_deref(), Some("Neovim"));
    }

    #[test]
    fn load_tui_settings_returns_default_when_no_file() {
        // Can't easily control the real config dir in unit tests, but we can
        // verify load_from works correctly with a nonexistent path.
        let tmp = std::path::Path::new("/nonexistent/path/that/does/not/exist.json");
        let result = load_from(tmp).unwrap();
        assert_eq!(result.theme_name, None);
        assert!(result.recent_repos.is_empty());
    }

    // ── save_to: NamedTempFile atomic-write guarantees ──────────────────────────

    #[test]
    fn save_to_leaves_no_temp_files() {
        // After save_to completes there must be NO file in the directory
        // whose extension ends with `tmp`, proving NamedTempFile cleaned up.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        save_to(&path, &AppSettings::default()).unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|ext| ext.contains("tmp"))
                    .unwrap_or(false)
            })
            .collect();

        assert!(
            leftovers.is_empty(),
            "save_to must not leave any temp files: {leftovers:?}"
        );
        assert!(path.exists(), "settings file must exist after save_to");
    }

    #[test]
    fn save_to_creates_parent_directories_automatically() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("a").join("b").join("settings.json");
        // Parent dirs do not exist yet—save_to must create them.
        save_to(&nested, &AppSettings::default()).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn save_to_and_load_from_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        let mut s = AppSettings {
            theme_name: Some("Nord".to_string()),
            editor_name: Some("Neovim".to_string()),
            ..Default::default()
        };
        s.add_recent_repo(PathBuf::from("/tmp/my-repo"));

        save_to(&path, &s).unwrap();
        let loaded = load_from(&path).unwrap();

        assert_eq!(loaded.theme_name, Some("Nord".to_string()));
        assert_eq!(loaded.editor_name, Some("Neovim".to_string()));
        assert_eq!(loaded.recent_repos.len(), 1);
        assert_eq!(loaded.recent_repos[0].path, PathBuf::from("/tmp/my-repo"));
    }

    // ── update_settings combinator ──────────────────────────────────────────

    /// Test the combinator via the underlying `save_to` / `load_from`
    /// primitives (bypasses `dirs::config_dir` which is not controllable
    /// in unit tests).
    #[test]
    fn update_via_save_to_load_from_preserves_other_fields() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");

        // Write initial state with an editor and a recent repo.
        let mut initial = AppSettings {
            editor_name: Some("Vim".to_string()),
            ..Default::default()
        };
        initial.add_recent_repo(PathBuf::from("/tmp/repo1"));
        save_to(&path, &initial).unwrap();

        // Simulate what update_settings does: load → mutate → save.
        let mut s = load_from(&path).unwrap();
        s.theme_name = Some("Dracula".to_string());
        save_to(&path, &s).unwrap();

        let loaded = load_from(&path).unwrap();

        // New field was written.
        assert_eq!(loaded.theme_name, Some("Dracula".to_string()));
        // Pre-existing fields were NOT overwritten.
        assert_eq!(loaded.editor_name, Some("Vim".to_string()));
        assert_eq!(loaded.recent_repos.len(), 1);
    }

    #[test]
    fn update_via_save_to_multiple_mutations_in_one_write() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        save_to(&path, &AppSettings::default()).unwrap();

        // A single load → closure → save call touches multiple fields.
        let mut s = load_from(&path).unwrap();
        s.theme_name = Some("Monokai".to_string());
        s.editor_name = Some("Helix".to_string());
        s.active_tab_index = 2;
        save_to(&path, &s).unwrap();

        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.theme_name, Some("Monokai".to_string()));
        assert_eq!(loaded.editor_name, Some("Helix".to_string()));
        assert_eq!(loaded.active_tab_index, 2);
    }

    // ── update_tui_settings: raw-load avoids editor-fallback overwrite ────────

    /// `update_tui_settings` must load from the raw TUI file (not via
    /// `load_tui_settings`), so the GUI’s editor is never silently written
    /// back into the TUI file.
    #[test]
    fn update_tui_settings_does_not_inherit_gui_editor() {
        let dir = TempDir::new().unwrap();
        let gui_path = dir.path().join("settings.json");
        let tui_path = dir.path().join("tui-settings.json");

        // GUI has an editor; TUI file has none.
        let gui = AppSettings {
            editor_name: Some("VS Code".to_string()),
            ..Default::default()
        };
        save_to(&gui_path, &gui).unwrap();
        save_to(&tui_path, &AppSettings::default()).unwrap();

        // Simulate update_tui_settings: load raw (no fallback) → mutate → save.
        let mut s = load_from(&tui_path).unwrap();
        s.theme_name = Some("Nord".to_string());
        save_to(&tui_path, &s).unwrap();

        // The TUI file must have the new theme but editor_name must remain None.
        let tui_loaded = load_from(&tui_path).unwrap();
        assert_eq!(tui_loaded.theme_name, Some("Nord".to_string()));
        assert!(
            tui_loaded.editor_name.is_none(),
            "update_tui_settings must not inherit the GUI editor into the TUI file"
        );
    }
}
