//! Rocket stage and multi-stage assembly types for performance analysis.
//!
//! This module provides two main types:
//!
//! - [`Stage`]: A single propulsion unit with engines, propellant, and structure
//! - [`Rocket`]: A complete multi-stage vehicle with payload
//!
//! # Stage Performance Metrics
//!
//! - **Delta-v**: Velocity change capability (from rocket equation)
//! - **TWR**: Thrust-to-weight ratio (determines acceleration)
//! - **Burn time**: Duration until propellant exhaustion
//! - **Mass ratio**: Wet mass / dry mass (key efficiency metric)
//!
//! # Rocket Performance Metrics
//!
//! - **Total delta-v**: Sum of stage delta-vs (accounting for staging)
//! - **Payload fraction**: Payload mass / total mass (efficiency metric)
//! - **Liftoff TWR**: First stage thrust-to-weight at ignition
//!
//! # Example: Single Stage
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
//!
//! # Example: Two-Stage Rocket
//!
//! ```
//! use tsi::stage::{Stage, Rocket};
//! use tsi::engine::EngineDatabase;
//! use tsi::units::Mass;
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let merlin = db.get("merlin-1d").expect("engine not found");
//! let mvac = db.get("merlin-vacuum").expect("engine not found");
//!
//! let stage1 = Stage::with_structural_ratio(merlin.clone(), 9, Mass::kg(400_000.0), 0.06);
//! let stage2 = Stage::with_structural_ratio(mvac.clone(), 1, Mass::kg(100_000.0), 0.04);
//!
//! let rocket = Rocket::new(vec![stage1, stage2], Mass::kg(22_800.0));
//!
//! // Combined delta-v exceeds LEO requirements
//! assert!(rocket.total_delta_v().as_mps() > 9_000.0);
//! assert!(rocket.payload_fraction().as_f64() > 0.03);
//! ```

mod rocket;
#[allow(clippy::module_inception)]
mod stage;

pub use rocket::{Rocket, TwrError};
pub use stage::Stage;
