//! GitKraft Core
//!
//! Shared, framework-free logic reused by both the Iced GUI and Ratatui TUI.
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`features`] | Git operations and types grouped by feature — repo, branches, commits, diff, staging, remotes, stash, log |
//! | [`utils`] | Helpers — relative time formatting, OID formatting, text truncation |
//!
//! This crate has NO GUI or TUI dependencies.

pub mod features;
pub mod utils;

// Convenience re-exports
pub use features::branches::{BranchInfo, BranchType};
pub use features::commits::CommitInfo;
pub use features::diff::{DiffFileEntry, DiffHunk, DiffInfo, DiffLine, FileStatus, StatusColorCategory};
pub use features::graph::{GraphEdge, GraphRow};
pub use features::persistence::{AppSettings, LayoutSettings, RepoHistoryEntry};
pub use features::remotes::RemoteInfo;
pub use features::repo::{RepoInfo, RepoState};
pub use features::stash::StashEntry;
pub use features::theme::{
    theme_by_index, theme_index_by_name, AppTheme, Rgb, THEME_COUNT, THEME_NAMES,
};
pub use utils::text::truncate_str;
pub use utils::short_oid_str;
