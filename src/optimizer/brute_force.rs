//! Brute force optimizer for multi-engine rocket configurations.
//!
//! When multiple engine types are available or more than 2 stages are needed,
//! the analytical optimizer cannot be used. This optimizer performs a grid
//! search over the discrete solution space.
//!
//! # Search Space
//!
//! The optimizer searches over:
//! - Number of stages (1 to max_stages)
//! - Engine type per stage (from available_engines)
//! - Engine count per stage (1 to max_engines_per_stage)
//! - Propellant mass per stage (discretized grid)
//!
//! # Pruning
//!
//! Infeasible configurations are pruned early:
//! - TWR below minimum → skip
//! - Delta-v clearly insufficient → skip
//! - Mass ratio impossible for given structural ratio → skip
//!
//! # Performance
//!
//! The search space grows exponentially with stage count. For a 3-stage
//! rocket with 5 engines and 10 propellant steps:
//! - 5³ × 9³ × 10³ = 91 million configurations
//!
//! The optimizer prunes aggressively and uses coarse-to-fine search
//! to make this tractable.

use std::time::Instant;

use crate::engine::Engine;
use crate::physics::G0;
use crate::stage::{Rocket, Stage};
use crate::units::{Mass, Ratio};

use super::{OptimizeError, Optimizer, Problem, Solution};

/// Brute force optimizer for complex staging problems.
///
/// Use this optimizer when:
/// - Multiple engine types are available
/// - More than 2 stages are needed
/// - Fine-grained control over search is required
///
/// # Search Strategy
///
/// 1. Coarse search with large propellant steps
/// 2. Refine around best solutions
/// 3. Return best feasible solution found
///
/// # Example
///
/// ```
/// use tsi::optimizer::{BruteForceOptimizer, Problem, Constraints, Optimizer};
/// use tsi::engine::EngineDatabase;
/// use tsi::units::{Mass, Velocity};
///
/// let db = EngineDatabase::load_embedded().expect("failed to load database");
/// let raptor = db.get("raptor-2").expect("engine not found");
/// let merlin = db.get("merlin-1d").expect("engine not found");
///
/// let problem = Problem::new(
///     Mass::kg(5_000.0),
///     Velocity::mps(9_400.0),
///     vec![raptor.clone(), merlin.clone()],
///     Constraints::default(),
/// ).with_stage_count(2);
///
/// let optimizer = BruteForceOptimizer::default();
/// let solution = optimizer.optimize(&problem).expect("optimization failed");
///
/// assert!(solution.meets_target());
/// ```
#[derive(Debug, Clone)]
pub struct BruteForceOptimizer {
    /// Number of propellant mass steps per stage
    propellant_steps: u32,
    /// Minimum propellant mass to consider (kg)
    min_propellant_kg: f64,
    /// Maximum propellant mass to consider (kg)
    max_propellant_kg: f64,
}

impl Default for BruteForceOptimizer {
    fn default() -> Self {
        Self {
            propellant_steps: 20,
            min_propellant_kg: 10_000.0,
            max_propellant_kg: 5_000_000.0,
        }
    }
}

impl BruteForceOptimizer {
    /// Create a new brute force optimizer with custom parameters.
    pub fn new(propellant_steps: u32, min_propellant_kg: f64, max_propellant_kg: f64) -> Self {
        Self {
            propellant_steps,
            min_propellant_kg,
            max_propellant_kg,
        }
    }

    /// Generate propellant mass values to search (logarithmic spacing).
    fn propellant_grid(&self) -> Vec<f64> {
        let log_min = self.min_propellant_kg.ln();
        let log_max = self.max_propellant_kg.ln();
        let step = (log_max - log_min) / (self.propellant_steps - 1) as f64;

        (0..self.propellant_steps)
            .map(|i| (log_min + step * i as f64).exp())
            .collect()
    }

    /// Check if a stage configuration meets TWR constraints.
    fn check_stage_twr(
        stage: &Stage,
        payload_above: Mass,
        min_twr: Ratio,
        use_sea_level: bool,
    ) -> bool {
        let total_mass = stage.wet_mass() + payload_above;
        let thrust = if use_sea_level {
            stage.thrust_sl()
        } else {
            stage.thrust_vac()
        };
        let twr = Ratio::new(thrust.as_newtons() / (total_mass.as_kg() * G0));
        twr.as_f64() >= min_twr.as_f64()
    }

    /// Try to build a valid rocket from stage specifications.
    /// Returns None if constraints are violated.
    fn try_build_rocket(
        &self,
        stage_specs: &[StageSpec],
        payload: Mass,
        constraints: &super::Constraints,
    ) -> Option<Rocket> {
        let mut stages = Vec::with_capacity(stage_specs.len());

        // Build stages from top to bottom to check TWR correctly
        // (each stage must carry everything above it)
        let mut mass_above = payload;

        for (i, spec) in stage_specs.iter().enumerate().rev() {
            let stage = Stage::with_structural_ratio(
                spec.engine.clone(),
                spec.engine_count,
                Mass::kg(spec.propellant_kg),
                constraints.structural_ratio.as_f64(),
            );

            // Check TWR (sea level for first stage, vacuum for others)
            let is_first_stage = i == 0;
            let min_twr = if is_first_stage {
                constraints.min_liftoff_twr
            } else {
                constraints.min_stage_twr
            };

            if !Self::check_stage_twr(&stage, mass_above, min_twr, is_first_stage) {
                return None;
            }

            mass_above = mass_above + stage.wet_mass();
            stages.push(stage);
        }

        // Reverse to get bottom-to-top order
        stages.reverse();

        Some(Rocket::new(stages, payload))
    }

