//! Monte Carlo simulation for uncertainty analysis.
//!
//! The theory is documented on the public type below, where rustdoc shows it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use serde::Serialize;

use super::solution::DELTA_V_TOLERANCE_MPS;
use super::{
    AnalyticalOptimizer, OptimizeError, Optimizer, ParameterSampler, Problem, Solution, Uncertainty,
};
use crate::units::Velocity;

/// Results from a Monte Carlo simulation.
///
/// Contains the distribution of outcomes from building one design many
/// times with perturbed parameters.
#[derive(Debug, Clone)]
pub struct MonteCarloResults {
    /// Delta-v achieved by each build (m/s)
    pub delta_v_samples: Vec<f64>,

    /// Liftoff mass of each build (kg)
    pub mass_samples: Vec<f64>,

    /// Builds that reached the target delta-v and could lift off
    pub successes: u64,

    /// Total number of builds evaluated
    pub total_runs: u64,

    /// Builds too heavy for their thrust to leave the pad (liftoff TWR ≤ 1)
    pub failures: u64,

    /// Target delta-v used for success calculation
    pub target_delta_v: Velocity,

    /// Time taken to run the simulation
    pub runtime: Duration,

    /// The design that was stressed
    pub nominal_solution: Solution,

    /// Seed that reproduces this run with [`MonteCarloRunner::with_seed`]
    pub seed: u64,
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
        let variance = self
            .delta_v_samples
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>()
            / (self.delta_v_samples.len() - 1) as f64;
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
    /// ```
    /// # use tsiolkovsky::engine::EngineDatabase;
    /// # use tsiolkovsky::optimizer::{Constraints, MonteCarloRunner, Problem, Uncertainty};
    /// # use tsiolkovsky::units::{Mass, Velocity};
    /// # let db = EngineDatabase::load_embedded().unwrap();
    /// # let problem = Problem::new(
    /// #     Mass::kg(5_000.0),
    /// #     Velocity::mps(9_400.0),
    /// #     vec![db.get("raptor-2").unwrap().clone()],
    /// #     Constraints::default(),
    /// # );
    /// let results = MonteCarloRunner::new(Uncertainty::default())
    ///     .with_seed(1)
    ///     .run(&problem, 2_000)
    ///     .unwrap();
    ///
    /// // How much margin for 95% confidence? A zero-margin design needs some.
    /// let margin = results.required_margin(0.95);
    /// assert!(margin > 0.0);
    /// println!("Need {margin:.0} m/s margin for 95% confidence");
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
                min: self
                    .delta_v_samples
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min),
                max: self
                    .delta_v_samples
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max),
            },
            mass: DistributionSummary {
                mean: self.mean_mass(),
                std_dev: 0.0, // Could add std_mass() if needed
                percentile_5: self.mass_percentile(5.0),
                percentile_50: self.mass_percentile(50.0),
                percentile_95: self.mass_percentile(95.0),
                min: self
                    .mass_samples
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min),
                max: self
                    .mass_samples
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max),
            },
            required_margin_95_mps: self.required_margin(0.95),
            seed: self.seed,
            design_total_mass_kg: self.nominal_solution.rocket.total_mass().as_kg(),
            design_stage_count: self.nominal_solution.rocket.stage_count(),
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

    /// Seed that reproduces this run
    pub seed: u64,

    /// Liftoff mass of the design that was stressed (kg)
    pub design_total_mass_kg: f64,

    /// Number of stages in the design that was stressed
    pub design_stage_count: usize,
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
    sorted.sort_by(f64::total_cmp);

    let p = percentile.clamp(0.0, 100.0) / 100.0;
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Monte Carlo simulation runner.
///
/// A design on paper hits its target exactly. The rocket that gets built
/// won't: engines come off the line a percent or so off their rated Isp,
/// welds and wiring make the structure heavier or lighter than drawn, and
/// thrust varies with chamber pressure. Monte Carlo analysis asks how
/// often *this design*, built with those errors, still does its job.
///
/// # How It Works
///
/// 1. Optimize the nominal problem, or take a design you already have.
/// 2. For each iteration, build the design again with every stage's as-built
///    parameters drawn from their distributions:
///    - one Isp factor per stage, scaling sea-level and vacuum Isp together
///      (a worse engine is worse at every altitude)
///    - one thrust factor per stage, likewise
///    - one factor on each stage's structural mass
/// 3. Evaluate the delta-v and liftoff TWR of each build.
///
/// The design itself never changes between samples. Earlier versions of tsi
/// re-optimized every sample, which measured whether *some* rocket could be
/// found under perturbed parameters. That is a different question, and it
/// always came out "yes".
///
/// # Reproducibility
///
/// Every run has a seed, reported in the results. Each sample draws from its
/// own generator seeded by (seed, sample index), so the same seed gives
/// bit-identical results however many threads rayon uses.
///
/// # Interpreting Results
///
/// - **Success probability**: fraction of builds reaching the target delta-v
///   that can also lift off (liftoff TWR > 1)
/// - **Confidence intervals**: range of delta-v the builds achieve
/// - **Required margin**: how much delta-v to design in for a given confidence
///
/// A zero-margin design succeeds about half the time, since half of all builds
/// come out below nominal. Adding margin ([`Constraints::margin`](super::Constraints::margin)) is how a
/// design buys confidence.
///
/// # Example
///
/// ```
/// use tsiolkovsky::optimizer::{Problem, Constraints, Uncertainty, MonteCarloRunner};
/// use tsiolkovsky::engine::EngineDatabase;
/// use tsiolkovsky::units::{Mass, Ratio, Velocity};
///
/// let db = EngineDatabase::load_embedded().expect("load db");
/// let engine = db.get("raptor-2").expect("engine");
///
/// let problem = Problem::new(
///     Mass::kg(5_000.0),
///     Velocity::mps(9_400.0),
///     vec![engine.clone()],
///     Constraints::default().with_margin(Ratio::new(0.02)),
/// ).with_stage_count(2);
///
/// let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(42);
/// let results = runner.run(&problem, 1000).expect("monte carlo");
///
/// // 2% margin covers most, but not all, manufacturing variation
/// assert!(results.success_probability() > 0.9);
/// println!("Delta-v 5th percentile: {:.0} m/s", results.delta_v_percentile(5.0));
/// ```
///
/// Builds a fixed design many times with perturbed as-built parameters and
/// measures how often it still meets its target.
#[derive(Debug, Clone)]
pub struct MonteCarloRunner {
    uncertainty: Uncertainty,
    show_progress: bool,
    seed: Option<u64>,
}

