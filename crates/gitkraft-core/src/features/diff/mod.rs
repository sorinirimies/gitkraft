//! Diff feature — types and operations for working-directory, staged, and
//! per-commit diffs.

pub mod ops;
pub mod types;

pub use ops::{
    checkout_file_at_commit, diff_file_commit_vs_workdir, file_list_commit_vs_workdir,
    get_commit_diff, get_commit_file_list, get_commit_range_diff, get_single_file_diff,
    get_staged_diff, get_working_dir_diff,
};
pub use types::{DiffFileEntry, DiffHunk, DiffInfo, DiffLine, FileStatus, StatusColorCategory};
