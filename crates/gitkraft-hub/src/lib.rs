//! GitKraft Hub
//!
//! What every front-end shares above the platform adapters: the account list,
//! dashboard sections turned into data, PR details, the pulse and the merged
//! contribution calendar.

pub mod accounts;
mod hub;
pub mod poller;
pub mod pulse;

pub use hub::{Hub, PrDetail, SectionData, SectionResult, is_open, shared};
