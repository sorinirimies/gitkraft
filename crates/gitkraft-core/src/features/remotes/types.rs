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
