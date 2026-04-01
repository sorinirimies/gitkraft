use serde::{Deserialize, Serialize};

/// Status of a file within a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Typechange,
    Untracked,
}

impl FileStatus {
    /// Convert a `git2::Delta` into our domain `FileStatus`.
    pub fn from_delta(delta: git2::Delta) -> Self {
        match delta {
            git2::Delta::Added => FileStatus::New,
            git2::Delta::Modified => FileStatus::Modified,
            git2::Delta::Deleted => FileStatus::Deleted,
            git2::Delta::Renamed => FileStatus::Renamed,
            git2::Delta::Copied => FileStatus::Copied,
            git2::Delta::Typechange => FileStatus::Typechange,
            git2::Delta::Untracked => FileStatus::Untracked,
            // Treat anything else (Ignored, Conflicted, Unreadable, Unmodified) as Modified
            _ => FileStatus::Modified,
        }
    }
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = match self {
            Self::New => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Copied => "C",
            Self::Typechange => "T",
            Self::Untracked => "?",
        };
        write!(f, "{ch}")
    }
}

/// A single line inside a diff hunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    /// Unchanged context line.
    Context(String),
    /// Added line.
    Addition(String),
    /// Removed line.
    Deletion(String),
    /// Hunk header (e.g. `@@ -10,7 +10,6 @@`).
    HunkHeader(String),
}

/// A contiguous hunk of changes within a single file diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// The hunk header string (e.g. `@@ -10,7 +10,6 @@`).
    pub header: String,
    /// The individual lines that make up this hunk.
    pub lines: Vec<DiffLine>,
}

/// Diff information for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInfo {
    /// Path of the old version of the file (may be empty for new / untracked files).
    pub old_file: String,
    /// Path of the new version of the file (may be empty for deleted files).
    pub new_file: String,
    /// The kind of change that happened to this file.
    pub status: FileStatus,
    /// Hunks that make up the diff for this file.
    pub hunks: Vec<DiffHunk>,
}
