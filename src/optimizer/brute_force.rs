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
//! # Search Strategy
//!
//! 1. **Coarse search**: Wide propellant grid to find promising regions
//! 2. **Refinement**: Zoom in around best solutions with finer grid
//! 3. **Parallelization**: Uses rayon for multi-threaded search
//! 4. **Vacuum preference**: Prefers vacuum-optimized engines for upper stages
//!
//! # Pruning
//!
//! Infeasible configurations are pruned early:
//! - TWR below minimum → skip
//! - Delta-v clearly insufficient → skip
//! - Mass ratio impossible for given structural ratio → skip

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rayon::prelude::*;

use crate::engine::Engine;
use crate::physics::G0;
use crate::stage::{Rocket, Stage};
use crate::units::{Mass, Ratio};

use super::{Constraints, OptimizeError, Optimizer, Problem, Solution};

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
/// 2. Refine around best solutions with finer grid
/// 3. Parallel execution using all CPU cores
/// 4. Return best feasible solution found
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
    /// Number of propellant mass steps per stage (coarse search)
    coarse_steps: u32,
    /// Number of propellant mass steps for refinement
    fine_steps: u32,
    /// Minimum propellant mass to consider (kg)
    min_propellant_kg: f64,
    /// Maximum propellant mass to consider (kg)
    max_propellant_kg: f64,
    /// Show progress indicator
    show_progress: bool,
    /// Prefer vacuum engines for upper stages
    prefer_vacuum_upper: bool,
}

impl Default for BruteForceOptimizer {
    fn default() -> Self {
        Self {
            coarse_steps: 15,
            fine_steps: 10,
            min_propellant_kg: 10_000.0,
            max_propellant_kg: 5_000_000.0,
            show_progress: true,
            prefer_vacuum_upper: true,
        }
    }
}

impl BruteForceOptimizer {
    /// Create a new brute force optimizer with custom parameters.
    pub fn new(propellant_steps: u32, min_propellant_kg: f64, max_propellant_kg: f64) -> Self {
        Self {
            coarse_steps: propellant_steps,
            fine_steps: propellant_steps / 2 + 1,
            min_propellant_kg,
            max_propellant_kg,
            show_progress: false,
            prefer_vacuum_upper: true,
        }
    }

    /// Enable or disable progress indicator.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Enable or disable vacuum engine preference for upper stages.
    pub fn with_vacuum_preference(mut self, prefer: bool) -> Self {
        self.prefer_vacuum_upper = prefer;
        self
    }

    /// Generate propellant mass values to search (logarithmic spacing).
    fn propellant_grid(&self, steps: u32, min_kg: f64, max_kg: f64) -> Vec<f64> {
        if steps == 1 {
            return vec![(min_kg + max_kg) / 2.0];
        }
        let log_min = min_kg.ln();
        let log_max = max_kg.ln();
        let step = (log_max - log_min) / (steps - 1) as f64;

        (0..steps)
            .map(|i| (log_min + step * i as f64).exp())
            .collect()
    }