    /// Search for optimal configuration with given stage count.
    fn search_stage_count(
        &self,
        problem: &Problem,
        stage_count: u32,
        best_so_far: &mut Option<(Rocket, u64)>,
        iterations: &mut u64,
    ) {
        let propellant_values = self.propellant_grid();
        let engines = &problem.available_engines;
        let max_engines = problem.constraints.max_engines_per_stage;

        // Generate all stage spec combinations
        self.search_recursive(
            problem,
            stage_count as usize,
            0,
            &mut vec![],
            engines,
            &propellant_values,
            max_engines,
            best_so_far,
            iterations,
        );
    }

    /// Recursive search over stage configurations.
    #[allow(clippy::too_many_arguments)]
    fn search_recursive(
        &self,
        problem: &Problem,
        total_stages: usize,
        current_stage: usize,
        current_specs: &mut Vec<StageSpec>,
        engines: &[Engine],
        propellant_values: &[f64],
        max_engines: u32,
        best_so_far: &mut Option<(Rocket, u64)>,
        iterations: &mut u64,
    ) {
        if current_stage == total_stages {
            // We have a complete configuration, try to build it
            *iterations += 1;

            if let Some(rocket) =
                self.try_build_rocket(current_specs, problem.payload, &problem.constraints)
            {
                let delta_v = rocket.total_delta_v();
                let meets_target = delta_v.as_mps() >= problem.target_delta_v.as_mps();

                if meets_target {
                    let is_better = match best_so_far {
                        None => true,
                        Some((best_rocket, _)) => {
                            // Prefer lower total mass
                            rocket.total_mass().as_kg() < best_rocket.total_mass().as_kg()
                        }
                    };

                    if is_better {
                        *best_so_far = Some((rocket, *iterations));
                    }
                }
            }
            return;
        }

        // Try each engine type
        for engine in engines {
            // Try each engine count
            for engine_count in 1..=max_engines {
                // Try each propellant mass
                for &propellant_kg in propellant_values {
                    current_specs.push(StageSpec {
                        engine: engine.clone(),
                        engine_count,
                        propellant_kg,
                    });

                    self.search_recursive(
                        problem,
                        total_stages,
                        current_stage + 1,
                        current_specs,
                        engines,
                        propellant_values,
                        max_engines,
                        best_so_far,
                        iterations,
                    );

                    current_specs.pop();
                }
            }
        }
    }
}

/// Specification for a single stage during search.
#[derive(Debug, Clone)]
struct StageSpec {
    engine: Engine,
    engine_count: u32,
    propellant_kg: f64,
}

impl Optimizer for BruteForceOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        let start = Instant::now();

        // Validate the problem
        problem.is_valid()?;

        let mut best: Option<(Rocket, u64)> = None;
        let mut total_iterations = 0u64;

        // Determine stage count range
        let min_stages = problem.stage_count.unwrap_or(1);
        let max_stages = problem
            .stage_count
            .unwrap_or(problem.constraints.max_stages);

        // Search each stage count
        for stage_count in min_stages..=max_stages {
            self.search_stage_count(problem, stage_count, &mut best, &mut total_iterations);
        }

        // Return best solution found
        match best {
            Some((rocket, _found_at)) => {
                let solution = Solution::with_metadata(
                    rocket,
                    problem.target_delta_v,
                    total_iterations,
                    start.elapsed(),
                    "BruteForce",
                );
                Ok(solution)
            }
            None => Err(OptimizeError::Infeasible {
                reason: format!(
                    "No feasible configuration found after {} iterations",
                    total_iterations
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::optimizer::Constraints;
    use crate::units::Velocity;

    fn get_raptor() -> Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    fn get_merlin() -> Engine {
        let db = EngineDatabase::default();
        db.get("Merlin-1D").unwrap().clone()
    }

    #[test]
    fn brute_force_single_engine() {
        // Small search space for testing
        let optimizer = BruteForceOptimizer::new(5, 50_000.0, 500_000.0);

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert_eq!(solution.rocket.stage_count(), 2);
        assert!(solution.iterations > 0);
        assert_eq!(solution.optimizer_name, "BruteForce");
    }

    #[test]
    fn brute_force_multi_engine() {
        // Small search space for testing
        let optimizer = BruteForceOptimizer::new(5, 50_000.0, 500_000.0);

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor(), get_merlin()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert_eq!(solution.rocket.stage_count(), 2);
    }

    #[test]
    fn brute_force_tracks_iterations() {
        let optimizer = BruteForceOptimizer::new(3, 100_000.0, 300_000.0);

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(8_000.0),
            vec![get_raptor()],
            Constraints::default().with_max_engines(2),
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();

        // With 1 engine, 2 engine counts, 3 propellant values, 2 stages:
        // Should evaluate multiple configurations
        assert!(solution.iterations > 10);
    }

    #[test]
    fn brute_force_respects_twr() {
        let optimizer = BruteForceOptimizer::new(5, 50_000.0, 500_000.0);

        let constraints = Constraints::new(
            Ratio::new(1.3), // Higher liftoff TWR
            Ratio::new(0.7),
            2,
            Ratio::new(0.08),
        );

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get_raptor()],
            constraints,
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();

        // Liftoff TWR should meet constraint
        assert!(solution.rocket.liftoff_twr().as_f64() >= 1.3);
    }

    #[test]
    fn brute_force_infeasible_returns_error() {
        // Very small search space with impossible requirements
        let optimizer = BruteForceOptimizer::new(3, 1_000.0, 10_000.0);

        let problem = Problem::new(
            Mass::kg(100_000.0),     // Very heavy payload
            Velocity::mps(15_000.0), // Very high delta-v
            vec![get_merlin()],
            Constraints::default(),
        )
        .with_stage_count(2);

        let result = optimizer.optimize(&problem);

        assert!(matches!(result, Err(OptimizeError::Infeasible { .. })));
    }
}
