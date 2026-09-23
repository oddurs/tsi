//! Optimization algorithms for rocket staging.
//!
//! This module provides the optimization framework for finding optimal
//! rocket configurations. Given a payload, target delta-v, and constraints,
//! the optimizer finds the best staging solution.
//!
//! # Architecture
//!
//! - [`Problem`]: Defines what to optimize (payload, delta-v, constraints)
//! - [`Solution`]: The optimal rocket configuration found
//! - [`Optimizer`]: Trait for optimization algorithms
//!
//! # Available Optimizers
//!
//! - [`AnalyticalOptimizer`]: Closed-form solution for 2-stage, single-engine
//! - [`BruteForceOptimizer`]: Grid search for multi-engine or N-stage problems
//!
//! # Example
//!
//! ```
//! use tsiolkovsky::prelude::*;
//!
//! let raptor = EngineDatabase::builtin().get("raptor-2").expect("engine not found");
//!
//! let problem = Problem::builder()
//!     .payload(Mass::tonnes(5.0))
//!     .target(Velocity::mps(9_400.0))
//!     .engine(raptor.clone())
//!     .stages(2)
//!     .build()?;
//!
//! let solution = AnalyticalOptimizer.optimize(&problem)?;
//!
//! println!("Total mass: {}", solution.rocket().total_mass());
//! println!("Payload fraction: {:.2}%", solution.payload_fraction_percent());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod analytical;
mod brute_force;
mod monte_carlo;
mod problem;
mod sizing;
mod solution;
mod uncertainty;

pub use analytical::AnalyticalOptimizer;
pub use brute_force::BruteForceOptimizer;
pub use monte_carlo::{MonteCarloResults, MonteCarloRunner};
pub use problem::{ConstraintError, Constraints, Problem, ProblemBuilder, ProblemError};
pub use solution::{OptimizerKind, Solution, JSON_SCHEMA_VERSION};
pub use uncertainty::{Uncertainty, UncertaintyError};

use crate::units::{Mass, Ratio, Velocity};

/// Trait for optimization algorithms.
///
/// Implementors find the lightest rocket that solves a problem. Different
/// algorithms have different trade-offs:
///
/// - **Analytical**: fast and exact for tsi's mass model
/// - **Brute force**: exhaustive grid search, an independent cross-check
/// - **Your own**: implement this trait, and report with [`Solution::new`]
pub trait Optimizer {
    /// Find the optimal rocket configuration.
    ///
    /// # Errors
    ///
    /// [`OptimizeError::Infeasible`] if no rocket meets the problem, with the
    /// [`Infeasibility`] that binds.
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError>;
}

/// Errors during optimization.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OptimizeError {
    /// No rocket meets the problem.
    #[error("No feasible solution: {0}")]
    Infeasible(Infeasibility),

    /// The search would be too large to run.
    #[error(
        "{stage_count} stages with these engines is {combinations} engine-to-stage \
        combinations, more than the {limit} tsi will search"
    )]
    TooManyCombinations {
        stage_count: u32,
        combinations: usize,
        limit: usize,
    },

    /// The Monte Carlo uncertainty isn't usable.
    #[error("Invalid uncertainty: {0}")]
    Uncertainty(#[from] UncertaintyError),

    /// Internal check: the sizing model and the rocket disagree. A bug.
    #[error(
        "Internal consistency check failed: sized for {designed:.3} m/s but the rocket \
        delivers {achieved:.3} m/s"
    )]
    InconsistentSizing { designed: f64, achieved: f64 },
}

/// Why no rocket meets a problem: the constraint that binds.
///
/// These are causes, not advice. The `tsi` command line turns each into
/// suggestions about which flag to change.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Infeasibility {
    /// No staging reaches the target: even stages of pure mass ratio, as
    /// large as their tanks allow, fall short.
    #[error("the structural ratio is too high to reach {target} with up to {max_stages} stages")]
    StructuralLimit { target: Velocity, max_stages: u32 },

    /// A stage needs more engines than allowed to reach its minimum TWR.
    #[error(
        "reaching TWR {required_twr:.2} on stage {} needs more than {max_engines} engines",
        stage + 1
    )]
    EngineLimit {
        /// Stage index, 0 = first stage
        stage: usize,
        required_twr: Ratio,
        max_engines: u32,
    },

    /// A stage's engines weigh too much for their thrust: no number of
    /// them reaches the minimum TWR.
    #[error(
        "no number of engines gives stage {} a TWR of {required_twr:.2}: each weighs \
        too much for its thrust at {gravity:.2} m/s²",
        stage + 1
    )]
    EnginesTooHeavy {
        /// Stage index, 0 = first stage
        stage: usize,
        required_twr: Ratio,
        gravity: f64,
    },

    /// The first stage can't deliver the delta-v it needs to carry an upper
    /// stage above the atmosphere.
    #[error("the first stage can't deliver the {floor} it needs below an upper stage")]
    BoosterTooSmall { floor: Velocity },

    /// The brute force grid found nothing, though a design exists.
    #[error(
        "the brute force grid found nothing after {evaluations} evaluations, but a \
        {known_mass} design exists"
    )]
    SearchMissed { evaluations: u64, known_mass: Mass },
}

/// Receives progress from long-running optimizers and Monte Carlo runs.
///
/// The library never prints. Give an optimizer a `Progress` to hear from it;
/// every method has a do-nothing default, so implement only what you need.
///
/// ```
/// use std::sync::atomic::{AtomicU64, Ordering};
/// use tsiolkovsky::optimizer::Progress;
///
/// #[derive(Default)]
/// struct Count(AtomicU64);
///
/// impl Progress for Count {
///     fn advance(&self, _done: u64) {
///         self.0.fetch_add(1, Ordering::Relaxed);
///     }
/// }
/// ```
pub trait Progress: Send + Sync {
    /// A phase of work begins, `total` steps long.
    fn start(&self, phase: &str, total: u64) {
        let _ = (phase, total);
    }

    /// `done` steps of the current phase are complete.
    fn advance(&self, done: u64) {
        let _ = done;
    }

    /// The current phase is complete.
    fn finish(&self) {}
}
