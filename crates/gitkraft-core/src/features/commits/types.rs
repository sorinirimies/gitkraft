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
}
