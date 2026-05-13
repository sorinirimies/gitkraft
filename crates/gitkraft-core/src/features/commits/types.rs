use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The kind of Git ref a [`RefLabel`] represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefKind {
    /// The branch currently checked out (HEAD points to this branch).
    Head,
    /// A local branch that is NOT the current HEAD.
    LocalBranch,
    /// A remote-tracking branch (e.g. `origin/main`).
    RemoteBranch,
    /// A lightweight or annotated tag.
    Tag,
}

impl RefKind {
    /// The semantic colour from [`AppTheme`](crate::AppTheme) for this ref kind.
    ///
    /// Both the GUI and TUI convert the returned [`Rgb`](crate::Rgb) into
    /// their framework-specific colour type, keeping the mapping in one place.
    pub fn color(&self, theme: &crate::AppTheme) -> crate::Rgb {
        match self {
            RefKind::Head => theme.accent,
            RefKind::LocalBranch => theme.success,
            RefKind::RemoteBranch => theme.warning,
            RefKind::Tag => theme.text_muted,
        }
    }
}

/// A human-readable Git reference attached to a commit for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefLabel {
    /// Short display name (e.g. `main`, `origin/main`, `v1.0.0`).
    pub name: String,
    /// Category that drives the badge colour in the commit log UI.
    pub kind: RefKind,
}

/// Full metadata for a single Git commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Full hex-encoded object id.
    pub oid: String,
    /// Abbreviated OID (first 7 characters).
    pub short_oid: String,
    /// First line of the commit message.
    pub summary: String,
    /// Full commit message (includes summary).
    pub message: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Commit timestamp (UTC).
    pub time: DateTime<Utc>,
    /// Hex-encoded parent object ids.
    pub parent_ids: Vec<String>,
    /// Branch / tag / HEAD labels that point directly at this commit.
    /// Populated only when loaded via `list_commits` or `get_log`.
    /// Most commits have an empty vec here.
    #[serde(default)]
    pub refs: Vec<RefLabel>,
}

impl CommitInfo {
    /// Build a `CommitInfo` from a `git2::Commit`.
    pub fn from_git2_commit(commit: &git2::Commit<'_>) -> Self {
        let oid = commit.id().to_string();
        let short_oid = oid[..7.min(oid.len())].to_string();

        let time_secs = commit.time().seconds();
        let time = DateTime::<Utc>::from_timestamp(time_secs, 0).unwrap_or_default();

        let summary = commit.summary().unwrap_or("").to_string();
        let message = commit.message().unwrap_or("").to_string();

        let author = commit.author();
        let author_name = author.name().unwrap_or("").to_string();
        let author_email = author.email().unwrap_or("").to_string();

        let parent_ids = (0..commit.parent_count())
            .filter_map(|i| commit.parent_id(i).ok())
            .map(|oid| oid.to_string())
            .collect();

        Self {
            oid,
            short_oid,
            summary,
            message,
            author_name,
            author_email,
            time,
            parent_ids,
            refs: Vec::new(),
        }
    }

    /// Whether this is a merge commit (2+ parents).
    pub fn is_merge(&self) -> bool {
        self.parent_ids.len() > 1
    }

    /// Human-readable relative time (e.g. "3 hours ago").
    pub fn relative_time(&self) -> String {
        crate::utils::relative_time(self.time)
    }

    /// Summary truncated to `max_chars` with "..." appended if shortened.
    pub fn short_summary(&self, max_chars: usize) -> String {
        crate::utils::truncate_str(&self.summary, max_chars)
    }
}

/// Severity level for the commit message first-line length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitMsgSeverity {
    /// Within the conventional subject-line budget (≤ 50 chars).
    Good,
    /// Approaching the hard limit (51–82 chars) — acceptable but verbose.
    Warning,
    /// Exceeds the recommended limit (> 82 chars).
    TooLong,
}

/// Maximum recommended first-line length for a commit message.
pub const COMMIT_SUBJECT_LIMIT: usize = 82;

/// Threshold below which the first-line length is considered "good".
pub const COMMIT_SUBJECT_SOFT_LIMIT: usize = 50;

