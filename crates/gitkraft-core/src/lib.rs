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
pub use features::commits::{CommitAction, CommitActionKind, COMMIT_MENU_GROUPS};
pub use features::diff::{
    blame_file, BlameLine, DiffFileEntry, DiffHunk, DiffInfo, DiffLine, FileStatus,
    StatusColorCategory,
};
pub use features::editor::{open_file_default, show_in_folder, Editor, EDITOR_NAMES};
pub use features::graph::{GraphEdge, GraphRow};
pub use features::log::file_history;
pub use features::persistence::{AppSettings, LayoutSettings, RepoHistoryEntry};
pub use features::remotes::RemoteInfo;
pub use features::repo::spawn_git_watcher;
pub use features::repo::{delete_file, load_repo_snapshot, RepoInfo, RepoSnapshot, RepoState};
pub use features::stash::StashEntry;
pub use features::theme::{
    theme_by_index, theme_index_by_name, AppTheme, Rgb, THEME_COUNT, THEME_NAMES,
};
pub use utils::ascending_range;
pub use utils::short_oid_str;
pub use utils::text::truncate_str;
