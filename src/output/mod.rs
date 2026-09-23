//! Output formatting for CLI results.
//!
//! This module provides formatters for different output types:
//!
//! - [`terminal`]: Pretty-printed output with box drawing
//! - [`diagram`]: ASCII rocket diagram generation
//!
//! # Example
//!
//! ```
//! # use tsiolkovsky::engine::EngineDatabase;
//! # use tsiolkovsky::optimizer::{AnalyticalOptimizer, Constraints, Optimizer, Problem};
//! # use tsiolkovsky::units::{Mass, Velocity};
//! # let db = EngineDatabase::load_embedded().unwrap();
//! # let problem = Problem::new(
//! #     Mass::kg(5_000.0),
//! #     Velocity::mps(9_400.0),
//! #     vec![db.get("raptor-2").unwrap().clone()],
//! #     Constraints::default(),
//! # );
//! use tsiolkovsky::output::terminal;
//!
//! let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
//! terminal::print_solution(&solution);
//! ```

pub mod diagram;
pub mod terminal;
