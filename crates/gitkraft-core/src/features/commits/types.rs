use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}
