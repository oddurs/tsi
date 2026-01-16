//! Monte Carlo simulation for uncertainty analysis.
//!
//! Monte Carlo simulation runs the optimizer many times with randomly
//! perturbed parameters to understand how uncertainties propagate to
//! the final solution.
//!
//! # How It Works
//!
//! 1. Start with nominal engine parameters and structural ratio
//! 2. For each iteration:
//!    - Sample perturbed parameters from their distributions
//!    - Re-optimize with perturbed parameters
//!    - Record the resulting delta-v and mass
//! 3. Analyze the distribution of results
//!
//! # Interpreting Results
//!
//! The Monte Carlo results show:
//! - **Success probability**: % of runs achieving target delta-v
//! - **Confidence intervals**: Range of expected performance
//! - **Margin requirements**: How much extra delta-v to budget
//!
//! # Example
//!
//! ```no_run
//! use tsi::optimizer::{Problem, Constraints, Uncertainty, MonteCarloRunner};
//! use tsi::engine::EngineDatabase;
//! use tsi::units::{Mass, Velocity};
//!
//! let db = EngineDatabase::load_embedded().expect("load db");
//! let engine = db.get("raptor-2").expect("engine");
//!
//! let problem = Problem::new(
//!     Mass::kg(5_000.0),
//!     Velocity::mps(9_400.0),
//!     vec![engine.clone()],
//!     Constraints::default(),
//! ).with_stage_count(2);
//!
//! let runner = MonteCarloRunner::new(Uncertainty::default());
//! let results = runner.run(&problem, 1000).expect("monte carlo");
//!
//! println!("Success probability: {:.1}%", results.success_probability() * 100.0);
//! println!("Delta-v 5th percentile: {:.0} m/s", results.delta_v_percentile(5.0));
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use serde::Serialize;

use super::{
    AnalyticalOptimizer, BruteForceOptimizer, Constraints, OptimizeError, Optimizer, Problem,
    ParameterSampler, Solution, Uncertainty,
};
use crate::engine::Engine;
use crate::units::Velocity;

/// Results from a Monte Carlo simulation.
///
/// Contains the distribution of outcomes from running the optimizer
/// many times with perturbed parameters.
#[derive(Debug, Clone)]
pub struct MonteCarloResults {
    /// Delta-v achieved in each successful run (m/s)
    pub delta_v_samples: Vec<f64>,

    /// Total mass in each successful run (kg)
    pub mass_samples: Vec<f64>,

    /// Number of runs that achieved target delta-v
    pub successes: u64,

    /// Total number of runs attempted
    pub total_runs: u64,

    /// Number of runs that failed to find a feasible solution
    pub failures: u64,

    /// Target delta-v used for success calculation
    pub target_delta_v: Velocity,

    /// Time taken to run the simulation
    pub runtime: Duration,

    /// The nominal (unperturbed) solution for reference
    pub nominal_solution: Solution,
}

impl MonteCarloResults {
    /// Probability that the design achieves the target delta-v.
    ///
    /// This is the key metric for mission planning. A value of 0.95
    /// means 95% confidence in achieving the target.
    pub fn success_probability(&self) -> f64 {
        if self.total_runs == 0 {
            return 0.0;
        }
        self.successes as f64 / self.total_runs as f64
    }

    /// Get a percentile of the delta-v distribution.
    ///
    /// # Arguments
    ///
    /// * `percentile` - Value from 0 to 100
    ///
    /// # Returns
    ///
    /// The delta-v value at that percentile (m/s), or 0 if no samples.
    ///
    /// # Example
    ///
    /// - 5th percentile: "worst case" performance
    /// - 50th percentile: median performance
    /// - 95th percentile: "best case" performance
    pub fn delta_v_percentile(&self, percentile: f64) -> f64 {
        percentile_of(&self.delta_v_samples, percentile)
    }

    /// Get a percentile of the mass distribution.
    ///
    /// # Arguments
    ///
    /// * `percentile` - Value from 0 to 100
    ///
    /// # Returns
    ///
    /// The total mass value at that percentile (kg), or 0 if no samples.
    pub fn mass_percentile(&self, percentile: f64) -> f64 {
        percentile_of(&self.mass_samples, percentile)
    }

