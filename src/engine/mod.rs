//! Rocket engine types, propellants, and the engine database.
//!
//! This module provides:
//!
//! - [`Engine`] - Representation of a rocket engine with performance parameters
//! - [`Propellant`] - Fuel/oxidizer combinations (kerosene, hydrogen, methane, etc.)
//! - [`EngineDatabase`] - Collection of real-world engine specifications
//!
//! # Built-in Engine Database
//!
//! The database includes 11 real engines from various launch vehicles:
//!
//! | Engine | Vehicle | Propellant | Isp (vac) |
//! |--------|---------|------------|-----------|
//! | Merlin-1D | Falcon 9 | LOX/RP-1 | 311s |
//! | Raptor-2 | Starship | LOX/CH4 | 350s |
//! | RS-25 | SLS/Shuttle | LOX/LH2 | 452s |
//! | F-1 | Saturn V | LOX/RP-1 | 304s |
//!
//! # Example
//!
//! ```
//! use tsi::engine::EngineDatabase;
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let raptor = db.get("raptor-2").expect("engine not found");
//!
//! println!("Raptor-2 Isp: {}", raptor.isp_vac());
//! println!("Raptor-2 thrust: {}", raptor.thrust_vac());
//! ```

mod database;
#[allow(clippy::module_inception)]
mod engine;
mod propellant;

pub use database::EngineDatabase;
pub use engine::Engine;
pub use propellant::Propellant;
