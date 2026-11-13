//! GitKraft Hub
//!
//! What every front-end shares above the platform adapters: the account list,
//! dashboard sections turned into data, PR details, the pulse and the merged
//! contribution calendar.

pub mod accounts;
pub mod artifacts;
pub mod checkout;
pub mod clone;
pub mod dispatch;
pub mod fuzzy;
mod hub;
pub mod keys;
pub mod labels;
pub mod markdown;
pub mod poller;
pub mod pulse;

pub use hub::{DayResult, Hub, PrDetail, SectionData, SectionResult, is_open, shared};
