//! `gitkraft-tui autostart …`: start the GitKraft window app (in the tray) when you log in.
//!
//! ```text
//! autostart status    whether the login entry is there, and what it starts
//! autostart on        add the login entry (macOS LaunchAgent, Linux autostart, Windows Run key)
//! autostart off       remove it
//! ```

use anyhow::{Result, anyhow};

/// A parsed `autostart` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Print the state.
    Status,
    /// Turn it on or off.
    Set(bool),
}

/// Parses the words after `autostart`.
///
/// # Errors
/// On anything but `status`, `on` or `off`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["status"] => Ok(Command::Status),
        ["on"] => Ok(Command::Set(true)),
        ["off"] => Ok(Command::Set(false)),
        _ => Err("usage: gitkraft-tui autostart [status|on|off]".into()),
    }
}

/// Runs it and returns what to print.
///
/// # Errors
/// If the entry cannot be written or removed.
pub fn run(cmd: Command) -> Result<String> {
    let exe = gitkraft_core::features::autostart::gui_executable();
    if let Command::Set(on) = cmd {
        gitkraft_core::features::autostart::set_enabled(on, &exe)
            .map_err(|e| anyhow!("could not change the login entry: {e}"))?;
        gitkraft_core::features::persistence::ops::update_settings(|s| s.autostart = on)
            .map_err(|e| anyhow!("could not save the setting: {e}"))?;
    }
    let on = gitkraft_core::features::autostart::is_enabled();
    Ok(format!(
        "autostart: {} ({})\n",
        if on { "on" } else { "off" },
        exe.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &[&str]) -> Vec<String> {
        s.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_words_pick_the_command() {
        assert_eq!(parse(&words(&[])), Ok(Command::Status));
        assert_eq!(parse(&words(&["status"])), Ok(Command::Status));
        assert_eq!(parse(&words(&["on"])), Ok(Command::Set(true)));
        assert_eq!(parse(&words(&["off"])), Ok(Command::Set(false)));
        assert!(parse(&words(&["maybe"])).is_err());
        assert!(parse(&words(&["on", "off"])).is_err());
    }
}
