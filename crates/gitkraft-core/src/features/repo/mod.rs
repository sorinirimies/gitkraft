//! Repository feature — types and operations for opening, initialising,
//! cloning, and inspecting Git repositories.

pub mod ops;
pub mod types;
pub mod watcher;

pub use ops::{
    checkout_commit_detached, delete_file, get_repo_info, init_repo, load_repo_snapshot,
    load_repo_snapshot_with_depth, open_repo, reset_to_commit, revert_commit,
};
pub use types::{RepoInfo, RepoSnapshot, RepoState, ResetMode};
pub use watcher::spawn_git_watcher;
