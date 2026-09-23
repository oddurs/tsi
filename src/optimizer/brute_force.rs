//! Brute force grid search over staging configurations.
//!
//! Where the [`AnalyticalOptimizer`](super::AnalyticalOptimizer) reasons its
//! way to an optimum, this optimizer simply tries configurations and keeps
//! the lightest one that works. It shares no search logic with the analytical
//! optimizer, which is what makes it useful as a cross-check: when they
//! agree, both are probably right.
//!
//! # Search Space
//!
//! - Number of stages (from the problem's stage count range)
//! - Engine type per stage (every candidate engine, no pruning)
//! - Propellant mass per stage (a logarithmic grid)
//!
//! Engine count is not searched. For a given engine and propellant load the
//! fewest engines that meet the TWR constraint are always best, since extra
//! engines only add dry mass, and that number has a closed form.
//!
//! # Search Strategy
//!
//! 1. **Coarse search** over a propellant grid scaled to the payload, from
//!    a twentieth of the payload mass to two thousand times it.
//!    If nothing feasible turns up, the grid doubles in density, up to
//!    three times: near the limits of an engine the feasible region is thin.
//! 2. **Refinement**: repeated finer grids centred on the best configuration,
//!    halving the span (in log space) each round.
//! 3. **Parallel and streaming**: rayon splits the search at the top stage,
//!    and each thread walks its share depth-first. Memory stays constant no
//!    matter how large the grid is.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rayon::prelude::*;

use crate::stage::{Rocket, Stage};
use crate::units::Mass;

use super::sizing::{size_for_propellant, StageEngine};
use super::{OptimizeError, Optimizer, Problem, Solution};

/// Default propellant grid, as multiples of the payload mass.
const DEFAULT_MIN_PROPELLANT_PER_PAYLOAD: f64 = 0.05;
const DEFAULT_MAX_PROPELLANT_PER_PAYLOAD: f64 = 2_000.0;

/// How many times the coarse grid may double in density when it finds nothing.
const COARSE_ATTEMPTS: u32 = 4;

/// Brute force optimizer for staging problems.
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
/// let optimizer = BruteForceOptimizer::default().with_progress(false);
/// let solution = optimizer.optimize(&problem).expect("optimization failed");
///
/// assert!(solution.meets_target());
/// ```
#[derive(Debug, Clone)]
pub struct BruteForceOptimizer {
    /// Propellant grid points per stage in the coarse search
    coarse_steps: u32,
    /// Propellant grid points per stage in each refinement round
    fine_steps: u32,
    /// Number of refinement rounds
    refine_rounds: u32,
    /// Explicit propellant bounds in kg; `None` scales them to the payload
    propellant_bounds: Option<(f64, f64)>,
    /// Show progress indicator
    show_progress: bool,
}

impl Default for BruteForceOptimizer {
    fn default() -> Self {
        Self {
            coarse_steps: 24,
            fine_steps: 11,
            refine_rounds: 8,
            propellant_bounds: None,
            show_progress: true,
        }
    }
}

impl BruteForceOptimizer {
    /// Create a brute force optimizer with an explicit propellant grid.
    ///
    /// `propellant_steps` points per stage, logarithmically spaced from
    /// `min_propellant_kg` to `max_propellant_kg`.
    pub fn new(propellant_steps: u32, min_propellant_kg: f64, max_propellant_kg: f64) -> Self {
        Self {
            coarse_steps: propellant_steps.max(1),
            fine_steps: propellant_steps / 2 + 1,
            propellant_bounds: Some((min_propellant_kg, max_propellant_kg)),
            show_progress: false,
            ..Self::default()
        }
    }

    /// Enable or disable progress indicator.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Formerly limited upper stages to high-Isp engines.
    ///
    /// The search now tries every engine on every stage, so the best upper
    /// stage engine is found without being told. This has no effect.
    #[deprecated(since = "0.7.0", note = "every engine is now searched on every stage")]
    pub fn with_vacuum_preference(self, _prefer: bool) -> Self {
        self
    }

