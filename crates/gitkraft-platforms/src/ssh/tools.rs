//! The `ssh`, `ssh-keygen` and `ssh-add` binaries, behind a trait so tests never touch real keys.

use std::path::Path;
use std::process::{Command, Stdio};

/// What `ssh -T git@host` told us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// The platform accepted the key; `user` when it said who we are.
    Authenticated {
        /// The account name in the greeting, if any.
        user: Option<String>,
    },
    /// No key was accepted.
    Denied,
    /// The host key is unknown or changed.
    HostKey,
    /// Could not reach the host.
    Unreachable,
    /// Anything else, with the output.
    Other(String),
}

/// Reads the output of `ssh -T git@host`.
pub fn interpret_probe(output: &str) -> Probe {
    let lower = output.to_lowercase();
    if lower.contains("permission denied") {
        return Probe::Denied;
    }
    if lower.contains("host key verification failed")
        || lower.contains("remote host identification has changed")
    {
        return Probe::HostKey;
    }
    if lower.contains("could not resolve")
        || lower.contains("connection timed out")
        || lower.contains("connection refused")
        || lower.contains("network is unreachable")
        || lower.contains("no route to host")
    {
        return Probe::Unreachable;
    }
    if lower.contains("successfully authenticated") || lower.contains("welcome to gitlab") {
        // GitHub / Gitea: "Hi NAME! You've successfully…", Gitea: "Hi there, NAME! …", GitLab: "Welcome to GitLab, @NAME!"
        let user = ["Hi there, ", "Hi ", "Welcome to GitLab, @"]
            .iter()
            .find_map(|p| output.find(p).map(|i| &output[i + p.len()..]))
            .and_then(|rest| rest.split(['!', ',', ' ', '\n']).next())
            .filter(|n| !n.is_empty() && *n != "there")
            .map(str::to_string);
        return Probe::Authenticated { user };
    }
    Probe::Other(output.trim().to_string())
}

/// Where to connect: `user@host:port`, each part but the host optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTarget {
    /// The login, `git` unless given.
    pub user: String,
    /// The host name.
    pub host: String,
    /// A port, or `None` for ssh's own default and `~/.ssh/config`.
    pub port: Option<u16>,
}

impl SshTarget {
    /// Reads `host`, `host:port`, `user@host` or `user@host:port`.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        let (user, rest) = match s.split_once('@') {
            Some((u, r)) if !u.is_empty() => (u, r),
            _ => ("git", s.trim_start_matches('@')),
        };
        let (host, port) = match rest.rsplit_once(':') {
            Some((h, p)) if p.parse::<u16>().is_ok() => (h, p.parse().ok()),
            _ => (rest, None),
        };
        Self {
            user: user.to_string(),
            host: host.to_string(),
            port,
        }
    }

    /// The arguments for `ssh -T`.
    #[must_use]
    pub fn args(&self) -> Vec<String> {
        let mut a = vec![
            "-T".to_string(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=10".into(),
        ];
        if let Some(p) = self.port {
            a.push("-p".into());
            a.push(p.to_string());
        }
        a.push(format!("{}@{}", self.user, self.host));
        a
    }
}

/// The ssh programs gitkraft needs.
pub trait SshTools {
    /// Creates an ed25519 key pair at `path`. With `interactive`, `ssh-keygen` asks for the
    /// passphrase itself (gitkraft never sees it); otherwise the key has none.
    fn generate(&self, path: &Path, comment: &str, interactive: bool) -> Result<(), String>;
    /// Public keys the agent holds (`ssh-add -L`); empty when there is no agent.
    fn agent_keys(&self) -> Vec<String>;
    /// Loads the private key at `path` into the agent.
    fn add_to_agent(&self, path: &Path) -> Result<(), String>;
    /// Tries `ssh -T`; `host` may be a full target such as `gitea@host:2222` (see [`SshTarget`]).
    fn probe(&self, host: &str) -> Probe;
}

/// The real programs.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSsh;

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

impl SshTools for SystemSsh {
    fn generate(&self, path: &Path, comment: &str, interactive: bool) -> Result<(), String> {
        let mut cmd = Command::new("ssh-keygen");
        cmd.args(["-t", "ed25519", "-C", comment, "-f"]).arg(path);
        if interactive {
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let status = cmd
                .status()
                .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
            return status
                .success()
                .then_some(())
                .ok_or_else(|| "ssh-keygen failed".to_string());
        }
        cmd.args(["-N", ""]);
        let out = cmd
            .output()
            .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
        out.status.success().then_some(()).ok_or_else(|| text(&out))
    }

