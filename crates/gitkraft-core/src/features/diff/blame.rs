//! Git blame — line-by-line commit attribution for a file.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};

/// A single blamed line: the line content plus the commit that last modified it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    /// 1-based line number.
    pub line_number: usize,
    /// Raw line content (no trailing newline).
    pub content: String,
    /// Abbreviated commit OID (7 chars).
    pub short_oid: String,
    /// Full commit OID.
    pub oid: String,
    /// Author name.
    pub author_name: String,
    /// Commit timestamp (UTC).
    pub time: DateTime<Utc>,
}

impl BlameLine {
    /// Human-readable relative time (delegates to the core utils).
    pub fn relative_time(&self) -> String {
        crate::utils::relative_time(self.time)
    }
}

/// Produce a `BlameLine` for every line in `file_path` relative to HEAD.
///
/// Reads the working-tree copy of the file for line content; uses libgit2's
/// blame API for attribution.
pub fn blame_file(repo: &Repository, file_path: &str) -> Result<Vec<BlameLine>> {
    let blame = repo
        .blame_file(std::path::Path::new(file_path), None)
        .with_context(|| format!("failed to blame '{file_path}'"))?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("bare repository has no working directory"))?;

    let content = std::fs::read_to_string(workdir.join(file_path))
        .with_context(|| format!("failed to read '{file_path}' for blame"))?;

    let mut lines = Vec::new();
    for (idx, line_content) in content.lines().enumerate() {
        let line_number = idx + 1;
        if let Some(hunk) = blame.get_line(line_number) {
            let commit_id = hunk.final_commit_id();
            let oid = commit_id.to_string();
            let short_oid = oid[..7.min(oid.len())].to_string();

            let sig = hunk.final_signature();
            let author_name = sig.name().unwrap_or("?").to_string();
            let time = DateTime::<Utc>::from_timestamp(sig.when().seconds(), 0).unwrap_or_default();

            lines.push(BlameLine {
                line_number,
                content: line_content.to_string(),
                short_oid,
                oid,
                author_name,
                time,
            });
        }
    }

    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_repo_with_file() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        std::fs::write(
            dir.path().join("file.txt"),
            "line one\nline two\nline three\n",
        )
        .unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn blame_file_returns_one_entry_per_line() {
        let (_dir, repo) = setup_repo_with_file();
        let lines = blame_file(&repo, "file.txt").unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_number, 1);
        assert_eq!(lines[0].content, "line one");
        assert_eq!(lines[1].line_number, 2);
        assert_eq!(lines[2].line_number, 3);
        assert_eq!(lines[2].content, "line three");
    }

    #[test]
    fn blame_file_populates_oid_and_author() {
        let (_dir, repo) = setup_repo_with_file();
        let lines = blame_file(&repo, "file.txt").unwrap();
        for line in &lines {
            assert_eq!(line.short_oid.len(), 7);
            assert!(!line.oid.is_empty());
            assert_eq!(line.author_name, "Test User");
        }
    }

    #[test]
    fn blame_file_nonexistent_returns_error() {
        let (_dir, repo) = setup_repo_with_file();
        let result = blame_file(&repo, "nonexistent.txt");
        assert!(result.is_err());
    }

    #[test]
    fn blame_line_relative_time_nonempty() {
        let (_dir, repo) = setup_repo_with_file();
        let lines = blame_file(&repo, "file.txt").unwrap();
        assert!(!lines[0].relative_time().is_empty());
    }
}
