//! Git operations grouped by feature area.
//!
//! Each sub-module owns both its domain types (`types.rs`) and its operations
//! (`ops.rs`), re-exporting both at the module level for ergonomic access.

pub mod branches;
pub mod commits;
pub mod diff;
pub mod graph;
pub mod log;
pub mod remotes;
pub mod repo;
pub mod staging;
pub mod stash;
