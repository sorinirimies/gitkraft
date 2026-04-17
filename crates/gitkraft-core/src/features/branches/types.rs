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
            self.name
                .split_once('/')
                .map(|(_, b)| b)
                .unwrap_or(&self.name)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn local_branch(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            branch_type: BranchType::Local,
            is_head: false,
            target_oid: Some("abcdef1234567890".to_string()),
        }
    }

    fn remote_branch(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            branch_type: BranchType::Remote,
            is_head: false,
            target_oid: Some("1234567890abcdef".to_string()),
        }
    }

    #[test]
    fn is_remote_local() {
        assert!(!local_branch("main").is_remote());
    }

    #[test]
    fn is_remote_remote() {
        assert!(remote_branch("origin/main").is_remote());
    }

    #[test]
    fn remote_name_local_returns_none() {
        assert_eq!(local_branch("main").remote_name(), None);
    }

    #[test]
    fn remote_name_remote_returns_remote() {
        assert_eq!(
            remote_branch("origin/feature").remote_name(),
            Some("origin")
        );
    }

    #[test]
    fn short_name_local_unchanged() {
        assert_eq!(local_branch("main").short_name(), "main");
    }

    #[test]
    fn short_name_remote_strips_prefix() {
        assert_eq!(remote_branch("origin/feature-x").short_name(), "feature-x");
    }

    #[test]
    fn short_name_remote_nested_slash() {
        // Only the first component is the remote name
        assert_eq!(remote_branch("origin/feat/sub").short_name(), "feat/sub");
    }

    #[test]
    fn short_oid_returns_7_chars() {
        let b = local_branch("main");
        assert_eq!(b.short_oid(), Some("abcdef1"));
    }

    #[test]
    fn short_oid_none_when_no_target() {
        let mut b = local_branch("main");
        b.target_oid = None;
        assert_eq!(b.short_oid(), None);
    }

    #[test]
    fn branch_type_display_local() {
        assert_eq!(format!("{}", BranchType::Local), "local");
    }

    #[test]
    fn branch_type_display_remote() {
        assert_eq!(format!("{}", BranchType::Remote), "remote");
    }
}
