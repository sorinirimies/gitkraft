//! Per-project notification settings.
//!
//! Layers, later wins: built-in defaults, `[defaults]` of the global file,
//! `[projects."host/owner/name"]` of the global file, then the repo's own
//! `.gitkraft/notifications.toml`. Every key is optional in every layer.

mod config;
mod filter;
mod glob;
mod status;

pub use config::{
    Effective, EventSet, LabelRule, NotifyConfig, NotifyEvent, NotifySettings, QuietHours, Scope,
    parse_duration, set_enabled,
};
pub use filter::Candidate;
pub use glob::glob_match;
pub use status::{NotifyStatus, StatusInput, status};
