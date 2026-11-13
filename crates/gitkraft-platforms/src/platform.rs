//! Platform kinds, repository identity and remote URL parsing.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The kind of platform behind a host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformKind {
    /// github.com or GitHub Enterprise.
    GitHub,
    /// gitlab.com or self-hosted GitLab.
    GitLab,
    /// Gitea, Forgejo, Codeberg.
    #[serde(alias = "forgejo", alias = "codeberg")]
    Gitea,
}

impl PlatformKind {
    /// Short name for the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Gitea => "Gitea",
        }
    }

    /// REST base URL for `host`.
    pub fn api_base(self, host: &str) -> String {
        match self {
            Self::GitHub if host == "github.com" => "https://api.github.com".into(),
            Self::GitHub => format!("https://{host}/api/v3"),
            Self::GitLab => format!("https://{host}/api/v4"),
            Self::Gitea => format!("https://{host}/api/v1"),
        }
    }
}

/// A repository on a host: `host/owner/name`. GitLab groups may nest, so
/// `owner` can contain slashes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RepoId {
    /// Host, lowercase, e.g. `github.com`.
    pub host: String,
    /// User, organisation or GitLab group path.
    pub owner: String,
    /// Repository name without `.git`.
    pub name: String,
}

impl RepoId {
    /// Builds an id.
    pub fn new(host: impl Into<String>, owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            host: host.into().to_lowercase(),
            owner: owner.into(),
            name: name.into(),
        }
    }

    /// `owner/name`.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// `host/owner/name`, the namespaced id used for caches and state.
    pub fn key(&self) -> String {
        format!("{}/{}/{}", self.host, self.owner, self.name)
    }

    /// Parses `host/owner/name`, the inverse of [`RepoId::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        let (host, rest) = key.split_once('/')?;
        let (owner, name) = rest.rsplit_once('/')?;
        (!host.is_empty() && !owner.is_empty() && !name.is_empty())
            .then(|| Self::new(host, owner, name))
    }
}

impl std::fmt::Display for RepoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.key())
    }
}

/// Parses a git remote URL (https, ssh, scp-like, `ssh://`) into a [`RepoId`].
pub fn parse_remote_url(url: &str) -> Option<RepoId> {
    let url = url.trim();
    let (host, path) = if let Some(rest) = url.split_once("://").map(|(_, r)| r) {
        // https://user@host:port/owner/name(.git)  or  ssh://git@host/owner/name
        let (authority, path) = rest.split_once('/')?;
        let authority = authority.rsplit('@').next()?;
        let host = authority.split(':').next()?;
        (host, path)
    } else {
        // git@host:owner/name.git
        let (authority, path) = url.split_once(':')?;
        (authority.rsplit('@').next()?, path)
    };
    let path = path
        .trim_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/');
    let (owner, name) = path.rsplit_once('/')?;
    if host.is_empty() || owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(RepoId::new(host, owner, name))
}

/// Percent-encodes everything except unreserved characters (GitLab project paths, query values).
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Maps hosts to platform kinds: the well-known ones plus the user's own.
#[derive(Debug, Clone, Default)]
pub struct HostMap {
    overrides: HashMap<String, PlatformKind>,
}

impl HostMap {
    /// Only the well-known hosts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a self-hosted host (overrides the built-ins).
    pub fn with(mut self, host: impl Into<String>, kind: PlatformKind) -> Self {
        self.overrides.insert(host.into().to_lowercase(), kind);
        self
    }

    /// The platform kind of `host`, or `None` when unknown.
    pub fn detect(&self, host: &str) -> Option<PlatformKind> {
        let host = host.to_lowercase();
        if let Some(kind) = self.overrides.get(&host) {
            return Some(*kind);
        }
        match host.as_str() {
            "github.com" => Some(PlatformKind::GitHub),
            "gitlab.com" => Some(PlatformKind::GitLab),
            "codeberg.org" | "gitea.com" | "try.gitea.io" => Some(PlatformKind::Gitea),
            h if h.starts_with("gitlab.") => Some(PlatformKind::GitLab),
            h if h.starts_with("gitea.") || h.starts_with("forgejo.") => Some(PlatformKind::Gitea),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_ssh_and_scp_urls() {
        let want = RepoId::new("github.com", "sorinirimies", "gitkraft");
        for url in [
            "https://github.com/sorinirimies/gitkraft.git",
            "https://github.com/sorinirimies/gitkraft",
            "https://user:tok@github.com/sorinirimies/gitkraft.git/",
            "git@github.com:sorinirimies/gitkraft.git",
            "ssh://git@github.com/sorinirimies/gitkraft.git",
            "ssh://git@github.com:22/sorinirimies/gitkraft.git",
        ] {
            assert_eq!(parse_remote_url(url), Some(want.clone()), "{url}");
        }
    }

    #[test]
    fn nested_gitlab_groups_keep_the_full_owner() {
        let id = parse_remote_url("git@gitlab.com:group/sub/proj.git").unwrap_or_default();
        assert_eq!(id.owner, "group/sub");
        assert_eq!(id.name, "proj");
    }

    #[test]
    fn rejects_garbage() {
        for url in [
            "",
            "nonsense",
            "https://github.com/",
            "https://github.com/only",
        ] {
            assert_eq!(parse_remote_url(url), None, "{url}");
        }
    }

    #[test]
    fn host_is_lowercased_and_key_round_trips() {
        let id = RepoId::new("GitHub.com", "a", "b");
        assert_eq!(id.host, "github.com");
        assert_eq!(RepoId::from_key(&id.key()), Some(id));
        assert_eq!(RepoId::from_key("x"), None);
        assert_eq!(RepoId::from_key("h/o/"), None);
    }

    #[test]
    fn detects_known_hosts_and_overrides() {
        let map = HostMap::new().with("git.corp.example", PlatformKind::GitLab);
        assert_eq!(map.detect("github.com"), Some(PlatformKind::GitHub));
        assert_eq!(map.detect("Codeberg.org"), Some(PlatformKind::Gitea));
        assert_eq!(map.detect("gitlab.internal"), Some(PlatformKind::GitLab));
        assert_eq!(map.detect("git.corp.example"), Some(PlatformKind::GitLab));
        assert_eq!(map.detect("example.com"), None);
    }

    #[test]
    fn url_encode_escapes_reserved_characters() {
        assert_eq!(url_encode("a/b c~d"), "a%2Fb%20c~d");
        assert_eq!(url_encode("é"), "%C3%A9");
    }

    #[test]
    fn api_bases() {
        assert_eq!(
            PlatformKind::GitHub.api_base("github.com"),
            "https://api.github.com"
        );
        assert_eq!(
            PlatformKind::GitHub.api_base("ghe.corp"),
            "https://ghe.corp/api/v3"
        );
        assert_eq!(
            PlatformKind::Gitea.api_base("codeberg.org"),
            "https://codeberg.org/api/v1"
        );
        assert_eq!(
            PlatformKind::GitLab.api_base("gitlab.com"),
            "https://gitlab.com/api/v4"
        );
    }
}
