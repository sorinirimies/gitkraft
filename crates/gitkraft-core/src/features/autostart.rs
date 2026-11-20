//! Start GitKraft when the user logs in (macOS, Linux and Windows).
//!
//! * macOS: a LaunchAgent plist in `~/Library/LaunchAgents`.
//! * Linux: an XDG autostart `.desktop` file in `~/.config/autostart`.
//! * Windows: a value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! The entry starts the app with [`BACKGROUND_ARG`], so it opens to the tray instead of a window.
//! Nothing here needs admin rights and every call is idempotent.

use std::io;
use std::path::{Path, PathBuf};

/// Argument the login entry passes: start hidden, in the tray.
pub const BACKGROUND_ARG: &str = "--background";

/// The name of the entry (the plist label, the `.desktop` file stem, the registry value).
pub const ENTRY_NAME: &str = "GitKraft";

const MAC_LABEL: &str = "dev.gitkraft.app";

/// The platforms an entry is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// A LaunchAgent.
    MacOs,
    /// An XDG autostart file.
    Linux,
    /// A `Run` registry value.
    Windows,
}

impl Os {
    /// The platform this was built for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Linux
        }
    }
}

/// The file that holds the entry, relative to `home` (`None` on Windows: it is in the registry).
#[must_use]
pub fn entry_file(os: Os, home: &Path) -> Option<PathBuf> {
    match os {
        Os::MacOs => Some(
            home.join("Library/LaunchAgents")
                .join(format!("{MAC_LABEL}.plist")),
        ),
        Os::Linux => Some(home.join(".config/autostart/gitkraft.desktop")),
        Os::Windows => None,
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Quotes one argument for a `.desktop` `Exec=` line.
fn desktop_quote(s: &str) -> String {
    if s.chars()
        .any(|c| c.is_whitespace() || "\"'\\><~|&;$*?#()`".contains(c))
    {
        let escaped: String = s
            .chars()
            .flat_map(|c| match c {
                '"' | '`' | '$' | '\\' => vec!['\\', c],
                _ => vec![c],
            })
            .collect();
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// The text of the entry file for `exe` (`None` on Windows).
#[must_use]
pub fn render(os: Os, exe: &Path) -> Option<String> {
    let exe = exe.display().to_string();
    match os {
        Os::MacOs => Some(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n<dict>\n\
  <key>Label</key><string>{MAC_LABEL}</string>\n\
  <key>ProgramArguments</key>\n  <array>\n    <string>{}</string>\n    <string>{BACKGROUND_ARG}</string>\n  </array>\n\
  <key>RunAtLoad</key><true/>\n\
  <key>ProcessType</key><string>Interactive</string>\n\
</dict>\n</plist>\n",
            xml_escape(&exe)
        )),
        Os::Linux => Some(format!(
            "[Desktop Entry]\nType=Application\nName={ENTRY_NAME}\nComment=Git client, pull requests and CI in one place\nExec={} {BACKGROUND_ARG}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
            desktop_quote(&exe)
        )),
        Os::Windows => None,
    }
}

/// The value written under the `Run` key.
#[must_use]
pub fn windows_command(exe: &Path) -> String {
    format!("\"{}\" {BACKGROUND_ARG}", exe.display())
}

/// Turns the login entry on or off, for the current platform.
///
/// # Errors
/// If the file or registry value cannot be written or removed.
pub fn set_enabled(enabled: bool, exe: &Path) -> io::Result<()> {
    if Os::current() == Os::Windows {
        return windows_set(enabled, exe);
    }
    let home = dirs_home()?;
    set_enabled_in(Os::current(), &home, enabled, exe)
}

/// Is the login entry there?
#[must_use]
pub fn is_enabled() -> bool {
    if Os::current() == Os::Windows {
        return windows_is_set();
    }
    dirs_home().is_ok_and(|h| is_enabled_in(Os::current(), &h))
}

/// [`set_enabled`] against an explicit home folder (file based platforms only).
///
/// # Errors
/// If the file cannot be written or removed.
pub fn set_enabled_in(os: Os, home: &Path, enabled: bool, exe: &Path) -> io::Result<()> {
    let (Some(path), Some(text)) = (entry_file(os, home), render(os, exe)) else {
        return Err(io::Error::other("this platform keeps the entry elsewhere"));
    };
    if enabled {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if std::fs::read_to_string(&path).is_ok_and(|t| t == text) {
            return Ok(());
        }
        std::fs::write(path, text)
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// [`is_enabled`] against an explicit home folder.
#[must_use]
pub fn is_enabled_in(os: Os, home: &Path) -> bool {
    entry_file(os, home).is_some_and(|p| p.is_file())
}

fn dirs_home() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .ok_or_else(|| io::Error::other("no home folder"))
}

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

fn windows_set(enabled: bool, exe: &Path) -> io::Result<()> {
    let mut cmd = std::process::Command::new("reg");
    if enabled {
        cmd.args(["add", RUN_KEY, "/v", ENTRY_NAME, "/t", "REG_SZ", "/d"])
            .arg(windows_command(exe))
            .arg("/f");
    } else {
        if !windows_is_set() {
            return Ok(());
        }
        cmd.args(["delete", RUN_KEY, "/v", ENTRY_NAME, "/f"]);
    }
    let out = cmd.stdin(std::process::Stdio::null()).output()?;
    if out.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

fn windows_is_set() -> bool {
    std::process::Command::new("reg")
        .args(["query", RUN_KEY, "/v", ENTRY_NAME])
        .stdin(std::process::Stdio::null())
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The GitKraft window app next to the running executable (the TUI's sibling), or the
/// running executable itself.
#[must_use]
pub fn gui_executable() -> PathBuf {
    let name = if cfg!(windows) {
        "gitkraft.exe"
    } else {
        "gitkraft"
    };
    std::env::current_exe()
        .ok()
        .and_then(|me| {
            let sibling = me.with_file_name(name);
            sibling.is_file().then_some(sibling).or(Some(me))
        })
        .unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_platform_names_its_entry() {
        let home = Path::new("/h");
        assert_eq!(
            entry_file(Os::MacOs, home),
            Some(PathBuf::from(
                "/h/Library/LaunchAgents/dev.gitkraft.app.plist"
            ))
        );
        assert_eq!(
            entry_file(Os::Linux, home),
            Some(PathBuf::from("/h/.config/autostart/gitkraft.desktop"))
        );
        assert_eq!(entry_file(Os::Windows, home), None);
    }

    #[test]
    fn entries_start_the_app_in_the_background_and_quote_odd_paths() {
        let exe = Path::new("/Apps/Git Kraft & Co/gitkraft");
        let plist = render(Os::MacOs, exe).expect("plist");
        assert!(plist.contains("<string>/Apps/Git Kraft &amp; Co/gitkraft</string>"));
        assert!(plist.contains("<string>--background</string>") && plist.contains("RunAtLoad"));
        let desktop = render(Os::Linux, exe).expect("desktop");
        assert!(desktop.contains("Exec=\"/Apps/Git Kraft & Co/gitkraft\" --background"));
        assert!(
            render(Os::Linux, Path::new("/usr/bin/gitkraft"))
                .expect("desktop")
                .contains("Exec=/usr/bin/gitkraft --background")
        );
        assert_eq!(
            windows_command(Path::new(r"C:\Program Files\GitKraft\gitkraft.exe")),
            r#""C:\Program Files\GitKraft\gitkraft.exe" --background"#
        );
        assert!(render(Os::Windows, exe).is_none());
    }

    #[test]
    fn turning_it_on_and_off_is_idempotent_and_leaves_other_files_alone() {
        let tmp = tempfile::tempdir().expect("tmp");
        let home = tmp.path();
        let exe = Path::new("/usr/bin/gitkraft");
        for os in [Os::MacOs, Os::Linux] {
            assert!(!is_enabled_in(os, home));
            set_enabled_in(os, home, false, exe).expect("off when absent");
            set_enabled_in(os, home, true, exe).expect("on");
            set_enabled_in(os, home, true, exe).expect("on again");
            assert!(is_enabled_in(os, home));
            set_enabled_in(os, home, false, exe).expect("off");
            set_enabled_in(os, home, false, exe).expect("off again");
            assert!(!is_enabled_in(os, home));
        }
        assert!(set_enabled_in(Os::Windows, home, true, exe).is_err());
    }
}
