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
            account
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
