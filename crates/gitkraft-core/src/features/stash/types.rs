use serde::{Deserialize, Serialize};

/// A single stash entry in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    /// Zero-based index of this stash in the stash list (0 = most recent).
    pub index: usize,
    /// The stash message (e.g. "WIP on main: abc1234 some commit").
    pub message: String,
    /// The OID of the stash commit as a hex string.
    pub oid: String,
}


impl StashEntry {
    /// Message truncated to `max_chars` with "..." appended if shortened.
    pub fn short_message(&self, max_chars: usize) -> String {
        crate::utils::truncate_str(&self.message, max_chars)
    }
}
