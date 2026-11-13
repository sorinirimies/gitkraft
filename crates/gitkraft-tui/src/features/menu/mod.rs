//! The animated settings menu (`M`): a popup that slides open over any screen, with rows that
//! change a setting in place or jump to where it is edited.

pub mod events;
mod state;
pub mod view;

pub use state::{DURATION, Menu, Row};
