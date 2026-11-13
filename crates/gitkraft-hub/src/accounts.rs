//! Which accounts exist: the hosts of your remotes, `accounts.toml`, and the well-known hosts you have a token for.

use gitkraft_platforms::{Account, HostMap, PlatformKind, Secret, parse_remote_url};
use serde::Deserialize;

/// One `[[account]]` in `accounts.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AccountConfig {
    /// Platform kind.
    pub kind: PlatformKind,
    /// Host, e.g. `git.example.com`.
    pub host: String,
    /// Which URL to use when gitkraft adds a remote or switches one (`ssh` or `https`).
    #[serde(default)]
    pub clone_via: Option<gitkraft_platforms::Transport>,
    /// Client id of your OAuth application on this host, for `login` without a pasted token.
    #[serde(default)]
    pub oauth_client_id: Option<String>,
    /// URL of a Woodpecker CI server that builds this host's repositories, e.g. `https://ci.codeberg.org`.
    /// The token comes from `GITKRAFT_WOODPECKER_TOKEN_<HOST>` or `GITKRAFT_WOODPECKER_TOKEN`, never from this file.
    #[serde(default)]
    pub woodpecker: Option<String>,
}

/// The whole `accounts.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AccountsFile {
    /// Self-hosted and extra accounts.
    #[serde(default, rename = "account")]
    pub accounts: Vec<AccountConfig>,
}

impl AccountsFile {
    /// Parses `accounts.toml`; an empty file is fine.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// The host map: well-known hosts plus these.
    pub fn host_map(&self) -> HostMap {
        self.accounts
            .iter()
            .fold(HostMap::new(), |m, a| m.with(a.host.clone(), a.kind))
    }
}

impl AccountsFile {
    /// The transport chosen for `host`, if any.
    pub fn clone_via(&self, host: &str) -> Option<gitkraft_platforms::Transport> {
        self.accounts
            .iter()
            .find(|a| a.host.eq_ignore_ascii_case(host))
            .and_then(|a| a.clone_via)
    }
}

