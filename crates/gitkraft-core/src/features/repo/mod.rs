//! Repository feature — types and operations for opening, initialising,
//! cloning, and inspecting Git repositories.

pub mod ops;
pub mod types;

pub use ops::{
    checkout_commit_detached, cherry_pick_commit, clone_repo, get_file_at_commit, get_repo_info,
    init_repo, open_repo, reset_to_commit, revert_commit,
};
pub use types::{RepoInfo, RepoState};
