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
