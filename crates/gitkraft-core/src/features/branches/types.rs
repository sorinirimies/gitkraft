use serde::{Deserialize, Serialize};

/// Whether a branch lives locally or tracks a remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchType {
    Local,
    Remote,
}

/// Lightweight snapshot of a single Git branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Branch name (e.g. `main`, `origin/main`).
    pub name: String,
    /// Whether this is a local or remote-tracking branch.
    pub branch_type: BranchType,
    /// `true` when this branch is the current HEAD.
    pub is_head: bool,
    /// The OID (hex string) the branch tip points to, if resolvable.
    pub target_oid: Option<String>,
}


impl BranchInfo {
    /// Whether this is a remote-tracking branch.
    pub fn is_remote(&self) -> bool {
        self.branch_type == BranchType::Remote
    }

    /// For remote branches like `"origin/feature"`, returns `Some("origin")`.
    /// For local branches, returns `None`.
    pub fn remote_name(&self) -> Option<&str> {
        if self.is_remote() {
            self.name.split_once('/').map(|(r, _)| r)
        } else {
            None
        }
    }

    /// For remote branches like `"origin/feature"`, returns `"feature"`.
    /// For local branches, returns the name unchanged.
    pub fn short_name(&self) -> &str {
        if self.is_remote() {
            self.name.split_once('/').map(|(_, b)| b).unwrap_or(&self.name)
        } else {
            &self.name
        }
    }

    /// Abbreviated OID of the branch tip (first 7 chars), if available.
    pub fn short_oid(&self) -> Option<&str> {
        self.target_oid.as_deref().map(crate::utils::short_oid_str)
    }
}

impl std::fmt::Display for BranchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Remote => write!(f, "remote"),
        }
    }
}
