use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