    /// Logarithmically spaced values from `min` to `max`.
    fn log_grid(steps: u32, min: f64, max: f64) -> Vec<f64> {
        if steps <= 1 {
            return vec![(min * max).sqrt()];
        }
        let (log_min, log_max) = (min.ln(), max.ln());
        let step = (log_max - log_min) / (steps - 1) as f64;
        (0..steps)
            .map(|i| (log_min + step * i as f64).exp())
            .collect()
    }

    /// Search one grid: `grids[i]` are the propellant loads to try on stage
    /// `i`, `engines[i]` the engines. Returns the lightest feasible rocket as
    /// (total mass, per-stage (engine index, engine count, propellant)).
    fn search_grid(
        &self,
        problem: &Problem,
        engines: &[Vec<StageEngine<'_>>],
        grids: &[Vec<f64>],
        counter: &AtomicU64,
        progress: Option<&Progress>,
    ) -> Option<(f64, Vec<Choice>)> {
        let n = engines.len();
        let top = n - 1;
        let top_choices: Vec<(usize, f64)> = (0..engines[top].len())
            .flat_map(|e| grids[top].iter().map(move |&p| (e, p)))
            .collect();
        let design_dv = problem.design_delta_v().as_mps();

        top_choices
            .into_par_iter()
            .filter_map(|(e, p)| {
                let mut walk = Walk {
                    problem,
                    engines,
                    grids,
                    design_dv,
                    booster_floor: problem.constraints.booster_delta_v_floor(n).as_mps(),
                    path: Vec::with_capacity(n),
                    best: None,
                    evaluated: 0,
                };
                walk.stage(top, e, p, problem.payload.as_kg(), 0.0);
                counter.fetch_add(walk.evaluated, Ordering::Relaxed);
                if let Some(progress) = progress {
                    progress.tick();
                }
                walk.best
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}

/// One stage's choice in a candidate rocket.
#[derive(Debug, Clone, Copy)]
struct Choice {
    engine: usize,
    count: u32,
    propellant: f64,
}

/// Depth-first walk of the grid below one top-stage choice.
struct Walk<'a, 'e> {
    problem: &'a Problem,
    engines: &'a [Vec<StageEngine<'e>>],
    grids: &'a [Vec<f64>],
    design_dv: f64,
    /// Least delta-v stage 0 must deliver
    booster_floor: f64,
    /// Choices from the top stage down to the current one
    path: Vec<Choice>,
    best: Option<(f64, Vec<Choice>)>,
    evaluated: u64,
}

impl Walk<'_, '_> {
    fn stage(&mut self, index: usize, engine: usize, propellant: f64, above: f64, dv: f64) {
        let constraints = &self.problem.constraints;
        let Ok((sized, stage_dv)) = size_for_propellant(
            &self.engines[index][engine],
            propellant,
            above,
            constraints.structural_ratio.as_f64(),
            constraints.surface_gravity,
            constraints.max_engines_per_stage,
        ) else {
            self.evaluated += 1;
            return;
        };
        self.path.push(Choice {
            engine,
            count: sized.engine_count,
            propellant,
        });
        let dv = dv + stage_dv;
        if index == 0 {
            self.evaluated += 1;
            if stage_dv < self.booster_floor {
                self.path.pop();
                return;
            }
            let lighter = self
                .best
                .as_ref()
                .is_none_or(|(mass, _)| sized.stack_mass < *mass);
            if dv >= self.design_dv && lighter {
                let mut stages = self.path.clone();
                stages.reverse();
                self.best = Some((sized.stack_mass, stages));
            }
        } else {
            let below = index - 1;
            for e in 0..self.engines[below].len() {
                for k in 0..self.grids[below].len() {
                    let p = self.grids[below][k];
                    self.stage(below, e, p, sized.stack_mass, dv);
                }
            }
        }
        self.path.pop();
    }
}

/// Progress reporting to stderr.
struct Progress {
    done: AtomicU64,
    total: u64,
}

impl Progress {
    fn tick(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        if done.is_multiple_of((self.total / 100).max(1)) || done == self.total {
            let percent = done as f64 / self.total as f64 * 100.0;
            eprint!("\r  Searching... {percent:.0}%");
            let _ = io::stderr().flush();
        }
    }
}

impl Optimizer for BruteForceOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        let start = Instant::now();
        problem.is_valid()?;

        let payload = problem.payload.as_kg();
        let (min_p, max_p) = self.propellant_bounds.unwrap_or((
            payload * DEFAULT_MIN_PROPELLANT_PER_PAYLOAD,
            payload * DEFAULT_MAX_PROPELLANT_PER_PAYLOAD,
        ));
        let (min_stages, max_stages) = problem.stage_count_range();
        let counter = AtomicU64::new(0);
        let mut best: Option<(f64, Vec<Choice>, Vec<Vec<StageEngine>>)> = None;

        if self.show_progress {
            eprintln!("  Optimizer: BruteForce (parallel)");
        }

        for stage_count in min_stages..=max_stages {
            let engines: Vec<Vec<StageEngine>> = (0..stage_count as usize)
                .map(|i| {
                    problem
                        .engines_for_stage(i)
                        .into_iter()
                        .map(|e| StageEngine::new(e, i, problem))
                        .collect()
                })
                .collect();
            if engines.iter().any(Vec::is_empty) {
                continue;
            }

            // Coarse search. Near the edge of what the engines can do, the
            // feasible region is a thin sliver that a coarse grid can step
            // over entirely, so if nothing is found, double the density and
            // try again.
            let mut found = None;
            let mut coarse_ratio = max_p / min_p;
            for attempt in 0..COARSE_ATTEMPTS {
                let steps = self.coarse_steps << attempt;
                let coarse = Self::log_grid(steps, min_p, max_p);
                if coarse.len() > 1 {
                    coarse_ratio = coarse[1] / coarse[0];
                }
                let grids = vec![coarse.clone(); stage_count as usize];
                let progress = self.show_progress.then(|| {
                    eprintln!("  Coarse search: {stage_count} stage(s), {steps} steps");
                    Progress {
                        done: AtomicU64::new(0),
                        total: (engines[stage_count as usize - 1].len() * coarse.len()) as u64,
                    }
                });
                found = self.search_grid(problem, &engines, &grids, &counter, progress.as_ref());
                if self.show_progress {
                    eprintln!();
                }
                if found.is_some() {
                    break;
                }
            }
            let Some((mass, choices)) = found else {
                continue;
            };

            // Refine: finer grids around the best point, halving the span in
            // log space each round. Engines stay free in every round.
            let (mut mass, mut choices) = (mass, choices);
            let mut span = coarse_ratio;
            for _ in 0..self.refine_rounds {
                let grids: Vec<Vec<f64>> = choices
                    .iter()
                    .map(|c| {
                        Self::log_grid(self.fine_steps, c.propellant / span, c.propellant * span)
                    })
                    .collect();
                if let Some((m, c)) = self.search_grid(problem, &engines, &grids, &counter, None) {
                    if m < mass {
                        mass = m;
                        choices = c;
                    }
                }
                span = span.sqrt();
            }

            if best.as_ref().is_none_or(|(b, _, _)| mass < *b) {
                best = Some((mass, choices, engines));
            }
        }

        let iterations = counter.load(Ordering::Relaxed);
        let Some((_, choices, engines)) = best else {
            return Err(OptimizeError::Infeasible {
                reason: format!(
                    "No feasible configuration found after {iterations} evaluations.\n\n\
                    Suggestions:\n  \
                    - Allow more stages with --max-stages\n  \
                    - Lower --min-twr to allow lower thrust-to-weight\n  \
                    - Try different engines with higher ISP or thrust\n  \
                    - Reduce target delta-v or payload mass"
                ),
            });
        };

        let tankage = problem.constraints.structural_ratio.as_f64();
        let stages = choices
            .iter()
            .enumerate()
            .map(|(i, c)| {
                Stage::with_structural_ratio(
                    engines[i][c.engine].engine.clone(),
                    c.count,
                    Mass::kg(c.propellant),
                    tankage,
                )
            })
            .collect();
        let rocket =
            Rocket::new(stages, problem.payload).with_booster_isp(problem.constraints.booster_isp);

        Ok(Solution::with_metadata(
            rocket,
            problem.target_delta_v,
            iterations,
            start.elapsed(),
            "BruteForce",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, EngineDatabase};
    use crate::optimizer::{AnalyticalOptimizer, Constraints};
    use crate::units::{Ratio, Velocity};

    fn get(name: &str) -> Engine {
        EngineDatabase::default().get(name).unwrap().clone()
    }

    fn quiet() -> BruteForceOptimizer {
        BruteForceOptimizer::default().with_progress(false)
    }

    #[test]
    fn brute_force_single_engine() {
        let optimizer = BruteForceOptimizer::new(5, 50_000.0, 500_000.0);
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get("raptor-2")],
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
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get("raptor-2"), get("merlin-1d")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = quiet().optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert_eq!(solution.rocket.stage_count(), 2);
    }

    #[test]
    fn iterations_count_every_evaluation() {
        // 1 engine, 3 grid points, 2 stages: the coarse pass alone evaluates
        // 3 × 3 = 9 configurations, and refinement adds more.
        let optimizer = BruteForceOptimizer::new(3, 20_000.0, 300_000.0);
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(8_000.0),
            vec![get("raptor-2")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = optimizer.optimize(&problem).unwrap();
        assert!(solution.iterations > 9, "{}", solution.iterations);
    }

    #[test]
    fn brute_force_respects_twr() {
        let constraints = Constraints::new(Ratio::new(1.3), Ratio::new(0.7), 2, Ratio::new(0.08));
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get("raptor-2")],
            constraints,
        )
        .with_stage_count(2);

        let solution = quiet().optimize(&problem).unwrap();

        assert!(solution.rocket.liftoff_twr().as_f64() >= 1.3);
        assert!(solution.rocket.stage_twr(1).as_f64() >= 0.7);
    }

    #[test]
    fn brute_force_infeasible_returns_error() {
        let optimizer = BruteForceOptimizer::new(3, 1_000.0, 10_000.0);
        let problem = Problem::new(
            Mass::kg(100_000.0),
            Velocity::mps(15_000.0),
            vec![get("merlin-1d")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let result = optimizer.optimize(&problem);

        assert!(matches!(result, Err(OptimizeError::Infeasible { .. })));
    }

    #[test]
    fn brute_force_stage_count_exploration() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(8_000.0),
            vec![get("raptor-2")],
            Constraints::default(),
        );

        let solution = quiet().optimize(&problem).unwrap();

        assert!(solution.meets_target());
        assert!((1..=3).contains(&solution.rocket.stage_count()));
    }

    #[test]
    fn finds_hydrogen_upper_stage_without_being_told() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get("raptor-2"), get("raptor-vacuum")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = quiet().optimize(&problem).unwrap();
        assert_eq!(solution.rocket.stages()[0].engine().name, "Raptor-2");
        assert_eq!(solution.rocket.stages()[1].engine().name, "Raptor-Vacuum");
    }

