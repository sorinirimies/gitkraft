//! The hub: pull requests, issues, CI runs, pulse and notifications of every
//! platform account, inside the desktop app.

pub mod charts;
pub mod state;
pub mod view;

pub use state::{HubMessage, HubState};