    fn agent_keys(&self) -> Vec<String> {
        Command::new("ssh-add")
            .arg("-L")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn add_to_agent(&self, path: &Path) -> Result<(), String> {
        let mut cmd = Command::new("ssh-add");
        if cfg!(target_os = "macos") {
            cmd.arg("--apple-use-keychain");
        }
        let out = cmd
            .arg(path)
            .output()
            .map_err(|e| format!("cannot run ssh-add: {e}"))?;
        out.status.success().then_some(()).ok_or_else(|| text(&out))
    }

    fn probe(&self, host: &str) -> Probe {
        match Command::new("ssh")
            .args(SshTarget::parse(host).args())
            .output()
        {
            Ok(out) => interpret_probe(&text(&out)),
            Err(e) => Probe::Other(format!("cannot run ssh: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_carry_user_and_port_and_default_to_git() {
        let t = SshTarget::parse("github.com");
        assert_eq!(
            (t.user.as_str(), t.host.as_str(), t.port),
            ("git", "github.com", None)
        );
        let t = SshTarget::parse("gitea@192.168.1.44");
        assert_eq!((t.user.as_str(), t.port), ("gitea", None));
        let t = SshTarget::parse("git@192.168.1.204:30009");
        assert_eq!((t.host.as_str(), t.port), ("192.168.1.204", Some(30009)));
        assert!(
            t.args()
                .ends_with(&["-p".into(), "30009".into(), "git@192.168.1.204".into()])
        );
        assert_eq!(SshTarget::parse("host:notaport").host, "host:notaport");
    }

    use super::*;

    #[test]
    fn greetings_of_each_platform_are_recognised() {
        assert_eq!(
            interpret_probe(
                "Hi octocat! You've successfully authenticated, but GitHub does not provide shell access.\n"
            ),
            Probe::Authenticated {
                user: Some("octocat".into())
            }
        );
        assert_eq!(
            interpret_probe("Welcome to GitLab, @sorin!\n"),
            Probe::Authenticated {
                user: Some("sorin".into())
            }
        );
        assert_eq!(
            interpret_probe(
                "Hi there, sorin! You've successfully authenticated with the key named gitkraft laptop, but Forgejo does not provide shell access.\n"
            ),
            Probe::Authenticated {
                user: Some("sorin".into())
            }
        );
        assert_eq!(
            interpret_probe("Hi there! You've successfully authenticated"),
            Probe::Authenticated { user: None }
        );
    }

    #[test]
    fn failures_are_told_apart() {
        assert_eq!(
            interpret_probe("git@github.com: Permission denied (publickey)."),
            Probe::Denied
        );
        assert_eq!(
            interpret_probe("Host key verification failed."),
            Probe::HostKey
        );
        assert_eq!(
            interpret_probe("ssh: Could not resolve hostname x: nodename nor servname"),
            Probe::Unreachable
        );
        assert_eq!(
            interpret_probe("ssh: connect to host x port 22: Connection timed out"),
            Probe::Unreachable
        );
        assert_eq!(interpret_probe("  weird \n"), Probe::Other("weird".into()));
    }
}

/// A fake for tests (here and in the front-ends): writes a made-up key pair, records what was asked.
#[derive(Debug, Clone, Default)]
pub struct FakeSsh {
    /// Keys the fake agent holds.
    pub agent: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// What `probe` answers.
    pub probe_result: Option<Probe>,
    /// Make `add_to_agent` fail with this.
    pub agent_error: Option<String>,
    /// Calls made, in order: `generate <path>`, `add <path>`, `probe <host>`.
    pub calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

/// A valid-looking public key line.
pub const FAKE_PUBLIC_KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJ0r5GmJcQGHlZ4gkUXZv9m+i1m8c4pQXnTQ4v8nY2ka";

impl FakeSsh {
    fn note(&self, call: String) {
        if let Ok(mut c) = self.calls.lock() {
            c.push(call);
        }
    }

    /// The calls so far.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

impl SshTools for FakeSsh {
    fn generate(&self, path: &Path, comment: &str, _interactive: bool) -> Result<(), String> {
        self.note(format!("generate {}", path.display()));
        std::fs::write(path, "PRIVATE").map_err(|e| e.to_string())?;
        std::fs::write(
            path.with_extension("pub"),
            format!("{FAKE_PUBLIC_KEY} {comment}\n"),
        )
        .map_err(|e| e.to_string())
    }

    fn agent_keys(&self) -> Vec<String> {
        self.agent.lock().map(|a| a.clone()).unwrap_or_default()
    }

    fn add_to_agent(&self, path: &Path) -> Result<(), String> {
        self.note(format!("add {}", path.display()));
        match &self.agent_error {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    fn probe(&self, host: &str) -> Probe {
        self.note(format!("probe {host}"));
        self.probe_result.clone().unwrap_or(Probe::Authenticated {
            user: Some("me".into()),
        })
    }
}
