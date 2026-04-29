//! Repository feature — types and operations for opening, initialising,
//! cloning, and inspecting Git repositories.

pub mod ops;
pub mod types;
pub mod watcher;

pub use ops::{
    checkout_commit_detached, cherry_pick_commit, clone_repo, delete_file, get_file_at_commit,
    get_repo_info, init_repo, load_repo_snapshot, open_repo, reset_to_commit, revert_commit,
};
pub use types::{RepoInfo, RepoSnapshot, RepoState};
pub use watcher::{spawn_git_watcher, spawn_git_watcher_with_fallback};