impl MonteCarloRunner {
    /// Create a new Monte Carlo runner with the given uncertainty.
    pub fn new(uncertainty: Uncertainty) -> Self {
        Self {
            uncertainty,
            show_progress: false,
            seed: None,
        }
    }

    /// Enable progress reporting to stderr.
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Use a fixed seed, making results reproducible.
    ///
    /// Without one, a random seed is chosen and reported in
    /// [`MonteCarloResults::seed`], so any run can be repeated later.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Optimize the problem, then stress the resulting design.
    ///
    /// # Errors
    ///
    /// Returns an error if the problem is invalid or has no solution.
    pub fn run(
        &self,
        problem: &Problem,
        iterations: u64,
    ) -> Result<MonteCarloResults, OptimizeError> {
        let nominal = AnalyticalOptimizer.optimize(problem)?;
        Ok(self.run_design(&nominal, iterations))
    }

    /// Stress an existing design: build it `iterations` times with
    /// perturbed parameters and evaluate each build against the design's
    /// target delta-v.
    pub fn run_design(&self, design: &Solution, iterations: u64) -> MonteCarloResults {
        let start = Instant::now();
        let seed = self.seed.unwrap_or_else(rand::random);
        let target = design.target_delta_v;
        let nominal = &design.rocket;

        // A build succeeds if it reaches the target (allowing the rounding
        // that a design sized exactly to it carries) and can leave the pad.
        let succeeds =
            |s: &Sample| s.lifts_off && s.delta_v >= target.as_mps() - DELTA_V_TOLERANCE_MPS;

        if self.uncertainty.is_zero() {
            // Every build is the nominal design, so evaluate it once and
            // count it as many times as asked.
            let sample = Sample {
                delta_v: nominal.total_delta_v().as_mps(),
                mass: nominal.total_mass().as_kg(),
                lifts_off: nominal.liftoff_twr().as_f64() > 1.0,
            };
            let once = |x: f64| if iterations == 0 { vec![] } else { vec![x] };
            return MonteCarloResults {
                delta_v_samples: once(sample.delta_v),
                mass_samples: once(sample.mass),
                successes: if succeeds(&sample) { iterations } else { 0 },
                total_runs: iterations,
                failures: if sample.lifts_off { 0 } else { iterations },
                target_delta_v: target,
                runtime: start.elapsed(),
                nominal_solution: design.clone(),
                seed,
            };
        }

        let sampler = ParameterSampler::new(self.uncertainty);
        let completed = AtomicU64::new(0);

        let samples: Vec<Sample> = (0..iterations)
            .into_par_iter()
            .map(|i| {
                let mut rng = StdRng::seed_from_u64(sample_seed(seed, i));
                let built = sampler.perturb_rocket(nominal, &mut rng);
                let sample = Sample {
                    delta_v: built.total_delta_v().as_mps(),
                    mass: built.total_mass().as_kg(),
                    lifts_off: built.liftoff_twr().as_f64() > 1.0,
                };

                if self.show_progress {
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if done.is_multiple_of((iterations / 100).max(1)) {
                        let pct = done as f64 / iterations as f64 * 100.0;
                        eprint!("\rMonte Carlo: {pct:.0}% ({done}/{iterations})");
                    }
                }
                sample
            })
            .collect();

        if self.show_progress {
            eprintln!("\rMonte Carlo: 100% ({iterations}/{iterations})");
        }

        let successes = samples.iter().filter(|s| succeeds(s)).count() as u64;
        let failures = samples.iter().filter(|s| !s.lifts_off).count() as u64;

        MonteCarloResults {
            delta_v_samples: samples.iter().map(|s| s.delta_v).collect(),
            mass_samples: samples.iter().map(|s| s.mass).collect(),
            successes,
            total_runs: iterations,
            failures,
            target_delta_v: target,
            runtime: start.elapsed(),
            nominal_solution: design.clone(),
            seed,
        }
    }
}

