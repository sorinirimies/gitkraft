//! Running `git` for anything that talks to a remote.
//!
//! libgit2 has no credentials here, so fetch and push go through the system `git`: ssh-agent,
//! `~/.ssh/config`, keychains and credential helpers then work exactly as in a terminal.
//! Failures are turned into a hint that says what to do about them.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

/// The `GIT_SSH_COMMAND` to set so a missing key fails at once instead of waiting for a
/// passphrase prompt nobody can answer. `None` when the user chose their own ssh command.
pub fn ssh_command_override(
    env_ssh_command: bool,
    env_git_ssh: bool,
    configured: bool,
) -> Option<&'static str> {
    (!env_ssh_command && !env_git_ssh && !configured).then_some("ssh -o BatchMode=yes")
}

fn user_chose_ssh(dir: Option<&Path>) -> bool {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.arg("-C").arg(d);
    }
    cmd.args(["config", "--get", "core.sshCommand"])
        .output()
        .is_ok_and(|o| o.status.success() && !o.stdout.iter().all(u8::is_ascii_whitespace))
}

/// A `git` command with prompts off and ssh set to fail fast. `dir` is the repository.
pub fn git_command(dir: Option<&Path>) -> Command {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.arg("-C").arg(d);
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    let over = ssh_command_override(
        std::env::var_os("GIT_SSH_COMMAND").is_some(),
        std::env::var_os("GIT_SSH").is_some(),
        user_chose_ssh(dir),
    );
    if let Some(ssh) = over {
        cmd.env("GIT_SSH_COMMAND", ssh);
    }
    cmd
}

/// Runs `git <args>` in `dir`; on failure the error is git's message plus a hint.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let output = git_command(Some(dir))
        .args(args)
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        anyhow::bail!(
            "{}",
            explain_git_error(&String::from_utf8_lossy(&output.stderr))
        );
    }
    Ok(())
}

/// Clones `url` into `dest` with the system `git`, so ssh keys and credential helpers work as in a terminal.
pub fn clone_repo(url: &str, dest: &Path) -> Result<()> {
    let output = git_command(None)
        .arg("clone")
        .arg("--")
        .arg(url)
        .arg(dest)
        .output()
        .context("failed to spawn git")?;
    if !output.status.success() {
        anyhow::bail!(
            "{}",
            explain_git_error(&String::from_utf8_lossy(&output.stderr))
        );
    }
    Ok(())
}

/// The host in git's `Could not resolve hostname x` / `ssh: connect to host x` lines, if any.
fn host_in(stderr: &str) -> Option<String> {
    let at = stderr.find("git@")?;
    let rest = &stderr[at + 4..];
    let end = rest.find([':', '/', '\'', ' ', '\n'])?;
    Some(rest[..end].to_string())
}

/// git's `stderr`, plus what to do when it is a known ssh or auth failure.
pub fn explain_git_error(stderr: &str) -> String {
    let text = stderr.trim();
    let lower = text.to_lowercase();
    let host = host_in(text).unwrap_or_else(|| "<host>".into());
    let hint = if lower.contains("permission denied (publickey") {
        Some(format!(
            "No SSH key was accepted. Run `gitkraft-tui ssh setup {host}` to create and register one, or load yours with `ssh-add`."
        ))
    } else if lower.contains("host key verification failed") {
        Some(format!(
            "The host is not trusted yet (or its key changed). Check with `ssh -T git@{host}` and accept the fingerprint only if it matches the platform's published one."
        ))
    } else if lower.contains("could not read from remote repository")
        && lower.contains("passphrase")
    {
        Some(
            "The key has a passphrase and is not in the agent. Run `ssh-add` and try again.".into(),
        )
    } else if lower.contains("terminal prompts disabled")
        || lower.contains("authentication failed")
        || lower.contains("could not read username")
    {
        Some("https needs a stored credential. Switch to ssh (`gitkraft-tui ssh switch ssh`) or sign in with your credential helper (`gh auth login`).".into())
    } else if lower.contains("could not resolve hostname")
        || lower.contains("network is unreachable")
    {
        Some("The host is not reachable: check the network or VPN.".into())
    } else {
        None
    };
    match hint {
        Some(h) => format!("{text}\n\nhint: {h}"),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_repository_can_be_cloned_and_a_missing_one_is_explained() {
        let root = tempfile::tempdir().expect("tmp");
        let src = root.path().join("src");
        std::fs::create_dir_all(&src).expect("mk");
        let run = |dir: &Path, args: &[&str]| {
            let st = Command::new("git")
                .current_dir(dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .status()
                .expect("git");
            assert!(st.success(), "{args:?}");
        };
        run(&src, &["init", "-q"]);
        run(&src, &["commit", "-q", "--allow-empty", "-m", "first"]);
        let dest = root.path().join("copy");
        clone_repo(src.to_str().expect("utf8"), &dest).expect("clone");
        assert!(dest.join(".git").exists());
        let err = clone_repo(
            root.path().join("nope").to_str().expect("utf8"),
            &root.path().join("x"),
        )
        .expect_err("missing");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn the_ssh_command_is_only_forced_when_the_user_set_none() {
        assert_eq!(
            ssh_command_override(false, false, false),
            Some("ssh -o BatchMode=yes")
        );
        assert_eq!(ssh_command_override(true, false, false), None);
        assert_eq!(ssh_command_override(false, true, false), None);
        assert_eq!(ssh_command_override(false, false, true), None);
    }

    #[test]
    fn a_denied_key_points_at_ssh_setup_with_the_host() {
        let msg = explain_git_error(
            "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.",
        );
        assert!(
            msg.contains("hint:") && msg.contains("ssh setup github.com"),
            "{msg}"
        );
        assert!(msg.starts_with("git@github.com: Permission denied"));
    }

    #[test]
    fn untrusted_hosts_passphrases_https_and_network_have_their_own_hints() {
        assert!(explain_git_error("Host key verification failed.").contains("ssh -T git@"));
        assert!(
            explain_git_error(
                "Enter passphrase\nfatal: Could not read from remote repository (passphrase)"
            )
            .contains("ssh-add")
        );
        assert!(
            explain_git_error(
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
            )
            .contains("ssh switch ssh")
        );
        assert!(
            explain_git_error("ssh: Could not resolve hostname github.com")
                .contains("network or VPN")
        );
    }

    #[test]
    fn unknown_errors_pass_through_unchanged() {
        assert_eq!(
            explain_git_error("  fatal: bad object\n"),
            "fatal: bad object"
        );
    }

    #[test]
    fn running_git_reports_its_stderr_with_context() {
        let dir = tempfile::tempdir().expect("tmp");
        let err = run_git(dir.path(), &["fetch", "nowhere"]).expect_err("not a repo");
        assert!(!err.to_string().is_empty());
    }
}
