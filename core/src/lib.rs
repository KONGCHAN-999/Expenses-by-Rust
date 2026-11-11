//! Pure, browser-independent logic for the expense tracker.
//!
//! Everything here is testable with plain `cargo test -p expenses-core`.

pub mod date;
pub mod format;
pub mod io;
pub mod keypad;
pub mod model;
pub mod summary;
