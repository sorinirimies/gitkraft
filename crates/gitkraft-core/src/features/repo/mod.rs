//! Repository feature — types and operations for opening, initialising,
//! cloning, and inspecting Git repositories.

pub mod ops;
pub mod types;

pub use ops::{
    checkout_commit_detached, clone_repo, get_repo_info, init_repo, open_repo, revert_commit,
};
pub use types::{RepoInfo, RepoState};
