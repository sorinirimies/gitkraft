//! GitKraft Core
//!
//! Shared, framework-free logic reused by both the Iced GUI and Ratatui TUI.
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`features`] | Git operations and types grouped by feature — repo, branches, commits, diff, staging, remotes, stash, log (settings and theme live in `gitkraft-settings`, re-exported here) |
//! | [`utils`] | Helpers — relative time formatting, OID formatting, text truncation |
//!
//! This crate has NO GUI or TUI dependencies.

pub mod about;
pub mod features;
pub mod utils;

// Convenience re-exports
pub use about::{APP_NAME, version_label};
pub use features::branches::{BranchInfo, BranchType, validate_ref_name};
pub use features::commits::CommitInfo;
pub use features::commits::{COMMIT_MENU_GROUPS, CommitAction, CommitActionKind};
pub use features::commits::{COMMIT_SUBJECT_LIMIT, CommitMsgSeverity, check_commit_message};
pub use features::commits::{RefKind, RefLabel};
pub use features::diff::{
    BlameLine, DiffFileEntry, DiffHunk, DiffInfo, DiffLine, FileStatus, StatusColorCategory,
    blame_file,
};
pub use features::editor::{EDITOR_NAMES, Editor, open_file_default, show_in_folder};
pub use features::graph::{GraphEdge, GraphRow};
pub use features::log::file_history;
pub use features::persistence::{AppSettings, LayoutSettings, RepoHistoryEntry};
pub use features::remotes::RemoteInfo;
pub use features::repo::spawn_git_watcher;
pub use features::repo::{
    RepoInfo, RepoSnapshot, RepoState, ResetMode, delete_file, load_repo_snapshot,
    load_repo_snapshot_with_depth,
};
pub use features::stash::StashEntry;
pub use features::theme::{
    AppTheme, Rgb, SYSTEM_THEME_NAME, THEME_COUNT, THEME_NAMES, current_system_theme_index,
    detect_system_dark, is_system_name, system_theme_index, theme_by_index, theme_index_by_name,
};
pub use utils::short_oid_str;
pub use utils::text::{path_basename, truncate_str};
pub use utils::{
    ascending_range, clamp_next, clamp_selection, repo_display_name, wrap_next, wrap_prev,
};

/// Caps libgit2's memory-mapped pack window: a 64 MB window, 256 MB in all.
///
/// Without it, large repositories can map GBs of packfiles across repeated
/// `Repository::discover` calls. Call it once at startup, before any other
/// libgit2 use.
#[allow(
    unsafe_code,
    reason = "git2::opts setters are unsafe: they race with concurrent libgit2 use"
)]
pub fn limit_git_memory() {
    // SAFETY: called once from `main` before any thread uses libgit2.
    unsafe {
        git2::opts::set_mwindow_size(64 * 1024 * 1024).ok();
        git2::opts::set_mwindow_mapped_limit(256 * 1024 * 1024).ok();
    }
}
