//! Output formatting for CLI results.
//!
//! This module provides formatters for different output types:
//!
//! - [`terminal`]: Pretty-printed output with box drawing
//!
//! # Example
//!
//! ```ignore
//! use tsi::output::terminal;
//!
//! terminal::print_solution(9400.0, 5000.0, &solution);
//! ```

pub mod terminal;
