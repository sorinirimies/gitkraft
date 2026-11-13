//! Where a token can be kept: the system keyring, through the programs the OS ships.
//!
//! macOS uses `security`, Linux uses `secret-tool` (libsecret). Nothing is written to a file
//! gitkraft owns. Without such a program, tokens come from the environment or a platform CLI.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::account::Secret;

/// The service name entries are filed under.
pub const SERVICE: &str = "gitkraft";

/// A place tokens can be read from and written to.
pub trait SecretStore {
    /// The token stored for `host`.
    fn get(&self, host: &str) -> Option<Secret>;
    /// Stores `token` for `host`, replacing an old one.
    fn set(&self, host: &str, token: &Secret) -> Result<(), String>;
    /// Removes the token of `host`; `false` when there was none.
    fn delete(&self, host: &str) -> Result<bool, String>;
}

/// `security find-generic-password` arguments (macOS).
pub fn macos_find_args(host: &str) -> Vec<String> {
    ["find-generic-password", "-s", SERVICE, "-a", host, "-w"]
        .map(String::from)
        .to_vec()
}

/// `security add-generic-password` arguments; the password is appended as the value of `-w`.
pub fn macos_add_args(host: &str) -> Vec<String> {
    [
        "add-generic-password",
        "-U",
        "-s",
        SERVICE,
        "-a",
        host,
        "-w",
    ]
    .map(String::from)
    .to_vec()
}

/// `security delete-generic-password` arguments.
pub fn macos_delete_args(host: &str) -> Vec<String> {
    ["delete-generic-password", "-s", SERVICE, "-a", host]
        .map(String::from)
        .to_vec()
}

/// `secret-tool lookup` arguments (Linux).
pub fn linux_lookup_args(host: &str) -> Vec<String> {
    ["lookup", "service", SERVICE, "host", host]
        .map(String::from)
        .to_vec()
}

/// `secret-tool store` arguments; the secret is read from stdin.
pub fn linux_store_args(host: &str) -> Vec<String> {
    [
        "store",
        "--label",
        &format!("gitkraft token for {host}"),
        "service",
        SERVICE,
        "host",
        host,
    ]
    .map(String::from)
    .to_vec()
}

/// `secret-tool clear` arguments.
pub fn linux_clear_args(host: &str) -> Vec<String> {
    ["clear", "service", SERVICE, "host", host]
        .map(String::from)
        .to_vec()
}

fn secret_of(out: &std::process::Output) -> Option<Secret> {
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !text.is_empty()).then(|| Secret::new(text))
}

/// The OS keyring.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemKeyring;

impl SecretStore for SystemKeyring {
    fn get(&self, host: &str) -> Option<Secret> {
        let out = if cfg!(target_os = "macos") {
            Command::new("security")
                .args(macos_find_args(host))
                .stderr(Stdio::null())
                .output()
        } else if cfg!(target_os = "linux") {
            Command::new("secret-tool")
                .args(linux_lookup_args(host))
                .stderr(Stdio::null())
                .output()
        } else {
            return None;
        };
        secret_of(&out.ok()?)
    }

    fn set(&self, host: &str, token: &Secret) -> Result<(), String> {
        if cfg!(target_os = "macos") {
            // `security` cannot read the secret from stdin, so it is an argument here and shows in
            // the process list for the moment the command runs. Linux (`secret-tool`) uses stdin.
            let mut args = macos_add_args(host);
            args.push(token.expose().to_string());
            let out = Command::new("security")
                .args(args)
                .output()
                .map_err(|e| format!("cannot run security: {e}"))?;
            out.status
                .success()
                .then_some(())
                .ok_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string())
        } else if cfg!(target_os = "linux") {
            let mut child = Command::new("secret-tool")
                .args(linux_store_args(host))
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("cannot run secret-tool (install libsecret-tools): {e}"))?;
            child
                .stdin
                .take()
                .ok_or("no stdin")?
                .write_all(token.expose().as_bytes())
                .map_err(|e| e.to_string())?;
            let out = child.wait_with_output().map_err(|e| e.to_string())?;
            out.status
                .success()
                .then_some(())
                .ok_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string())
        } else {
            Err("no system keyring support on this OS: set GITKRAFT_TOKEN_<HOST> instead".into())
        }
    }

    fn delete(&self, host: &str) -> Result<bool, String> {
        let had = self.get(host).is_some();
        if !had {
            return Ok(false);
        }
        let out = if cfg!(target_os = "macos") {
            Command::new("security")
                .args(macos_delete_args(host))
                .output()
        } else {
            Command::new("secret-tool")
                .args(linux_clear_args(host))
                .output()
        }
        .map_err(|e| e.to_string())?;
        out.status
            .success()
            .then_some(true)
            .ok_or_else(|| String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// An in-memory store for tests.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore(
    pub std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
);

impl SecretStore for MemoryStore {
    fn get(&self, host: &str) -> Option<Secret> {
        self.0.lock().ok()?.get(host).cloned().map(Secret::new)
    }

    fn set(&self, host: &str, token: &Secret) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .insert(host.to_string(), token.expose().to_string());
        Ok(())
    }

    fn delete(&self, host: &str) -> Result<bool, String> {
        Ok(self
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .remove(host)
            .is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_built_per_os_without_the_token_in_the_list_where_possible() {
        assert_eq!(
            macos_find_args("github.com"),
            [
                "find-generic-password",
                "-s",
                "gitkraft",
                "-a",
                "github.com",
                "-w"
            ]
        );
        assert_eq!(macos_add_args("h").last().map(String::as_str), Some("-w"));
        assert_eq!(macos_delete_args("h")[0], "delete-generic-password");
        assert_eq!(
            linux_lookup_args("h"),
            ["lookup", "service", "gitkraft", "host", "h"]
        );
        assert!(linux_store_args("h").contains(&"gitkraft token for h".to_string()));
        assert_eq!(linux_clear_args("h")[0], "clear");
    }

    #[test]
    fn the_memory_store_round_trips() {
        let s = MemoryStore::default();
        assert!(s.get("h").is_none());
        s.set("h", &Secret::new("t")).expect("set");
        assert_eq!(s.get("h").map(|x| x.expose().to_string()), Some("t".into()));
        s.set("h", &Secret::new("u")).expect("replace");
        assert_eq!(s.get("h").map(|x| x.expose().to_string()), Some("u".into()));
        assert!(s.delete("h").expect("delete"));
        assert!(!s.delete("h").expect("again"));
    }
}
