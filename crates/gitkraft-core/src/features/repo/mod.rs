//! Repository feature — types and operations for opening, initialising,
//! cloning, and inspecting Git repositories.

pub mod ops;
pub mod types;

pub use ops::{clone_repo, get_repo_info, init_repo, open_repo};
pub use types::{RepoInfo, RepoState};
