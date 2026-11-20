//! Accounts: one platform host plus the credential used on it.

use crate::platform::PlatformKind;

/// A token that never prints itself.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The raw token, for building a request header.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// One login on one host. The unit dashboards, caches and notification state hang off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// Platform kind.
    pub kind: PlatformKind,
    /// Host, e.g. `github.com`.
    pub host: String,
    /// The token, if any (public reads work without).
    pub token: Option<Secret>,
    /// Your login on this host; filled by `current_user`.
    pub me: Option<String>,
    /// A Woodpecker CI server that builds this host's repositories (Codeberg uses one).
    pub woodpecker: Option<Woodpecker>,
    /// Where the forge's web server is when it is not `https://<host>`: a self-hosted one on
    /// plain http or another port, e.g. `http://192.168.1.44:3000`. The remotes of a checkout
    /// name only the SSH host, so this is the one place the port and scheme can come from.
    pub base_url: Option<String>,
}

/// A Woodpecker CI server and your token on it (a different token from the forge's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Woodpecker {
    /// Server URL, e.g. `https://ci.codeberg.org`.
    pub url: String,
    /// Personal access token from the server's user page.
    pub token: Option<Secret>,
}

impl Account {
    /// An account without credentials.
    pub fn new(kind: PlatformKind, host: impl Into<String>) -> Self {
        Self {
            kind,
            host: host.into().to_lowercase(),
            token: None,
            me: None,
            woodpecker: None,
            base_url: None,
        }
    }

    /// Builds with a Woodpecker CI server.
    pub fn with_woodpecker(mut self, url: impl Into<String>, token: Option<Secret>) -> Self {
        self.woodpecker = Some(Woodpecker {
            url: url.into().trim_end_matches('/').to_string(),
            token,
        });
        self
    }

    /// Sets the token.
    pub fn with_token(mut self, token: Secret) -> Self {
        self.token = Some(token);
        self
    }

    /// Sets where the forge's web server is (see [`Account::base_url`]).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        let url: String = url.into();
        let url = url.trim().trim_end_matches('/');
        self.base_url = (!url.is_empty()).then(|| url.to_string());
        self
    }

    /// REST base URL.
    pub fn api_base(&self) -> String {
        match &self.base_url {
            Some(url) => format!("{url}{}", self.kind.api_path(&self.host)),
            None => self.kind.api_base(&self.host),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_never_prints() {
        let account =
            Account::new(PlatformKind::GitHub, "GitHub.com").with_token(Secret::new("ghp_abc"));
        let shown = format!("{account:?}");
        assert!(!shown.contains("ghp_abc"));
        assert_eq!(account.host, "github.com");
        let own = Account::new(PlatformKind::Gitea, "192.168.1.44")
            .with_base_url("http://192.168.1.44:3000/");
        assert_eq!(own.api_base(), "http://192.168.1.44:3000/api/v1");
        let gl = Account::new(PlatformKind::GitLab, "git.x").with_base_url("https://git.x/gitlab");
        assert_eq!(
            gl.api_base(),
            "https://git.x/gitlab/api/v4",
            "a sub-path install"
        );
        assert_eq!(
            Account::new(PlatformKind::Gitea, "h")
                .with_base_url(" ")
                .base_url,
            None
        );
        assert_eq!(account.token.as_ref().map(Secret::expose), Some("ghp_abc"));
    }
}
