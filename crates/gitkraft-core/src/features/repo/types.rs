use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Typed reset mode — eliminates stringly-typed `"soft"` / `"mixed"` / `"hard"`
/// arguments and prevents invalid values from being constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Move HEAD; staged and working-directory changes are preserved.
    Soft,
    /// Move HEAD and unstage changes; working directory is preserved.
    Mixed,
    /// Move HEAD and permanently discard all uncommitted changes.
    Hard,
}

impl ResetMode {
    /// The corresponding `git reset` flag (e.g. `--soft`).
    pub fn as_flag(self) -> &'static str {
        match self {
            Self::Soft  => "--soft",
            Self::Mixed => "--mixed",
            Self::Hard  => "--hard",
        }
    }
}

impl std::fmt::Display for ResetMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Soft  => write!(f, "soft"),
            Self::Mixed => write!(f, "mixed"),
            Self::Hard  => write!(f, "hard"),
        }
    }
}

/// High-level state the repository can be in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoState {
    Clean,
    Merging,
    Rebasing,
    Reverting,
    CherryPicking,
    Bisecting,
    ApplyMailbox,
    RebaseInteractive,
}

impl From<git2::RepositoryState> for RepoState {
    fn from(state: git2::RepositoryState) -> Self {
        match state {
            git2::RepositoryState::Clean => Self::Clean,
            git2::RepositoryState::Merge => Self::Merging,
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => {
                Self::Reverting
            }
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                Self::CherryPicking
            }
            git2::RepositoryState::Bisect => Self::Bisecting,
            git2::RepositoryState::Rebase | git2::RepositoryState::RebaseMerge => Self::Rebasing,
            git2::RepositoryState::RebaseInteractive => Self::RebaseInteractive,
            git2::RepositoryState::ApplyMailbox | git2::RepositoryState::ApplyMailboxOrRebase => {
                Self::ApplyMailbox
            }
        }
    }
}

impl std::fmt::Display for RepoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "Clean"),
            Self::Merging => write!(f, "Merging"),
            Self::Rebasing => write!(f, "Rebasing"),
            Self::Reverting => write!(f, "Reverting"),
            Self::CherryPicking => write!(f, "Cherry-picking"),
            Self::Bisecting => write!(f, "Bisecting"),
            Self::ApplyMailbox => write!(f, "Applying Mailbox"),
            Self::RebaseInteractive => write!(f, "Interactive Rebase"),
        }
    }
}

/// Summary snapshot of a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// Absolute path to the `.git` directory (or the bare repo root).
    pub path: PathBuf,
    /// Absolute path to the work-tree, if any.
    pub workdir: Option<PathBuf>,
    /// Name of the branch HEAD points to, if any.
    pub head_branch: Option<String>,
    /// Whether the repository is bare.
    pub is_bare: bool,
    /// Current repository state.
    pub state: RepoState,
}

use crate::features::{
    branches::BranchInfo, commits::CommitInfo, diff::DiffFileEntry, graph::GraphRow,
    remotes::RemoteInfo, stash::StashEntry,
};

/// Full snapshot of a repository loaded in one background operation.
///
/// Returned by [`load_repo_snapshot`] and used by both GUI and TUI to
/// apply a fresh repo state without multiple round-trips.
#[derive(Debug, Clone)]
pub struct RepoSnapshot {
    pub info: RepoInfo,
    pub branches: Vec<BranchInfo>,
    pub commits: Vec<CommitInfo>,
    pub graph_rows: Vec<GraphRow>,
    pub unstaged: Vec<DiffFileEntry>,
    pub staged: Vec<DiffFileEntry>,
    pub stashes: Vec<StashEntry>,
    pub remotes: Vec<RemoteInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ResetMode::as_flag ────────────────────────────────────────────────

    #[test]
    fn reset_mode_as_flag_soft() {
        assert_eq!(ResetMode::Soft.as_flag(), "--soft");
    }

    #[test]
    fn reset_mode_as_flag_mixed() {
        assert_eq!(ResetMode::Mixed.as_flag(), "--mixed");
    }

    #[test]
    fn reset_mode_as_flag_hard() {
        assert_eq!(ResetMode::Hard.as_flag(), "--hard");
    }

    // ── ResetMode::Display ────────────────────────────────────────────────

    #[test]
    fn reset_mode_display_soft() {
        assert_eq!(ResetMode::Soft.to_string(), "soft");
    }

    #[test]
    fn reset_mode_display_mixed() {
        assert_eq!(ResetMode::Mixed.to_string(), "mixed");
    }

    #[test]
    fn reset_mode_display_hard() {
        assert_eq!(ResetMode::Hard.to_string(), "hard");
    }

    // ── ResetMode semantics ───────────────────────────────────────────────

    /// Copy + PartialEq let callers compare and pass modes by value.
    #[test]
    fn reset_mode_is_copy_and_eq() {
        let m = ResetMode::Hard;
        let m2 = m; // Copy — must not consume `m`
        assert_eq!(m, m2);
    }

    #[test]
    fn reset_mode_variants_are_distinct() {
        assert_ne!(ResetMode::Soft, ResetMode::Mixed);
        assert_ne!(ResetMode::Mixed, ResetMode::Hard);
        assert_ne!(ResetMode::Soft, ResetMode::Hard);
    }

    /// Each variant's flag must start with `--` so git accepts it.
    #[test]
    fn all_flags_start_with_double_dash() {
        for mode in [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard] {
            assert!(
                mode.as_flag().starts_with("--"),
                "{mode} flag must start with '--'"
            );
        }
    }

    /// Display output must equal the flag without the leading `--`.
    #[test]
    fn display_matches_flag_without_dashes() {
        for mode in [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard] {
            let flag = mode.as_flag();
            let display = mode.to_string();
            assert_eq!(
                format!("--{display}"),
                flag,
                "Display for {mode:?} must match flag minus the '--'"
            );
        }
    }

    /// Debug output should be non-empty (sanity-check derive).
    #[test]
    fn reset_mode_debug_nonempty() {
        for mode in [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard] {
            assert!(!format!("{mode:?}").is_empty());
        }
    }
}
