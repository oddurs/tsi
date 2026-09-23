//! Output formatting for CLI results.
//!
//! - [`terminal`]: Pretty-printed output with box drawing
//! - [`diagram`]: ASCII rocket diagram generation
//!
//! These live in the `tsi` binary, not the library: how a solution looks in
//! a terminal is the CLI's business. Library users get the data and serialize
//! or print it however they like.

pub mod diagram;
pub mod terminal;