/// Analyse the first line of a commit message and return its length + severity.
///
/// Both the GUI and TUI use this to show a character counter with colour feedback.
pub fn check_commit_message(message: &str) -> (usize, CommitMsgSeverity) {
    let first_line = message.lines().next().unwrap_or("");
    let len = first_line.chars().count();
    let severity = if len > COMMIT_SUBJECT_LIMIT {
        CommitMsgSeverity::TooLong
    } else if len > COMMIT_SUBJECT_SOFT_LIMIT {
        CommitMsgSeverity::Warning
    } else {
        CommitMsgSeverity::Good
    };
    (len, severity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_commit(parents: usize, summary: &str, author: &str) -> CommitInfo {
        CommitInfo {
            oid: "abcdef1234567890".to_string(),
            short_oid: "abcdef1".to_string(),
            summary: summary.to_string(),
            message: summary.to_string(),
            author_name: author.to_string(),
            author_email: "test@test.com".to_string(),
            time: Utc::now(),
            parent_ids: (0..parents).map(|i| format!("parent{i}")).collect(),
            refs: Vec::new(),
        }
    }

    #[test]
    fn is_merge_false_for_single_parent() {
        assert!(!make_commit(1, "fix", "alice").is_merge());
    }

    #[test]
    fn is_merge_true_for_two_parents() {
        assert!(make_commit(2, "merge", "alice").is_merge());
    }

    #[test]
    fn is_merge_false_for_root() {
        assert!(!make_commit(0, "init", "alice").is_merge());
    }

    #[test]
    fn short_summary_fits() {
        let c = make_commit(1, "short", "alice");
        assert_eq!(c.short_summary(20), "short");
    }

    #[test]
    fn short_summary_truncates() {
        let c = make_commit(1, "a very long commit summary message", "alice");
        let s = c.short_summary(10);
        assert_eq!(s.chars().count(), 10);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn relative_time_returns_nonempty() {
        let c = make_commit(1, "fix", "alice");
        assert!(!c.relative_time().is_empty());
    }

    #[test]
    fn ref_kind_color_maps_correctly() {
        let theme = crate::theme_by_index(0);
        assert_eq!(RefKind::Head.color(&theme), theme.accent);
        assert_eq!(RefKind::LocalBranch.color(&theme), theme.success);
        assert_eq!(RefKind::RemoteBranch.color(&theme), theme.warning);
        assert_eq!(RefKind::Tag.color(&theme), theme.text_muted);
    }

    // ── check_commit_message ──────────────────────────────────────────────

    #[test]
    fn commit_msg_empty_is_good() {
        let (len, sev) = check_commit_message("");
        assert_eq!(len, 0);
        assert_eq!(sev, CommitMsgSeverity::Good);
    }

    #[test]
    fn commit_msg_short_is_good() {
        let (len, sev) = check_commit_message("fix typo");
        assert_eq!(len, 8);
        assert_eq!(sev, CommitMsgSeverity::Good);
    }

    #[test]
    fn commit_msg_at_soft_limit_is_good() {
        let msg = "a".repeat(COMMIT_SUBJECT_SOFT_LIMIT);
        let (len, sev) = check_commit_message(&msg);
        assert_eq!(len, 50);
        assert_eq!(sev, CommitMsgSeverity::Good);
    }

    #[test]
    fn commit_msg_above_soft_limit_is_warning() {
        let msg = "a".repeat(COMMIT_SUBJECT_SOFT_LIMIT + 1);
        let (_, sev) = check_commit_message(&msg);
        assert_eq!(sev, CommitMsgSeverity::Warning);
    }

    #[test]
    fn commit_msg_at_hard_limit_is_warning() {
        let msg = "a".repeat(COMMIT_SUBJECT_LIMIT);
        let (len, sev) = check_commit_message(&msg);
        assert_eq!(len, 82);
        assert_eq!(sev, CommitMsgSeverity::Warning);
    }

    #[test]
    fn commit_msg_above_hard_limit_is_too_long() {
        let msg = "a".repeat(COMMIT_SUBJECT_LIMIT + 1);
        let (_, sev) = check_commit_message(&msg);
        assert_eq!(sev, CommitMsgSeverity::TooLong);
    }

    #[test]
    fn commit_msg_multiline_only_checks_first_line() {
        let msg = "short subject\n\nthis is a very long body line that goes way beyond any limit and should not affect the severity at all";
        let (len, sev) = check_commit_message(msg);
        assert_eq!(len, 13); // "short subject"
        assert_eq!(sev, CommitMsgSeverity::Good);
    }
}