    /// Mean delta-v across all successful runs.
    pub fn mean_delta_v(&self) -> f64 {
        if self.delta_v_samples.is_empty() {
            return 0.0;
        }
        self.delta_v_samples.iter().sum::<f64>() / self.delta_v_samples.len() as f64
    }

    /// Standard deviation of delta-v across all successful runs.
    pub fn std_delta_v(&self) -> f64 {
        if self.delta_v_samples.len() < 2 {
            return 0.0;
        }
        let mean = self.mean_delta_v();
        let variance = self.delta_v_samples.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / (self.delta_v_samples.len() - 1) as f64;
        variance.sqrt()
    }

    /// Mean total mass across all successful runs.
    pub fn mean_mass(&self) -> f64 {
        if self.mass_samples.is_empty() {
            return 0.0;
        }
        self.mass_samples.iter().sum::<f64>() / self.mass_samples.len() as f64
    }

    /// Margin needed to achieve target delta-v at given confidence level.
    ///
    /// Returns the additional delta-v (above target) needed to ensure
    /// the specified probability of success.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Desired success probability (0.0 to 1.0)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // How much margin for 95% confidence?
    /// let margin = results.required_margin(0.95);
    /// println!("Need {} m/s margin for 95% confidence", margin);
    /// ```
    pub fn required_margin(&self, confidence: f64) -> f64 {
        if self.delta_v_samples.is_empty() {
            return 0.0;
        }
        // Find the percentile where we have (1 - confidence) failures
        let failure_percentile = (1.0 - confidence) * 100.0;
        let dv_at_percentile = self.delta_v_percentile(failure_percentile);
        let target = self.target_delta_v.as_mps();

        // Margin is how much below target the worst cases are
        (target - dv_at_percentile).max(0.0)
    }

    /// Convert to JSON-serializable summary.
    pub fn to_json_summary(&self) -> MonteCarloJsonSummary {
        MonteCarloJsonSummary {
            success_probability: self.success_probability(),
            total_runs: self.total_runs,
            successes: self.successes,
            failures: self.failures,
            target_delta_v_mps: self.target_delta_v.as_mps(),
            runtime_ms: self.runtime.as_millis() as u64,
            delta_v: DistributionSummary {
                mean: self.mean_delta_v(),
                std_dev: self.std_delta_v(),
                percentile_5: self.delta_v_percentile(5.0),
                percentile_50: self.delta_v_percentile(50.0),
                percentile_95: self.delta_v_percentile(95.0),
                min: self.delta_v_samples.iter().cloned().fold(f64::INFINITY, f64::min),
                max: self.delta_v_samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            },
            mass: DistributionSummary {
                mean: self.mean_mass(),
                std_dev: 0.0, // Could add std_mass() if needed
                percentile_5: self.mass_percentile(5.0),
                percentile_50: self.mass_percentile(50.0),
                percentile_95: self.mass_percentile(95.0),
                min: self.mass_samples.iter().cloned().fold(f64::INFINITY, f64::min),
                max: self.mass_samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            },
            required_margin_95_mps: self.required_margin(0.95),
        }
    }
}

/// JSON-serializable Monte Carlo summary.
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloJsonSummary {
    /// Probability of achieving target delta-v (0.0 to 1.0)
    pub success_probability: f64,

    /// Total number of Monte Carlo iterations
    pub total_runs: u64,

    /// Number of successful runs
    pub successes: u64,

    /// Number of failed optimization attempts
    pub failures: u64,

    /// Target delta-v in m/s
    pub target_delta_v_mps: f64,

    /// Simulation runtime in milliseconds
    pub runtime_ms: u64,

    /// Delta-v distribution statistics
    pub delta_v: DistributionSummary,

    /// Total mass distribution statistics
    pub mass: DistributionSummary,

    /// Additional margin needed for 95% confidence (m/s)
    pub required_margin_95_mps: f64,
}

/// Summary statistics for a distribution.
#[derive(Debug, Clone, Serialize)]
pub struct DistributionSummary {
    /// Mean value
    pub mean: f64,

    /// Standard deviation
    pub std_dev: f64,

    /// 5th percentile (worst case)
    pub percentile_5: f64,

