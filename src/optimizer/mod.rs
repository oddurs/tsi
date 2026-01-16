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
//! use tsi::optimizer::{Problem, Constraints, Optimizer, AnalyticalOptimizer};
//! use tsi::engine::EngineDatabase;
//! use tsi::units::{Mass, Velocity};
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let raptor = db.get("raptor-2").expect("engine not found");
//!
//! let problem = Problem::new(
//!     Mass::kg(5_000.0),
//!     Velocity::mps(9_400.0),
//!     vec![raptor.clone()],
//!     Constraints::default(),
//! ).with_stage_count(2);
//!
//! let optimizer = AnalyticalOptimizer;
//! let solution = optimizer.optimize(&problem).expect("optimization failed");
//!
//! println!("Total mass: {}", solution.rocket.total_mass());
//! println!("Payload fraction: {:.2}%", solution.payload_fraction_percent());
//! ```

mod analytical;
mod brute_force;
mod monte_carlo;
mod problem;
mod solution;
mod uncertainty;

pub use analytical::AnalyticalOptimizer;
pub use brute_force::BruteForceOptimizer;
pub use monte_carlo::{
    DistributionSummary, MonteCarloJsonSummary, MonteCarloResults, MonteCarloRunner,
};
pub use problem::{ConstraintError, Constraints, Problem, ProblemError};
pub use solution::Solution;
pub use uncertainty::{ParameterSampler, Uncertainty};

/// Trait for optimization algorithms.
///
/// Implementors find the optimal rocket configuration for a given problem.
/// Different algorithms have different trade-offs:
///
/// - **Analytical**: Fast, exact, but limited to simple cases
/// - **Brute Force**: Exhaustive, handles discrete choices, but slow
/// - **Genetic/Evolutionary**: Good for complex spaces, but approximate
pub trait Optimizer {
    /// Find the optimal rocket configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The problem is invalid
    /// - No feasible solution exists
    /// - The optimizer cannot handle this problem type
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError>;
}

/// Errors during optimization.
#[derive(Debug, thiserror::Error)]
pub enum OptimizeError {
    /// The problem specification is invalid
    #[error("Invalid problem: {0}")]
    InvalidProblem(#[from] ProblemError),

    /// No feasible solution exists within constraints
    #[error("No feasible solution: {reason}")]
    Infeasible { reason: String },

    /// This optimizer cannot handle the given problem
    #[error("Unsupported problem type: {reason}")]
    Unsupported { reason: String },
}