/// One perturbed build of the design.
struct Sample {
    delta_v: f64,
    mass: f64,
    lifts_off: bool,
}

/// Derive an independent, well-mixed seed for one sample (SplitMix64).
fn sample_seed(seed: u64, index: u64) -> u64 {
    let mut z = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, EngineDatabase};
    use crate::optimizer::Constraints;
    use crate::units::{Mass, Ratio};

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
        )
        .with_stage_count(2)
    }

    #[test]
    fn monte_carlo_zero_uncertainty() {
        let runner = MonteCarloRunner::new(Uncertainty::none());
        let problem = simple_problem();

        let results = runner
            .run(&problem, 10)
            .expect("monte carlo should succeed");

        // Every build is the nominal design, which hits its target exactly:
        // all 10 succeed, and the count reports the 10 asked for.
        assert_eq!(results.successes, 10);
        assert_eq!(results.total_runs, 10);
        assert_eq!(results.success_probability(), 1.0);
    }

    #[test]
    fn zero_margin_design_succeeds_about_half_the_time() {
        // Half of all builds come out below nominal, so a design with no
        // margin should fail roughly half the time. v0.6 reported 100%.
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1);
        let results = runner.run(&simple_problem(), 2_000).unwrap();

        assert_eq!(results.total_runs, 2_000);
        let p = results.success_probability();
        assert!((0.3..0.7).contains(&p), "success probability {p}");
        let mean = results.mean_delta_v();
        assert!((mean - 9_400.0).abs() < 50.0, "mean delta-v {mean}");
    }

    #[test]
    fn margin_buys_confidence() {
        let mut problem = simple_problem();
        problem.constraints = Constraints::default().with_margin(Ratio::new(0.03));
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1);
        let results = runner.run(&problem, 2_000).unwrap();
        assert!(results.success_probability() > 0.95);
    }

    #[test]
    fn stresses_the_design_it_was_given() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let results = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(7)
            .run_design(&design, 200);
        assert_eq!(
            results.nominal_solution.rocket.total_mass().as_kg(),
            design.rocket.total_mass().as_kg()
        );
        // Only structure is perturbed, so liftoff mass stays within a few
        // percent of nominal rather than being re-optimized each time.
        let nominal = design.rocket.total_mass().as_kg();
        for &m in &results.mass_samples {
            assert!((m / nominal - 1.0).abs() < 0.05, "{m} vs {nominal}");
        }
    }

    #[test]
    fn seed_makes_runs_reproducible_across_thread_counts() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(42);
        let many = runner.run_design(&design, 500);
        let one = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap()
            .install(|| runner.run_design(&design, 500));

        assert_eq!(many.seed, 42);
        assert_eq!(many.delta_v_samples, one.delta_v_samples);
        assert_eq!(many.successes, one.successes);
    }

    #[test]
    fn lunar_designs_are_judged_against_lunar_gravity() {
        // A Moon launch with liftoff TWR 1.3 at 1.62 m/s² has TWR 0.2 at g₀.
        // Monte Carlo used to judge it at g₀ and call every build a failure.
        let db = EngineDatabase::default();
        let problem = Problem::new(
            Mass::kg(300_000.0),
            Velocity::mps(4_000.0),
            vec![db.get("merlin-1d").unwrap().clone()],
            Constraints::default()
                .with_surface_gravity(1.62)
                .with_booster_isp(crate::physics::IspModel::Vacuum)
                .with_max_engines(30)
                .with_margin(Ratio::new(0.03)),
        );
        let results = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(3)
            .run(&problem, 500)
            .unwrap();
        assert_eq!(results.failures, 0, "builds judged unable to lift off");
        assert!(results.success_probability() > 0.9);
    }

    #[test]
    fn unseeded_runs_report_their_seed() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let first = MonteCarloRunner::new(Uncertainty::default()).run_design(&design, 100);
        let again = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(first.seed)
            .run_design(&design, 100);
        assert_eq!(first.delta_v_samples, again.delta_v_samples);
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
                target_delta_v: Velocity::mps(9500.0),
                margin: Velocity::mps(100.0),
                iterations: 1,
                runtime: Duration::from_secs(0),
                optimizer_name: "test".to_string(),
            },
            seed: 0,
        };

        assert!((results.success_probability() - 0.8).abs() < 0.01);
        assert!((results.mean_delta_v() - 9600.0).abs() < 1.0);
        assert!(results.std_delta_v() > 0.0);
    }
}
