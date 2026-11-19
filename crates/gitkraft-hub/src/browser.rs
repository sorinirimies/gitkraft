//! Opens a web page in the user's default browser.
//!
//! Only `http` and `https` links are opened: a URL comes from a forge's API, and nothing a
//! server sends should be able to start another kind of handler.

use std::io;
use std::process::{Command, Stdio};

/// Whether `url` is a link this module will open.
#[must_use]
pub fn is_web_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// The command that opens `url` on this OS.
fn command(url: &str) -> Command {
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");
    cmd.arg(url);
    cmd
}

/// Opens `url` in the default browser without waiting for it.
///
/// # Errors
/// If the link is not a plain http(s) URL, or the opener cannot be started.
pub fn open(url: &str) -> io::Result<()> {
    if !is_web_url(url) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a web link: {url}"),
        ));
    }
    command(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_web_links_are_opened() {
        assert!(is_web_url("https://github.com/o/r/pull/1"));
        assert!(is_web_url("http://localhost:3333/x"));
        for bad in [
            "",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://x",
            "https://a b",
            "https://a\nb",
            "github.com/o/r",
        ] {
            assert!(!is_web_url(bad), "{bad:?}");
            assert!(open(bad).is_err(), "{bad:?}");
        }
    }
}
