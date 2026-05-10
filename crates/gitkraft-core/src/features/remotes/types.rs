use serde::{Deserialize, Serialize};

/// Information about a configured Git remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    /// Remote name (e.g. `origin`).
    pub name: String,
    /// URL the remote points to (may be `None` for anonymous remotes).
    pub url: Option<String>,
    /// Fetch refspecs configured for this remote.
    pub fetch_refspecs: Vec<String>,
}

impl RemoteInfo {
    /// The URL to display in the UI, falling back to `"<no url>"` when absent.
    pub fn display_url(&self) -> &str {
        self.url.as_deref().unwrap_or("<no url>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_url_with_url() {
        let r = RemoteInfo {
            name: "origin".into(),
            url: Some("https://example.com".into()),
            fetch_refspecs: vec![],
        };
        assert_eq!(r.display_url(), "https://example.com");
    }

    #[test]
    fn display_url_without_url() {
        let r = RemoteInfo {
            name: "origin".into(),
            url: None,
            fetch_refspecs: vec![],
        };
        assert_eq!(r.display_url(), "<no url>");
    }
}