    /// 50th percentile (median)
    pub percentile_50: f64,

    /// 95th percentile (best case)
    pub percentile_95: f64,

    /// Minimum value
    pub min: f64,

    /// Maximum value
    pub max: f64,
}

/// Calculate percentile of a sample set.
fn percentile_of(samples: &[f64], percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p = percentile.clamp(0.0, 100.0) / 100.0;
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Monte Carlo simulation runner.
///
/// Runs multiple optimization iterations with perturbed parameters
/// to assess the robustness of a rocket design.
#[derive(Debug, Clone)]
pub struct MonteCarloRunner {
    uncertainty: Uncertainty,
    show_progress: bool,
}

impl MonteCarloRunner {
    /// Create a new Monte Carlo runner with the given uncertainty.
    pub fn new(uncertainty: Uncertainty) -> Self {
        Self {
            uncertainty,
            show_progress: false,
        }
    }

    /// Enable progress reporting to stderr.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Run Monte Carlo simulation.
    ///
    /// # Arguments
    ///
    /// * `problem` - The nominal optimization problem
    /// * `iterations` - Number of Monte Carlo iterations
    ///
    /// # Returns
    ///
    /// Results containing distributions and statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if the nominal problem is invalid.
    pub fn run(&self, problem: &Problem, iterations: u64) -> Result<MonteCarloResults, OptimizeError> {
        // Validate the nominal problem first
        problem.is_valid()?;

        let start = Instant::now();

        // Run nominal optimization to get baseline
        let nominal_solution = self.optimize_problem(problem)?;

        // If zero uncertainty, just return nominal result
        if self.uncertainty.is_zero() {
            return Ok(MonteCarloResults {
                delta_v_samples: vec![nominal_solution.rocket.total_delta_v().as_mps()],
                mass_samples: vec![nominal_solution.rocket.total_mass().as_kg()],
                successes: 1,
                total_runs: 1,
                failures: 0,
                target_delta_v: problem.target_delta_v,
                runtime: start.elapsed(),
                nominal_solution,
            });
        }

        // Parallel Monte Carlo
        let sampler = ParameterSampler::new(self.uncertainty);
        let target_dv = problem.target_delta_v.as_mps();

        // Shared state for results
        let delta_v_samples = Arc::new(Mutex::new(Vec::with_capacity(iterations as usize)));
        let mass_samples = Arc::new(Mutex::new(Vec::with_capacity(iterations as usize)));
        let successes = AtomicU64::new(0);
        let failures = AtomicU64::new(0);
        let completed = AtomicU64::new(0);

        // Run parallel iterations
        (0..iterations).into_par_iter().for_each(|_| {
            // Perturb engines
            let perturbed_engines: Vec<Engine> = problem.available_engines
                .iter()
                .map(|e| sampler.perturb_engine(e))
                .collect();

            // Perturb structural ratio
            let perturbed_structural = sampler.perturb_structural_ratio(
                problem.constraints.structural_ratio
            );

            // Create perturbed problem
            let perturbed_constraints = Constraints {
                structural_ratio: perturbed_structural,
                ..problem.constraints.clone()
            };

            let perturbed_problem = Problem {
                payload: problem.payload,
                target_delta_v: problem.target_delta_v,
                available_engines: perturbed_engines,
                constraints: perturbed_constraints,
                stage_count: problem.stage_count,
            };

            // Run optimization
            match self.optimize_problem(&perturbed_problem) {
                Ok(solution) => {
                    let dv = solution.rocket.total_delta_v().as_mps();
                    let mass = solution.rocket.total_mass().as_kg();

                    // Record results
                    delta_v_samples.lock().unwrap().push(dv);
                    mass_samples.lock().unwrap().push(mass);

                    // Count success if meets target
                    if dv >= target_dv {
                        successes.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => {
                    failures.fetch_add(1, Ordering::Relaxed);
                }
            }

            // Progress reporting
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            if self.show_progress && done.is_multiple_of(100) {
                let pct = (done as f64 / iterations as f64) * 100.0;
                eprint!("\rMonte Carlo: {:.0}% ({}/{})", pct, done, iterations);
            }
        });

        if self.show_progress {
            eprintln!("\rMonte Carlo: 100% ({}/{})", iterations, iterations);
        }

        Ok(MonteCarloResults {
            delta_v_samples: Arc::try_unwrap(delta_v_samples).unwrap().into_inner().unwrap(),
            mass_samples: Arc::try_unwrap(mass_samples).unwrap().into_inner().unwrap(),
            successes: successes.load(Ordering::Relaxed),
            total_runs: iterations,
            failures: failures.load(Ordering::Relaxed),
            target_delta_v: problem.target_delta_v,
            runtime: start.elapsed(),
            nominal_solution,
        })
    }

    /// Run optimization on a problem, selecting appropriate optimizer.
    fn optimize_problem(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        // Use analytical for simple 2-stage single-engine, brute force otherwise
        if problem.is_single_engine() && problem.stage_count == Some(2) {
            let optimizer = AnalyticalOptimizer;
            optimizer.optimize(problem)
        } else {
            let optimizer = BruteForceOptimizer::default();
            optimizer.optimize(problem)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::units::Mass;

    fn get_raptor() -> Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    fn simple_problem() -> Problem {
        Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_400.0),
            vec![get_raptor()],
            Constraints::default(),
        ).with_stage_count(2)
    }

    #[test]
    fn monte_carlo_zero_uncertainty() {
        let runner = MonteCarloRunner::new(Uncertainty::none());
        let problem = simple_problem();

        let results = runner.run(&problem, 10).expect("monte carlo should succeed");

        // With zero uncertainty, should have 100% success
        assert_eq!(results.successes, 1);
        assert_eq!(results.total_runs, 1);
        assert_eq!(results.success_probability(), 1.0);
    }

    #[test]
    fn monte_carlo_with_uncertainty() {
        let runner = MonteCarloRunner::new(Uncertainty::default());
        let problem = simple_problem();

        let results = runner.run(&problem, 100).expect("monte carlo should succeed");

        // Should have completed 100 runs
        assert_eq!(results.total_runs, 100);

        // Most runs should succeed (design has margin)
        assert!(results.success_probability() > 0.5,
            "Expected >50% success, got {:.1}%", results.success_probability() * 100.0);

        // Delta-v samples should be reasonable
        assert!(!results.delta_v_samples.is_empty());
        let mean_dv = results.mean_delta_v();
        assert!(mean_dv > 9000.0 && mean_dv < 11000.0,
            "Mean delta-v {} outside expected range", mean_dv);
    }

    #[test]
    fn percentile_calculation() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        // 0th percentile = minimum
        assert!((percentile_of(&samples, 0.0) - 1.0).abs() < 0.1);

        // 50th percentile = median
        assert!((percentile_of(&samples, 50.0) - 5.5).abs() < 1.0);

        // 100th percentile = maximum
        assert!((percentile_of(&samples, 100.0) - 10.0).abs() < 0.1);
    }

    #[test]
    fn percentile_empty() {
        let empty: Vec<f64> = vec![];
        assert_eq!(percentile_of(&empty, 50.0), 0.0);
    }

    #[test]
    fn results_statistics() {
        use crate::stage::Stage;

        // Create a minimal valid rocket for the test
        let engine = get_raptor();
        let stage = Stage::new(engine, 1, Mass::kg(100_000.0), Mass::kg(8_000.0));
        let rocket = crate::stage::Rocket::new(vec![stage], Mass::kg(5000.0));

        let results = MonteCarloResults {
            delta_v_samples: vec![9400.0, 9500.0, 9600.0, 9700.0, 9800.0],
            mass_samples: vec![100000.0, 101000.0, 102000.0, 103000.0, 104000.0],
            successes: 4,
            total_runs: 5,
            failures: 0,
            target_delta_v: Velocity::mps(9500.0),
            runtime: Duration::from_secs(1),
            nominal_solution: Solution {
                rocket,
                margin: Velocity::mps(100.0),
                iterations: 1,
                runtime: Duration::from_secs(0),
                optimizer_name: "test".to_string(),
            },
        };

        assert!((results.success_probability() - 0.8).abs() < 0.01);
        assert!((results.mean_delta_v() - 9600.0).abs() < 1.0);
        assert!(results.std_delta_v() > 0.0);
    }
}
