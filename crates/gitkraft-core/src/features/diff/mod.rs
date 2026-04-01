//! Diff feature — types and operations for working-directory, staged, and
//! per-commit diffs.

pub mod ops;
pub mod types;

pub use ops::{get_commit_diff, get_staged_diff, get_working_dir_diff};
pub use types::{DiffHunk, DiffInfo, DiffLine, FileStatus};
