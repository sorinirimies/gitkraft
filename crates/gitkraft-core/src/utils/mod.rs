//! Utility helpers shared across the crate — OID formatting, relative time, text, etc.

pub mod text;
pub mod time;

pub use text::truncate_str;
pub use time::{fmt_oid, relative_time, short_oid, short_oid_str};
