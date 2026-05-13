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
    /// Commits the local branch is ahead of its upstream tracking branch.
    /// `None` for remote branches or locals with no upstream configured.
    #[serde(default)]
    pub upstream_ahead: Option<usize>,
    /// Commits the local branch is behind its upstream tracking branch.
    /// `None` for remote branches or locals with no upstream configured.
    #[serde(default)]
    pub upstream_behind: Option<usize>,
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

    /// Format the ahead/behind counts as a display string.
    ///
    /// Returns `None` when there is no upstream tracking info.
    /// Returns `Some("✓")` when fully in sync (0 ahead, 0 behind).
    /// Otherwise returns something like `"↑3 ↓2"`, `"↑3"`, or `"↓2"`.
    pub fn upstream_status(&self) -> Option<String> {
        match (self.upstream_ahead, self.upstream_behind) {
            (Some(0), Some(0)) => Some("✓".to_string()),
            (Some(ahead), Some(behind)) => {
                let mut s = String::new();
                if ahead > 0 {
                    s.push_str(&format!("↑{ahead}"));
                }
                if behind > 0 {
                    if !s.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(&format!("↓{behind}"));
                }
                Some(s)
            }
            (Some(ahead), None) if ahead > 0 => Some(format!("↑{ahead}")),
            (None, Some(behind)) if behind > 0 => Some(format!("↓{behind}")),
            _ => None,
        }
    }
}

/// Validate a proposed Git ref name (branch or tag).
///
/// Returns `Ok(())` if the name is valid, or `Err` with a human-readable
/// error message explaining why it's invalid.
///
/// This mirrors the rules of `git check-ref-format` and provides friendlier
/// messages than libgit2's generic "failed to create reference" errors.
pub fn validate_ref_name(name: &str) -> Result<(), &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("name cannot be empty");
    }
    if trimmed.starts_with('.') {
        return Err("name cannot start with '.'");
    }
    if trimmed.starts_with('-') {
        return Err("name cannot start with '-'");
    }
    if trimmed.ends_with('.') {
        return Err("name cannot end with '.'");
    }
    if trimmed.ends_with(".lock") {
        return Err("name cannot end with '.lock'");
    }
    if trimmed == "@" {
        return Err("name cannot be '@'");
    }
    if trimmed.contains("..") {
        return Err("name cannot contain '..'");
    }
    if trimmed.contains("//") {
        return Err("name cannot contain '//'");
    }
    if trimmed.contains("@{") {
        return Err("name cannot contain '@{'");
    }
    const INVALID_CHARS: &[char] = &[' ', '~', '^', ':', '?', '*', '[', '\\'];
    for ch in INVALID_CHARS {
        if trimmed.contains(*ch) {
            return Err(match ch {
                ' ' => "name cannot contain spaces",
                '~' => "name cannot contain '~'",
                '^' => "name cannot contain '^'",
                ':' => "name cannot contain ':'",
                '?' => "name cannot contain '?'",
                '*' => "name cannot contain '*'",
                '[' => "name cannot contain '['",
                '\\' => "name cannot contain '\\'",
                _ => "name contains invalid characters",
            });
        }
    }
    // Check for ASCII control characters
    if trimmed.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err("name cannot contain control characters");
    }
    Ok(())
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
            upstream_ahead: None,
            upstream_behind: None,
        }
    }

    fn remote_branch(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            branch_type: BranchType::Remote,
            is_head: false,
            target_oid: Some("1234567890abcdef".to_string()),
            upstream_ahead: None,
            upstream_behind: None,
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

    #[test]
    fn upstream_status_in_sync() {
        let mut b = local_branch("main");
        b.upstream_ahead = Some(0);
        b.upstream_behind = Some(0);
        assert_eq!(b.upstream_status(), Some("✓".to_string()));
    }

    #[test]
    fn upstream_status_ahead_only() {
        let mut b = local_branch("main");
        b.upstream_ahead = Some(3);
        b.upstream_behind = Some(0);
        assert_eq!(b.upstream_status(), Some("↑3".to_string()));
    }

    #[test]
    fn upstream_status_behind_only() {
        let mut b = local_branch("main");
        b.upstream_ahead = Some(0);
        b.upstream_behind = Some(5);
        assert_eq!(b.upstream_status(), Some("↓5".to_string()));
    }

    #[test]
    fn upstream_status_both() {
        let mut b = local_branch("main");
        b.upstream_ahead = Some(2);
        b.upstream_behind = Some(4);
        assert_eq!(b.upstream_status(), Some("↑2 ↓4".to_string()));
    }

    #[test]
    fn upstream_status_no_upstream() {
        let b = local_branch("main");
        assert_eq!(b.upstream_status(), None);
    }

    // ── validate_ref_name ─────────────────────────────────────────────────

    #[test]
    fn validate_ref_name_valid_simple() {
        assert!(validate_ref_name("main").is_ok());
    }

    #[test]
    fn validate_ref_name_valid_with_slash() {
        assert!(validate_ref_name("feature/my-branch").is_ok());
    }

    #[test]
    fn validate_ref_name_valid_with_dots() {
        assert!(validate_ref_name("v1.0.0").is_ok());
    }

    #[test]
    fn validate_ref_name_empty() {
        assert!(validate_ref_name("").is_err());
    }

    #[test]
    fn validate_ref_name_whitespace_only() {
        assert!(validate_ref_name("   ").is_err());
    }

    #[test]
    fn validate_ref_name_starts_with_dot() {
        assert!(validate_ref_name(".hidden").is_err());
    }

    #[test]
    fn validate_ref_name_starts_with_dash() {
        assert!(validate_ref_name("-flag").is_err());
    }

    #[test]
    fn validate_ref_name_ends_with_dot() {
        assert!(validate_ref_name("branch.").is_err());
    }

    #[test]
    fn validate_ref_name_ends_with_lock() {
        assert!(validate_ref_name("branch.lock").is_err());
    }

    #[test]
    fn validate_ref_name_contains_space() {
        assert!(validate_ref_name("my branch").is_err());
    }

    #[test]
    fn validate_ref_name_contains_tilde() {
        assert!(validate_ref_name("branch~1").is_err());
    }

    #[test]
    fn validate_ref_name_contains_caret() {
        assert!(validate_ref_name("branch^2").is_err());
    }

    #[test]
    fn validate_ref_name_contains_colon() {
        assert!(validate_ref_name("branch:name").is_err());
    }

    #[test]
    fn validate_ref_name_double_dot() {
        assert!(validate_ref_name("branch..other").is_err());
    }

    #[test]
    fn validate_ref_name_at_sign() {
        assert!(validate_ref_name("@").is_err());
    }

    #[test]
    fn validate_ref_name_at_brace() {
        assert!(validate_ref_name("branch@{0}").is_err());
    }

    #[test]
    fn validate_ref_name_question_mark() {
        assert!(validate_ref_name("branch?").is_err());
    }

    #[test]
    fn validate_ref_name_asterisk() {
        assert!(validate_ref_name("branch*").is_err());
    }

    #[test]
    fn validate_ref_name_backslash() {
        assert!(validate_ref_name("branch\\name").is_err());
    }
}
