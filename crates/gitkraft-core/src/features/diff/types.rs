use serde::{Deserialize, Serialize};

/// Semantic color category for file statuses — frontends map these to
/// their framework-specific colors (e.g. `Added → green`, `Deleted → red`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColorCategory {
    /// New / untracked / added files (green / success).
    Added,
    /// Modified / typechanged files (yellow / warning).
    Modified,
    /// Deleted files (red / error).
    Deleted,
    /// Renamed / copied files (accent color).
    Renamed,
}

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

    /// Map to a semantic color category for consistent frontend rendering.
    pub fn color_category(&self) -> StatusColorCategory {
        match self {
            Self::New | Self::Untracked => StatusColorCategory::Added,
            Self::Modified | Self::Typechange => StatusColorCategory::Modified,
            Self::Deleted => StatusColorCategory::Deleted,
            Self::Renamed | Self::Copied => StatusColorCategory::Renamed,
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

impl DiffInfo {
    /// The most relevant file path to display — prefers `new_file`,
    /// falls back to `old_file` for deletions.
    pub fn display_path(&self) -> &str {
        if self.new_file.is_empty() {
            &self.old_file
        } else {
            &self.new_file
        }
    }

    /// Extract just the filename component from the display path.
    pub fn file_name(&self) -> &str {
        let path = self.display_path();
        path.rsplit('/').next().unwrap_or(path)
    }

    /// Extract the parent directory of the display path, or empty string if none.
    pub fn parent_dir(&self) -> &str {
        let path = self.display_path();
        path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
    }

    /// The last component of the parent directory (for compact display hints).
    pub fn short_parent_dir(&self) -> &str {
        let parent = self.parent_dir();
        if parent.is_empty() {
            ""
        } else {
            parent.rsplit('/').next().unwrap_or(parent)
        }
    }
}

/// Lightweight entry in a commit's changed-file list.
///
/// Contains only the file paths and status — no diff hunks or lines.
/// Used by the GUI to instantly display the file sidebar when a commit is
/// selected, before any per-file diff content is parsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFileEntry {
    /// Path of the old version of the file (may be empty for new files).
    pub old_file: String,
    /// Path of the new version of the file (may be empty for deleted files).
    pub new_file: String,
    /// The kind of change that happened to this file.
    pub status: FileStatus,
}

impl DiffFileEntry {
    /// The most relevant file path to display — prefers `new_file`,
    /// falls back to `old_file` for deletions.
    pub fn display_path(&self) -> &str {
        if self.new_file.is_empty() {
            &self.old_file
        } else {
            &self.new_file
        }
    }

    /// Extract just the filename component from the display path.
    pub fn file_name(&self) -> &str {
        let path = self.display_path();
        path.rsplit('/').next().unwrap_or(path)
    }

    /// Extract the parent directory of the display path, or empty string if none.
    pub fn parent_dir(&self) -> &str {
        let path = self.display_path();
        path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("")
    }

    /// The last component of the parent directory (for compact display hints).
    pub fn short_parent_dir(&self) -> &str {
        let parent = self.parent_dir();
        if parent.is_empty() {
            ""
        } else {
            parent.rsplit('/').next().unwrap_or(parent)
        }
    }
}
