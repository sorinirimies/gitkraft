//! GitKraft Hub
//!
//! What every front-end shares above the platform adapters: the account list,
//! dashboard sections turned into data, PR details, the pulse and the merged
//! contribution calendar.

pub mod accounts;
pub mod ai;
pub mod args;
pub mod artifacts;
pub mod checkout;
pub mod clone;
pub mod commit_ai;
pub mod community;
pub mod dispatch;
pub mod fuzzy;
mod hub;
pub mod keys;
pub mod labels;
pub mod markdown;
pub mod poller;
pub mod project;
pub mod pulse;
pub mod review_ai;

pub use hub::{DayResult, Hub, PrDetail, SectionData, SectionResult, is_open, shared};