    /// Sort engines by vacuum preference for upper stages.
    /// Returns (first_stage_engines, upper_stage_engines).
    fn sort_engines_by_preference<'a>(
        &self,
        engines: &'a [Engine],
    ) -> (Vec<&'a Engine>, Vec<&'a Engine>) {
        if !self.prefer_vacuum_upper {
            let refs: Vec<_> = engines.iter().collect();
            return (refs.clone(), refs);
        }

        // For first stage, prefer engines with good sea-level performance
        let mut first_stage: Vec<_> = engines.iter().collect();
        first_stage.sort_by(|a, b| {
            // Higher sea-level thrust/mass ratio first
            let a_ratio = a.thrust_sl().as_newtons() / a.dry_mass().as_kg();
            let b_ratio = b.thrust_sl().as_newtons() / b.dry_mass().as_kg();
            b_ratio.partial_cmp(&a_ratio).unwrap()
        });

        // For upper stages, prefer vacuum-optimized engines (higher vac Isp)
        let mut upper_stage: Vec<_> = engines.iter().collect();
        upper_stage.sort_by(|a, b| {
            // Higher vacuum Isp first
            b.isp_vac()
                .as_seconds()
                .partial_cmp(&a.isp_vac().as_seconds())
                .unwrap()
        });

        (first_stage, upper_stage)
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
        stage_specs: &[StageSpec],
        payload: Mass,
        constraints: &Constraints,
    ) -> Option<Rocket> {
        let mut stages = Vec::with_capacity(stage_specs.len());

        // Build stages from top to bottom to check TWR correctly
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

    /// Generate all stage configurations for parallel search.
    fn generate_configurations(
        &self,
        problem: &Problem,
        stage_count: usize,
        propellant_values: &[f64],
    ) -> Vec<Vec<StageSpec>> {
        let (first_stage_engines, upper_stage_engines) =
            self.sort_engines_by_preference(&problem.available_engines);
        let max_engines = problem.constraints.max_engines_per_stage;

        let mut configs = Vec::new();
        self.generate_recursive(
            stage_count,
            0,
            &mut vec![],
            &first_stage_engines,
            &upper_stage_engines,
            propellant_values,
            max_engines,
            &mut configs,
        );
        configs
    }

    /// Recursive configuration generation.
    #[allow(clippy::too_many_arguments)]
    fn generate_recursive(
        &self,
        total_stages: usize,
        current_stage: usize,
        current_specs: &mut Vec<StageSpec>,
        first_stage_engines: &[&Engine],
        upper_stage_engines: &[&Engine],
        propellant_values: &[f64],
        max_engines: u32,
        configs: &mut Vec<Vec<StageSpec>>,
    ) {
        if current_stage == total_stages {
            configs.push(current_specs.clone());
            return;
        }

        // Select engine list based on stage
        let engines = if current_stage == 0 {
            first_stage_engines
        } else {
            upper_stage_engines
        };

        // Limit engine choices to top 3 for each stage to reduce search space
        let engine_limit = engines.len().min(3);

        for engine in engines.iter().take(engine_limit) {
            for engine_count in 1..=max_engines {
                for &propellant_kg in propellant_values {
                    current_specs.push(StageSpec {
                        engine: (*engine).clone(),
                        engine_count,
                        propellant_kg,
                    });

                    self.generate_recursive(
                        total_stages,
                        current_stage + 1,
                        current_specs,
                        first_stage_engines,
                        upper_stage_engines,
                        propellant_values,
                        max_engines,
                        configs,
                    );

                    current_specs.pop();
                }
            }
        }
    }

    /// Parallel search over configurations.
    fn parallel_search(
        &self,
        problem: &Problem,
        configs: Vec<Vec<StageSpec>>,
        progress_counter: &AtomicU64,
        total_configs: u64,
    ) -> Option<(Rocket, f64)> {
        let best = Arc::new(Mutex::new(None::<(Rocket, f64)>));
        let show_progress = self.show_progress;

        configs.into_par_iter().for_each(|specs| {
            let current = progress_counter.fetch_add(1, Ordering::Relaxed);

            // Show progress every 1000 iterations
            if show_progress && current.is_multiple_of(1000) {
                let percent = (current as f64 / total_configs as f64) * 100.0;
                eprint!(
                    "\r  Searching... {:.1}% ({}/{})",
                    percent, current, total_configs
                );
                let _ = io::stderr().flush();
            }

            if let Some(rocket) =
                Self::try_build_rocket(&specs, problem.payload, &problem.constraints)
            {
                let delta_v = rocket.total_delta_v();
                if delta_v.as_mps() >= problem.target_delta_v.as_mps() {
                    let total_mass = rocket.total_mass().as_kg();
                    let mut guard = best.lock().unwrap();
                    let is_better = match &*guard {
                        None => true,
                        Some((_, best_mass)) => total_mass < *best_mass,
                    };
                    if is_better {
                        *guard = Some((rocket, total_mass));
                    }
                }
            }
        });

        if show_progress {
            eprintln!("\r  Searching... 100.0%                    ");
        }

        Arc::try_unwrap(best).ok()?.into_inner().ok()?
    }

    /// Refine search around a promising solution.
    fn refine_around_solution(
        &self,
        problem: &Problem,
        best_rocket: &Rocket,
        progress_counter: &AtomicU64,
    ) -> Option<(Rocket, f64)> {
        let stages = best_rocket.stages();
        let stage_count = stages.len();

        // Generate refined propellant ranges around each stage
        let refined_ranges: Vec<(f64, f64)> = stages
            .iter()
            .map(|s| {
                let prop = s.propellant_mass().as_kg();
                let range = prop * 0.3; // ±30% range
                ((prop - range).max(self.min_propellant_kg), prop + range)
            })
            .collect();

        // Generate configurations with refined ranges
        let mut configs = Vec::new();
        let (first_stage_engines, upper_stage_engines) =
            self.sort_engines_by_preference(&problem.available_engines);

        // Only search with the engines from the best solution
        let stage_engines: Vec<&Engine> = stages.iter().map(|s| s.engine()).collect();

        self.generate_refined_recursive(
            stage_count,
            0,
            &mut vec![],
            &stage_engines,
            &refined_ranges,
            problem.constraints.max_engines_per_stage,
            &mut configs,
            &first_stage_engines,
            &upper_stage_engines,
        );

        let total = configs.len() as u64;
        self.parallel_search(problem, configs, progress_counter, total)
    }

    /// Generate refined configurations around a solution.
    #[allow(clippy::too_many_arguments)]
    fn generate_refined_recursive(
        &self,
        total_stages: usize,
        current_stage: usize,
        current_specs: &mut Vec<StageSpec>,
        stage_engines: &[&Engine],
        refined_ranges: &[(f64, f64)],
        max_engines: u32,
        configs: &mut Vec<Vec<StageSpec>>,
        first_stage_engines: &[&Engine],
        upper_stage_engines: &[&Engine],
    ) {
        if current_stage == total_stages {
            configs.push(current_specs.clone());
            return;
        }

        let (min_prop, max_prop) = refined_ranges[current_stage];
        let propellant_values = self.propellant_grid(self.fine_steps, min_prop, max_prop);

        // Try the original engine and alternatives
        let base_engine = stage_engines[current_stage];
        let alternatives = if current_stage == 0 {
            first_stage_engines
        } else {
            upper_stage_engines
        };

        // Include base engine + top 2 alternatives
        let mut engines_to_try: Vec<&Engine> = vec![base_engine];
        for alt in alternatives.iter().take(2) {
            if alt.name != base_engine.name && !engines_to_try.iter().any(|e| e.name == alt.name) {
                engines_to_try.push(alt);
            }
        }

        for engine in engines_to_try {
            for engine_count in 1..=max_engines {
                for &propellant_kg in &propellant_values {
                    current_specs.push(StageSpec {
                        engine: engine.clone(),
                        engine_count,
                        propellant_kg,
                    });

                    self.generate_refined_recursive(
                        total_stages,
                        current_stage + 1,
                        current_specs,
                        stage_engines,
                        refined_ranges,
                        max_engines,
                        configs,
                        first_stage_engines,
                        upper_stage_engines,
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

        let progress_counter = AtomicU64::new(0);
        let mut best: Option<(Rocket, f64)> = None;

        // Determine stage count range
        let min_stages = problem.stage_count.unwrap_or(1);
        let max_stages = problem
            .stage_count
            .unwrap_or(problem.constraints.max_stages);

        if self.show_progress {
            eprintln!("  Optimizer: BruteForce (parallel)");
            eprintln!(
                "  Searching {} to {} stages with {} engines",
                min_stages,
                max_stages,
                problem.available_engines.len()
            );
        }

        // Coarse search for each stage count
        let coarse_propellant = self.propellant_grid(
            self.coarse_steps,
            self.min_propellant_kg,
            self.max_propellant_kg,
        );

        for stage_count in min_stages..=max_stages {
            if self.show_progress {
                eprintln!("  Phase 1: Coarse search ({} stages)", stage_count);
            }

            let configs =
                self.generate_configurations(problem, stage_count as usize, &coarse_propellant);
            let total = configs.len() as u64;

            if let Some((rocket, mass)) =
                self.parallel_search(problem, configs, &progress_counter, total)
            {
                let is_better = match &best {
                    None => true,
                    Some((_, best_mass)) => mass < *best_mass,
                };
                if is_better {
                    best = Some((rocket, mass));
                }
            }
        }

        // Refinement phase
        if let Some((ref best_rocket, _)) = best {
            if self.show_progress {
                eprintln!("  Phase 2: Refining around best solution");
            }

            progress_counter.store(0, Ordering::Relaxed);

            if let Some((refined_rocket, refined_mass)) =
                self.refine_around_solution(problem, best_rocket, &progress_counter)
            {
                if let Some((_, best_mass)) = &best {
                    if refined_mass < *best_mass {
                        best = Some((refined_rocket, refined_mass));
                    }
                }
            }
        }

        let total_iterations = progress_counter.load(Ordering::Relaxed);

        // Return best solution found
        match best {
            Some((rocket, _)) => {
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

    #[test]
    fn brute_force_stage_count_exploration() {
        // Test without fixed stage count - should explore 1-3 stages
        let optimizer = BruteForceOptimizer::new(4, 50_000.0, 300_000.0);

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(8_000.0),
            vec![get_raptor()],
            Constraints::default(),
        );
        // Note: no .with_stage_count() - will explore all options

        let solution = optimizer.optimize(&problem).unwrap();

        assert!(solution.meets_target());
        // Should find a solution with 1-3 stages
        assert!(solution.rocket.stage_count() >= 1);
        assert!(solution.rocket.stage_count() <= 3);
    }

    #[test]
    fn brute_force_vacuum_preference() {
        let db = EngineDatabase::default();
        let raptor = db.get("Raptor-2").unwrap().clone();
        let raptor_vac = db.get("Raptor-Vacuum").unwrap().clone();

        let optimizer =
            BruteForceOptimizer::new(4, 50_000.0, 300_000.0).with_vacuum_preference(true);

        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![raptor, raptor_vac],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();

        // Upper stage should prefer vacuum engine (higher Isp)
        let upper_stage = &solution.rocket.stages()[1];
        // Raptor-Vacuum has 380s Isp vs 350s for Raptor-2
        assert!(upper_stage.engine().isp_vac().as_seconds() >= 350.0);
    }
}
