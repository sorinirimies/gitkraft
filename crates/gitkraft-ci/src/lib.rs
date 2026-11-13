//! GitKraft CI
//!
//! Runs, jobs, steps and logs of GitHub Actions, Gitea/Forgejo Actions and
//! GitLab CI behind one model, plus the UI-free logic around them.

pub mod annotations;
pub mod backend;
pub mod events;
mod github_like;
mod gitlab;
pub mod inputs;
pub mod log;
pub mod model;
pub mod stats;
pub mod timing;
pub mod watch;
pub mod woodpecker;

pub use model::{Artifact, Job, RerunScope, Run, RunDetail, RunQuery, Status, Step, Workflow};

pub use backend::CiBackend;
pub use github_like::ActionsBackend;
pub use gitlab::GitLabCi;
pub use inputs::{WorkflowInput, workflow_inputs};
pub use woodpecker::WoodpeckerCi;
