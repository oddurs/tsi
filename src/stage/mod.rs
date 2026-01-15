//! Rocket stage types for performance analysis.
//!
//! A stage is a self-contained propulsion unit with engines, propellant tanks,
//! and structure. This module provides the [`Stage`] type for calculating
//! stage-level performance metrics.
//!
//! # Stage Performance Metrics
//!
//! - **Delta-v**: Velocity change capability (from rocket equation)
//! - **TWR**: Thrust-to-weight ratio (determines acceleration)
//! - **Burn time**: Duration until propellant exhaustion
//! - **Mass ratio**: Wet mass / dry mass (key efficiency metric)
//!
//! # Example
//!
//! ```
//! use tsi::stage::Stage;
//! use tsi::engine::EngineDatabase;
//! use tsi::units::Mass;
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let merlin = db.get("merlin-1d").expect("engine not found");
//!
//! // Falcon 9 first stage approximation
//! let stage = Stage::with_structural_ratio(
//!     merlin.clone(),
//!     9,                      // 9 Merlin engines
//!     Mass::kg(400_000.0),    // ~400 tonnes propellant
//!     0.10,                   // 10% structural ratio
//! );
//!
//! assert!(stage.delta_v().as_mps() > 7_000.0);
//! assert!(stage.twr_vac().as_f64() > 1.5);
//! ```

#[allow(clippy::module_inception)]
mod stage;

pub use stage::Stage;
