//! # tsiolkovsky
//!
//! Rocket staging optimization: the rocket equation, real engines, and the
//! lightest rocket that does the job.
//!
//! > *"The Earth is the cradle of humanity, but one cannot remain in the
//! > cradle forever."* — Konstantin Tsiolkovsky, 1911
//!
//! In 1903 Konstantin Tsiolkovsky wrote down the equation that governs every
//! rocket: the velocity a rocket can gain, Δv, is its exhaust velocity times
//! the natural log of how much lighter it gets as it burns.
//!
//! ```text
//! Δv = Isp · g₀ · ln(m_wet / m_dry)
//! ```
//!
//! That logarithm is the whole story of spaceflight. Doubling the propellant
//! doesn't double the delta-v, so no single stage reaches orbit comfortably,
//! and rockets stage: they throw away empty tanks and engines on the way up.
//! This crate works out how.
//!
//! ## A first rocket
//!
//! ```
//! use tsiolkovsky::prelude::*;
//!
//! let raptor = EngineDatabase::builtin().get("raptor-2").expect("engine not found");
//!
//! let problem = Problem::builder()
//!     .payload(Mass::tonnes(5.0))
//!     .target(Velocity::mps(9_400.0)) // low Earth orbit, losses included
//!     .engine(raptor.clone())
//!     .build()?;
//!
//! let solution = AnalyticalOptimizer.optimize(&problem)?;
//! let rocket = solution.rocket();
//!
//! // With up to three stages allowed (the default), three wins here.
//! assert_eq!(rocket.stage_count(), 3);
//! println!("{} stages, {} at liftoff", rocket.stage_count(), rocket.total_mass());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## What's here
//!
//! | Module | What it does |
//! |--------|--------------|
//! | [`units`] | Mass, velocity, force, Isp, time and ratios that can't be mixed up |
//! | [`physics`] | The rocket equation, TWR, burn time, how Isp changes with altitude |
//! | [`engine`] | Real engines (Merlin, Raptor, RS-25, F-1…) and their database |
//! | [`stage`] | A stage, and a rocket built from stages |
//! | [`optimizer`] | Problems, the optimizers that solve them, and Monte Carlo analysis |
//!
//! [`prelude`] brings in everything a typical program needs.
//!
//! ## The `tsi` command
//!
//! This crate also builds `tsi`, a command-line tool over the same library,
//! behind the default `cli` feature. To use the library alone:
//!
//! ```toml
//! tsiolkovsky = { version = "0.8", default-features = false }
//! ```

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod engine;
pub mod optimizer;
pub mod physics;
pub mod stage;
pub mod units;

/// The types and traits most programs need, in one import.
///
/// ```
/// use tsiolkovsky::prelude::*;
/// ```
pub mod prelude {
    pub use crate::engine::{Engine, EngineDatabase, Propellant};
    pub use crate::optimizer::{
        AnalyticalOptimizer, BruteForceOptimizer, Constraints, MonteCarloRunner, Optimizer,
        Problem, Solution, Uncertainty,
    };
    pub use crate::physics::{delta_v, IspModel, G0};
    pub use crate::stage::{Rocket, Stage};
    pub use crate::units::{Force, Isp, Mass, Ratio, Time, Velocity};
}