    #[test]
    fn finds_small_rockets() {
        // v0.6 had a fixed 10 t propellant floor and called this infeasible.
        let problem = Problem::new(
            Mass::kg(300.0),
            Velocity::mps(9_400.0),
            vec![get("rutherford")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let solution = quiet().optimize(&problem).unwrap();
        assert!(solution.meets_target());
        assert!(solution.rocket.total_mass().as_kg() < 30_000.0);
    }

    #[test]
    fn super_heavy_engine_counts_are_expressible() {
        // 100 t to a Starship-class orbit needs dozens of Raptors on the booster.
        let problem = Problem::new(
            Mass::kg(100_000.0),
            Velocity::mps(9_400.0),
            vec![get("raptor-2")],
            Constraints::default().with_max_engines(40),
        )
        .with_stage_count(2);

        let solution = quiet().optimize(&problem).unwrap();
        assert!(solution.rocket.stages()[0].engine_count() > 9);
    }

    #[test]
    fn agrees_with_analytical() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_400.0),
            vec![get("raptor-2")],
            Constraints::default(),
        )
        .with_stage_count(2);

        let brute = quiet().optimize(&problem).unwrap();
        let analytical = AnalyticalOptimizer.optimize(&problem).unwrap();
        let b = brute.rocket.total_mass().as_kg();
        let a = analytical.rocket.total_mass().as_kg();
        assert!(a <= b * 1.001, "analytical {a:.0} vs brute force {b:.0}");
        assert!(b <= a * 1.01, "brute force {b:.0} vs analytical {a:.0}");
    }
}
