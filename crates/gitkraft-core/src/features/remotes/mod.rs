//! Remote operations and types.

pub mod ops;
pub mod types;

pub use ops::{fetch_remote, list_remotes, pull, push};
pub use types::RemoteInfo;
