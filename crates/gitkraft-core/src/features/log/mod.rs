//! Log browsing and commit search operations.
//!
//! This feature module has no types of its own — it uses
//! [`CommitInfo`](super::commits::CommitInfo) from the commits feature.

pub mod ops;

pub use ops::{get_log, search_commits};
