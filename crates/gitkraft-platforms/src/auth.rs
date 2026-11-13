//! Finding a token: environment first, then the platform's own CLI.

use std::process::Command;

use crate::account::Secret;
use crate::platform::PlatformKind;

/// Environment variable names tried for `host`, most specific first:
/// `GITKRAFT_TOKEN_<HOST>` (dots and dashes become `_`), then the platform's usual one.
pub fn env_var_names(kind: PlatformKind, host: &str) -> Vec<String> {
    let host_var: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let mut names = vec![format!("GITKRAFT_TOKEN_{host_var}")];
    match kind {
        PlatformKind::GitHub => names.extend(["GH_TOKEN".into(), "GITHUB_TOKEN".into()]),
        PlatformKind::GitLab => names.extend(["GITLAB_TOKEN".into(), "GLAB_TOKEN".into()]),
        PlatformKind::Gitea => {
            names.extend(["GITEA_TOKEN".into(), "FORGEJO_TOKEN".into()]);
            if host == "codeberg.org" {
                names.push("CODEBERG_TOKEN".into());
            }
        }
    }
    names
}

/// First non-empty value among `names` according to `lookup`.
pub fn token_from_env(names: &[String], lookup: impl Fn(&str) -> Option<String>) -> Option<Secret> {
    names
        .iter()
        .filter_map(|n| lookup(n))
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
        .map(Secret::new)
}

/// Asks the platform CLI (`gh auth token`, `glab auth status -t`) for a token.
/// Returns `None` when the CLI is missing or not logged in.
pub fn token_from_cli(kind: PlatformKind, host: &str) -> Option<Secret> {
    let output = match kind {
        PlatformKind::GitHub => Command::new("gh")
            .args(["auth", "token", "--hostname", host])
            .output()
            .ok()?,
        PlatformKind::GitLab => {
            let out = Command::new("glab")
                .args(["config", "get", "token", "--host", host])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            out
        }
        PlatformKind::Gitea => return None,
    };
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!token.is_empty()).then(|| Secret::new(token))
}

/// The `password=` of a `git credential fill` answer.
pub fn password_from_credential(output: &str) -> Option<Secret> {
    output
        .lines()
        .find_map(|l| l.strip_prefix("password="))
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(Secret::new)
}

/// Asks git's credential helper (keychain, `gh auth setup-git`, …) for the https password of
/// `host`, which for the platforms is a personal access token. Never prompts.
pub fn token_from_git_credential(host: &str) -> Option<Secret> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("git")
        .args(["credential", "fill"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child
        .stdin
        .take()?
        .write_all(format!("protocol=https\nhost={host}\n\n").as_bytes())
        .ok()?;
    let out = child.wait_with_output().ok()?;
    out.status
        .success()
        .then(|| password_from_credential(&String::from_utf8_lossy(&out.stdout)))
        .flatten()
}

/// Resolves a token for `host`: environment, then the system keyring (`gitkraft-tui login`), then
/// the platform CLI, then git's credential helper. Tokens are never stored by gitkraft.
pub fn resolve_token(kind: PlatformKind, host: &str) -> Option<Secret> {
    use crate::credentials::{SecretStore, SystemKeyring};
    token_from_env(&env_var_names(kind, host), |n| std::env::var(n).ok())
        .or_else(|| SystemKeyring.get(host))
        .or_else(|| token_from_cli(kind, host))
        .or_else(|| token_from_git_credential(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_specific_variable_comes_first() {
        let names = env_var_names(PlatformKind::Gitea, "codeberg.org");
        assert_eq!(names[0], "GITKRAFT_TOKEN_CODEBERG_ORG");
        assert!(names.contains(&"CODEBERG_TOKEN".to_string()));
        assert_eq!(
            env_var_names(PlatformKind::GitLab, "git-lab.example.com")[0],
            "GITKRAFT_TOKEN_GIT_LAB_EXAMPLE_COM"
        );
    }

    #[test]
    fn the_credential_answer_yields_its_password() {
        let out = "protocol=https\nhost=github.com\nusername=me\npassword=ghp_secret\n";
        assert_eq!(
            password_from_credential(out).map(|s| s.expose().to_string()),
            Some("ghp_secret".into())
        );
        assert!(password_from_credential("username=me\npassword=\n").is_none());
        assert!(password_from_credential("").is_none());
    }

    #[test]
    fn first_non_empty_wins_and_is_trimmed() {
        let names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let lookup = |n: &str| match n {
            "A" => Some("  ".to_string()),
            "B" => Some(" tok\n".to_string()),
            "C" => Some("other".to_string()),
            _ => None,
        };
        assert_eq!(
            token_from_env(&names, lookup).map(|s| s.expose().to_string()),
            Some("tok".into())
        );
        assert_eq!(token_from_env(&names, |_| None), None);
    }
}