fn env_suffix(host: &str) -> String {
    host.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// The Woodpecker server configured for `host`, with its token from `env`.
/// Remote URLs of the git checkout containing `dir`, `origin` first.
pub fn remote_urls(dir: &std::path::Path) -> Vec<String> {
    let Ok(repo) = git2::Repository::discover(dir) else {
        return Vec::new();
    };
    let Ok(names) = repo.remotes() else {
        return Vec::new();
    };
    let mut names: Vec<&str> = names.iter().flatten().flatten().collect();
    names.sort_by_key(|n| *n != "origin");
    names
        .into_iter()
        .filter_map(|n| repo.find_remote(n).ok()?.url().ok().map(str::to_string))
        .collect()
}

/// The repository of the checkout containing `dir`, from its first remote.
pub fn repo_of_dir(dir: &std::path::Path) -> Option<gitkraft_platforms::RepoId> {
    remote_urls(dir)
        .iter()
        .find_map(|u| gitkraft_platforms::platform::parse_remote_url(u))
}

/// Accounts for the checkout in `dir` plus `repos` (`host/owner/name`), from `accounts.toml` and the
/// usual token sources. Blocks (it may run `gh` or `git credential fill`).
///
/// # Errors
/// `accounts.toml` is present but does not parse.
pub fn discover_for(
    dir: &std::path::Path,
    repos: &[String],
) -> Result<Vec<gitkraft_platforms::Account>, String> {
    use gitkraft_settings::files;
    let file = match files::accounts_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
    {
        Some(text) => AccountsFile::parse(&text).map_err(|e| format!("accounts.toml: {e}"))?,
        None => AccountsFile::default(),
    };
    let mut urls = remote_urls(dir);
    urls.extend(repos.iter().map(|k| format!("https://{k}")));
    Ok(discover(
        &urls,
        &file,
        gitkraft_platforms::auth::resolve_token,
    ))
}

pub fn woodpecker_for(
    host: &str,
    file: &AccountsFile,
    env: impl Fn(&str) -> Option<String>,
) -> Option<gitkraft_platforms::Woodpecker> {
    let url = file
        .accounts
        .iter()
        .find(|a| a.host.eq_ignore_ascii_case(host))?
        .woodpecker
        .clone()
        .filter(|u| !u.trim().is_empty())?;
    let token = env(&format!("GITKRAFT_WOODPECKER_TOKEN_{}", env_suffix(host)))
        .or_else(|| env("GITKRAFT_WOODPECKER_TOKEN"))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .map(Secret::new);
    Some(gitkraft_platforms::Woodpecker {
        url: url.trim().trim_end_matches('/').to_string(),
        token,
    })
}

/// The OAuth client id to use for `host`: the argument, then `GITKRAFT_OAUTH_CLIENT_ID_<HOST>`,
/// then `GITKRAFT_OAUTH_CLIENT_ID`, then `accounts.toml`.
pub fn oauth_client_id(
    host: &str,
    arg: Option<&str>,
    env: impl Fn(&str) -> Option<String>,
    file: &AccountsFile,
) -> Option<String> {
    let suffix: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    arg.map(str::to_string)
        .or_else(|| env(&format!("GITKRAFT_OAUTH_CLIENT_ID_{suffix}")))
        .or_else(|| env("GITKRAFT_OAUTH_CLIENT_ID"))
        .or_else(|| {
            file.accounts
                .iter()
                .find(|a| a.host.eq_ignore_ascii_case(host))
                .and_then(|a| a.oauth_client_id.clone())
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Builds the account list: every host found in `remote_urls` or `file` (deduplicated, in first-seen order),
/// plus the well-known hosts when `resolve` finds a token for them. `resolve` supplies the token.
pub fn discover(
    remote_urls: &[String],
    file: &AccountsFile,
    resolve: impl Fn(PlatformKind, &str) -> Option<Secret>,
) -> Vec<Account> {
    let map = file.host_map();
    let mut hosts: Vec<(PlatformKind, String)> = Vec::new();
    let mut add = |kind: PlatformKind, host: String| {
        let host = host.to_lowercase();
        if !hosts.iter().any(|(_, h)| *h == host) {
            hosts.push((kind, host));
        }
    };
    for url in remote_urls {
        if let Some(id) = parse_remote_url(url)
            && let Some(kind) = map.detect(&id.host)
        {
            add(kind, id.host);
        }
    }
    for a in &file.accounts {
        add(a.kind, a.host.clone());
    }
    for (kind, host) in [
        (PlatformKind::GitHub, "github.com"),
        (PlatformKind::GitLab, "gitlab.com"),
        (PlatformKind::Gitea, "codeberg.org"),
    ] {
        if resolve(kind, host).is_some() {
            add(kind, host.to_string());
        }
    }
    hosts
        .into_iter()
        .map(|(kind, host)| {
            let mut account = Account::new(kind, host.clone());
            account.token = resolve(kind, &host);
            account.woodpecker = woodpecker_for(&host, file, |n| std::env::var(n).ok());
            account
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn remotes_of_a_checkout_come_origin_first_and_give_the_default_repo() {
        let dir = tempfile::tempdir().expect("tmp");
        let repo = git2::Repository::init(dir.path()).expect("init");
        repo.remote("upstream", "https://gitlab.com/u/p.git")
            .expect("remote");
        repo.remote("origin", "git@github.com:o/r.git")
            .expect("remote");
        let urls = super::remote_urls(dir.path());
        assert_eq!(
            urls,
            ["git@github.com:o/r.git", "https://gitlab.com/u/p.git"]
        );
        assert_eq!(
            super::repo_of_dir(dir.path()).map(|r| r.key()).as_deref(),
            Some("github.com/o/r")
        );
        let none = tempfile::tempdir().expect("tmp");
        assert!(super::remote_urls(none.path()).is_empty());
        assert!(super::repo_of_dir(none.path()).is_none());
    }

    #[test]
    fn a_woodpecker_server_comes_from_the_file_and_its_token_from_the_environment() {
        let file = AccountsFile::parse("[[account]]\nkind = \"gitea\"\nhost = \"codeberg.org\"\nwoodpecker = \"https://ci.codeberg.org/\"\n\n[[account]]\nkind = \"gitea\"\nhost = \"git.x.org\"\n").expect("file");
        let none = |_: &str| None;
        let w = woodpecker_for("codeberg.org", &file, none).expect("configured");
        assert_eq!(
            (w.url.as_str(), w.token.is_none()),
            ("https://ci.codeberg.org", true)
        );
        let env = |n: &str| match n {
            "GITKRAFT_WOODPECKER_TOKEN_CODEBERG_ORG" => Some(" host-token ".into()),
            "GITKRAFT_WOODPECKER_TOKEN" => Some("global".into()),
            _ => None,
        };
        let w = woodpecker_for("CODEBERG.org", &file, env).expect("configured");
        assert_eq!(w.token.as_ref().map(Secret::expose), Some("host-token"));
        let global = |n: &str| (n == "GITKRAFT_WOODPECKER_TOKEN").then(|| "global".to_string());
        assert_eq!(
            woodpecker_for("codeberg.org", &file, global)
                .and_then(|w| w.token)
                .as_ref()
                .map(Secret::expose),
            Some("global")
        );
        assert!(woodpecker_for("git.x.org", &file, none).is_none());
        assert!(woodpecker_for("unknown", &file, none).is_none());
    }

    #[test]
    fn the_oauth_client_id_comes_from_the_argument_then_env_then_the_file() {
        let file = AccountsFile::parse("[[account]]\nkind = \"gitlab\"\nhost = \"git.example.com\"\noauth_client_id = \"from-file\"\n").expect("file");
        let none = |_: &str| None;
        assert_eq!(
            oauth_client_id("git.example.com", None, none, &file).as_deref(),
            Some("from-file")
        );
        let env = |n: &str| match n {
            "GITKRAFT_OAUTH_CLIENT_ID_GIT_EXAMPLE_COM" => Some("host-env".into()),
            "GITKRAFT_OAUTH_CLIENT_ID" => Some("global-env".into()),
            _ => None,
        };
        assert_eq!(
            oauth_client_id("git.example.com", None, env, &file).as_deref(),
            Some("host-env")
        );
        assert_eq!(
            oauth_client_id("github.com", None, env, &file).as_deref(),
            Some("global-env")
        );
        assert_eq!(
            oauth_client_id("git.example.com", Some("arg"), env, &file).as_deref(),
            Some("arg")
        );
        assert_eq!(oauth_client_id("github.com", None, none, &file), None);
        assert_eq!(oauth_client_id("github.com", Some("  "), none, &file), None);
    }

    use super::*;

    fn token_for(host: &'static str) -> impl Fn(PlatformKind, &str) -> Option<Secret> {
        move |_, h| (h == host).then(|| Secret::new("t"))
    }

    #[test]
    fn hosts_come_from_remotes_deduplicated_in_order() {
        let urls = vec![
            "git@github.com:a/b.git".to_string(),
            "https://codeberg.org/c/d".to_string(),
            "https://github.com/x/y".to_string(),
            "https://unknown.example/q/r".to_string(),
        ];
        let accounts = discover(&urls, &AccountsFile::default(), |_, _| None);
        let hosts: Vec<_> = accounts.iter().map(|a| a.host.as_str()).collect();
        assert_eq!(hosts, ["github.com", "codeberg.org"]);
        assert!(accounts.iter().all(|a| a.token.is_none()));
    }

    #[test]
    fn self_hosted_hosts_need_the_file_to_be_understood() {
        let file =
            AccountsFile::parse("[[account]]\nkind = \"gitlab\"\nhost = \"git.corp.example\"\n")
                .expect("valid");
        let urls = vec!["git@git.corp.example:team/app.git".to_string()];
        let accounts = discover(&urls, &file, |_, _| None);
        assert_eq!(
            (accounts.len(), accounts[0].kind),
            (1, PlatformKind::GitLab)
        );
        // Without the file the host is unknown and skipped.
        assert!(discover(&urls, &AccountsFile::default(), |_, _| None).is_empty());
    }

    #[test]
    fn well_known_hosts_with_a_token_are_added_even_without_a_remote() {
        let accounts = discover(&[], &AccountsFile::default(), token_for("gitlab.com"));
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].host, "gitlab.com");
        assert!(accounts[0].token.is_some());
    }

    #[test]
    fn accounts_file_accepts_forgejo_and_codeberg_aliases_and_rejects_nonsense() {
        let f = AccountsFile::parse("[[account]]\nkind = \"forgejo\"\nhost = \"f.example\"\n")
            .expect("valid");
        assert_eq!(f.accounts[0].kind, PlatformKind::Gitea);
        assert!(AccountsFile::parse("[[account]]\nkind = \"svn\"\nhost = \"x\"").is_err());
        assert_eq!(
            AccountsFile::parse("").expect("empty"),
            AccountsFile::default()
        );
    }
}
