//! Monte Carlo simulation for uncertainty analysis.
//!
//! The theory is documented on [`MonteCarloRunner`], where rustdoc shows it.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use serde::Serialize;

use super::solution::DELTA_V_TOLERANCE_MPS;
use super::uncertainty::ParameterSampler;
use super::{
    AnalyticalOptimizer, OptimizeError, Optimizer, Problem, Progress, Solution, Uncertainty,
    UncertaintyError,
};
use crate::units::{Mass, Velocity};

/// Results from a Monte Carlo simulation.
///
/// The distribution of outcomes from building one design many times with
/// perturbed parameters.
#[derive(Debug, Clone)]
pub struct MonteCarloResults {
    delta_v_samples: Vec<f64>,
    mass_samples: Vec<f64>,
    successes: u64,
    total_runs: u64,
    failures: u64,
    target_delta_v: Velocity,
    runtime: Duration,
    design: Solution,
    seed: u64,
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

    /// Delta-v achieved by each build (m/s). With no uncertainty every build
    /// is the same, and this holds the one value once.
    pub fn delta_v_samples(&self) -> &[f64] {
        &self.delta_v_samples
    }

    /// Liftoff mass of each build (kg).
    pub fn mass_samples(&self) -> &[f64] {
        &self.mass_samples
    }

    /// Builds that reached the target delta-v and could lift off.
    pub fn successes(&self) -> u64 {
        self.successes
    }

    /// Builds evaluated.
    pub fn total_runs(&self) -> u64 {
        self.total_runs
    }

    /// Builds too heavy for their thrust to leave the pad (liftoff TWR ≤ 1).
    pub fn failures(&self) -> u64 {
        self.failures
    }

    /// The delta-v each build was judged against.
    pub fn target_delta_v(&self) -> Velocity {
        self.target_delta_v
    }

    /// Time the simulation took.
    pub fn runtime(&self) -> Duration {
        self.runtime
    }

    /// The design that was stressed.
    pub fn design(&self) -> &Solution {
        &self.design
    }

    /// Seed that reproduces this run with [`MonteCarloRunner::with_seed`].
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Get a percentile (0-100) of the delta-v distribution, in m/s, or 0
    /// with no samples.
    ///
    /// - 5th percentile: "worst case" performance
    /// - 50th percentile: median performance
    /// - 95th percentile: "best case" performance
    pub fn delta_v_percentile(&self, percentile: f64) -> f64 {
        percentile_of(&self.delta_v_samples, percentile)
    }

    /// Get a percentile (0-100) of the liftoff mass distribution, in kg.
    pub fn mass_percentile(&self, percentile: f64) -> f64 {
        percentile_of(&self.mass_samples, percentile)
    }

    /// Mean delta-v across all builds.
    pub fn mean_delta_v(&self) -> f64 {
        mean(&self.delta_v_samples)
    }

    /// Standard deviation of delta-v across all builds.
    pub fn std_delta_v(&self) -> f64 {
        std_dev(&self.delta_v_samples)
    }

    /// Mean liftoff mass across all builds.
    pub fn mean_mass(&self) -> f64 {
        mean(&self.mass_samples)
    }

    /// Margin needed to achieve target delta-v at given confidence level.
    ///
    /// Returns the additional delta-v (above target, m/s) needed so that a
    /// fraction `confidence` (0.0 to 1.0) of builds reach the target.
    ///
    /// ```
    /// # use tsiolkovsky::prelude::*;
    /// # let problem = Problem::builder()
    /// #     .payload(Mass::tonnes(5.0))
    /// #     .target(Velocity::mps(9_400.0))
    /// #     .engine(EngineDatabase::builtin().get("raptor-2").unwrap().clone())
    /// #     .build()?;
    /// let results = MonteCarloRunner::new(Uncertainty::default())
    ///     .with_seed(1)
    ///     .run(&problem, 2_000)?;
    ///
    /// // How much margin for 95% confidence? A zero-margin design needs some.
    /// let margin = results.required_margin(0.95);
    /// assert!(margin > 0.0);
    /// println!("Need {margin:.0} m/s margin for 95% confidence");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn required_margin(&self, confidence: f64) -> f64 {
        if self.delta_v_samples.is_empty() {
            return 0.0;
        }
        // Find the percentile where we have (1 - confidence) failures
        let failure_percentile = (1.0 - confidence) * 100.0;
        let dv_at_percentile = self.delta_v_percentile(failure_percentile);
        (self.target_delta_v.as_mps() - dv_at_percentile).max(0.0)
    }

    /// Everything above, ready to serialize.
    pub(crate) fn summary(&self) -> MonteCarloSummary {
        MonteCarloSummary {
            success_probability: self.success_probability(),
            total_runs: self.total_runs,
            successes: self.successes,
            failures: self.failures,
            target_delta_v_mps: self.target_delta_v,
            runtime_ms: self.runtime.as_millis() as u64,
            delta_v: DistributionSummary::of(&self.delta_v_samples),
            mass: DistributionSummary::of(&self.mass_samples),
            required_margin_95_mps: self.required_margin(0.95),
            seed: self.seed,
            design_total_mass_kg: self.design.rocket().total_mass(),
            design_stage_count: self.design.rocket().stage_count(),
        }
    }
}

/// Results serialize as their statistics (not the raw samples): success
/// probability, run counts, delta-v and mass distributions, the margin
/// needed for 95% confidence, the seed, and the design that was stressed.
impl Serialize for MonteCarloResults {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.summary().serialize(serializer)
    }
}

fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn std_dev(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let m = mean(samples);
    let variance =
        samples.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;
    variance.sqrt()
}

/// A [`MonteCarloResults`] reduced to its statistics, for serialization.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MonteCarloSummary {
    /// Probability of achieving target delta-v (0.0 to 1.0)
    success_probability: f64,
    /// Builds evaluated
    total_runs: u64,
    /// Builds that reached the target and could lift off
    successes: u64,
    /// Builds too heavy to lift off
    failures: u64,
    target_delta_v_mps: Velocity,
    runtime_ms: u64,
    /// Delta-v distribution (m/s)
    delta_v: DistributionSummary,
    /// Liftoff mass distribution (kg)
    mass: DistributionSummary,
    /// Additional margin needed for 95% confidence (m/s)
    required_margin_95_mps: f64,
    /// Seed that reproduces this run
    seed: u64,
    /// Liftoff mass of the design that was stressed
    design_total_mass_kg: Mass,
    /// Stages in the design that was stressed
    design_stage_count: usize,
}

/// Summary statistics for a distribution.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DistributionSummary {
    mean: f64,
    std_dev: f64,
    /// 5th percentile (worst case)
    percentile_5: f64,
    /// 50th percentile (median)
    percentile_50: f64,
    /// 95th percentile (best case)
    percentile_95: f64,
    min: f64,
    max: f64,
}

impl DistributionSummary {
    fn of(samples: &[f64]) -> Self {
        Self {
            mean: mean(samples),
            std_dev: std_dev(samples),
            percentile_5: percentile_of(samples, 5.0),
            percentile_50: percentile_of(samples, 50.0),
            percentile_95: percentile_of(samples, 95.0),
            min: samples.iter().copied().fold(f64::INFINITY, f64::min),
            max: samples.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }
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
/// 3. Evaluate the delta-v and liftoff TWR of each build, at the gravity the
///    design was sized for.
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
/// come out below nominal. Adding margin
/// ([`Constraints::with_margin`](super::Constraints::with_margin)) is how a
/// design buys confidence.
///
/// # Example
///
/// ```
/// use tsiolkovsky::prelude::*;
///
/// let raptor = EngineDatabase::builtin().get("raptor-2").expect("engine");
/// let problem = Problem::builder()
///     .payload(Mass::tonnes(5.0))
///     .target(Velocity::mps(9_400.0))
///     .engine(raptor.clone())
///     .constraints(Constraints::default().with_margin(0.02))
///     .build()?;
///
/// let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(42);
/// let results = runner.run(&problem, 1000)?;
///
/// // 2% margin covers most, but not all, manufacturing variation
/// assert!(results.success_probability() > 0.9);
/// println!("Delta-v 5th percentile: {:.0} m/s", results.delta_v_percentile(5.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct MonteCarloRunner {
    uncertainty: Uncertainty,
    progress: Option<Arc<dyn Progress>>,
    seed: Option<u64>,
}

impl fmt::Debug for MonteCarloRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonteCarloRunner")
            .field("uncertainty", &self.uncertainty)
            .field("progress", &self.progress.is_some())
            .field("seed", &self.seed)
            .finish()
    }
}

impl MonteCarloRunner {
    /// Create a new Monte Carlo runner with the given uncertainty.
    pub fn new(uncertainty: Uncertainty) -> Self {
        Self {
            uncertainty,
            progress: None,
            seed: None,
        }
    }

    /// Report progress to `progress` as builds complete.
    pub fn with_progress(mut self, progress: impl Progress + 'static) -> Self {
        self.progress = Some(Arc::new(progress));
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
    /// Whatever the optimizer returns, or [`OptimizeError::Uncertainty`] for
    /// an unusable uncertainty.
    pub fn run(
        &self,
        problem: &Problem,
        iterations: u64,
    ) -> Result<MonteCarloResults, OptimizeError> {
        let nominal = AnalyticalOptimizer.optimize(problem)?;
        Ok(self.run_design(&nominal, iterations)?)
    }

    /// Stress an existing design: build it `iterations` times with
    /// perturbed parameters and evaluate each build against the design's
    /// target delta-v.
    ///
    /// # Errors
    ///
    /// [`UncertaintyError`] if a percentage is negative or not a number.
    pub fn run_design(
        &self,
        design: &Solution,
        iterations: u64,
    ) -> Result<MonteCarloResults, UncertaintyError> {
        self.uncertainty.validate()?;
        let start = Instant::now();
        let seed = self.seed.unwrap_or_else(rand::random);
        let target = design.target_delta_v();
        let nominal = design.rocket();

        // A build succeeds if it reaches the target (allowing the rounding
        // that a design sized exactly to it carries) and can leave the pad.
        let succeeds =
            |s: &Sample| s.lifts_off && s.delta_v >= target.as_mps() - DELTA_V_TOLERANCE_MPS;

        let results =
            |samples: Vec<Sample>, runs: u64, count: &dyn Fn(&Sample) -> u64| MonteCarloResults {
                successes: samples.iter().filter(|s| succeeds(s)).map(count).sum(),
                failures: samples.iter().filter(|s| !s.lifts_off).map(count).sum(),
                delta_v_samples: samples.iter().map(|s| s.delta_v).collect(),
                mass_samples: samples.iter().map(|s| s.mass).collect(),
                total_runs: runs,
                target_delta_v: target,
                runtime: start.elapsed(),
                design: design.clone(),
                seed,
            };

        if self.uncertainty.is_zero() {
            // Every build is the nominal design, so evaluate it once and
            // count it as many times as asked.
            let once = if iterations == 0 {
                vec![]
            } else {
                vec![Sample::of(nominal)]
            };
            return Ok(results(once, iterations, &|_| iterations));
        }

        let sampler = ParameterSampler::new(self.uncertainty);
        let completed = AtomicU64::new(0);
        let progress = self.progress.as_deref();
        if let Some(p) = progress {
            p.start("Monte Carlo", iterations);
        }

        let samples: Vec<Sample> = (0..iterations)
            .into_par_iter()
            .map(|i| {
                let mut rng = StdRng::seed_from_u64(sample_seed(seed, i));
                let sample = Sample::of(&sampler.perturb_rocket(nominal, &mut rng));
                if let Some(p) = progress {
                    p.advance(completed.fetch_add(1, Ordering::Relaxed) + 1);
                }
                sample
            })
            .collect();

        if let Some(p) = progress {
            p.finish();
        }
        Ok(results(samples, iterations, &|_| 1))
    }
}

/// One build of the design.
struct Sample {
    delta_v: f64,
    mass: f64,
    lifts_off: bool,
}

impl Sample {
    fn of(rocket: &crate::stage::Rocket) -> Self {
        Self {
            delta_v: rocket.total_delta_v().as_mps(),
            mass: rocket.total_mass().as_kg(),
            lifts_off: rocket.liftoff_twr().as_f64() > 1.0,
        }
    }
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
    use crate::engine::EngineDatabase;
    use crate::optimizer::Constraints;
    use crate::physics::IspModel;

    fn problem(constraints: Constraints) -> Problem {
        Problem::builder()
            .payload(Mass::kg(5_000.0))
            .target(Velocity::mps(9_400.0))
            .engine(EngineDatabase::builtin().get("raptor-2").unwrap().clone())
            .constraints(constraints)
            .stages(2)
            .build()
            .unwrap()
    }

    fn simple_problem() -> Problem {
        problem(Constraints::default())
    }

    #[test]
    fn zero_uncertainty_counts_every_build() {
        // Every build is the nominal design, which hits its target exactly:
        // all 10 succeed, and the count reports the 10 asked for.
        let results = MonteCarloRunner::new(Uncertainty::none())
            .run(&simple_problem(), 10)
            .unwrap();
        assert_eq!(results.successes(), 10);
        assert_eq!(results.total_runs(), 10);
        assert_eq!(results.success_probability(), 1.0);
    }

    #[test]
    fn zero_margin_design_succeeds_about_half_the_time() {
        // Half of all builds come out below nominal, so a design with no
        // margin should fail roughly half the time. v0.6 reported 100%.
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1);
        let results = runner.run(&simple_problem(), 2_000).unwrap();

        assert_eq!(results.total_runs(), 2_000);
        let p = results.success_probability();
        assert!((0.3..0.7).contains(&p), "success probability {p}");
        let mean = results.mean_delta_v();
        assert!((mean - 9_400.0).abs() < 50.0, "mean delta-v {mean}");
    }

    #[test]
    fn margin_buys_confidence() {
        let problem = problem(Constraints::default().with_margin(0.03));
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1);
        let results = runner.run(&problem, 2_000).unwrap();
        assert!(results.success_probability() > 0.95);
    }

    #[test]
    fn stresses_the_design_it_was_given() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let results = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(7)
            .run_design(&design, 200)
            .unwrap();
        let nominal = design.rocket().total_mass().as_kg();
        assert_eq!(results.design().rocket().total_mass().as_kg(), nominal);
        // Only structure is perturbed, so liftoff mass stays within a few
        // percent of nominal rather than being re-optimized each time.
        for &m in results.mass_samples() {
            assert!((m / nominal - 1.0).abs() < 0.05, "{m} vs {nominal}");
        }
    }

    #[test]
    fn seed_makes_runs_reproducible_across_thread_counts() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(42);
        let many = runner.run_design(&design, 500).unwrap();
        let one = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap()
            .install(|| runner.run_design(&design, 500).unwrap());

        assert_eq!(many.seed(), 42);
        assert_eq!(many.delta_v_samples(), one.delta_v_samples());
        assert_eq!(many.successes(), one.successes());
    }

    #[test]
    fn unseeded_runs_report_their_seed() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let first = MonteCarloRunner::new(Uncertainty::default())
            .run_design(&design, 100)
            .unwrap();
        let again = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(first.seed())
            .run_design(&design, 100)
            .unwrap();
        assert_eq!(first.delta_v_samples(), again.delta_v_samples());
    }

    #[test]
    fn invalid_uncertainty_is_an_error_not_a_panic() {
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        let bad = Uncertainty::default().with_isp_percent(-3.0);
        assert!(MonteCarloRunner::new(bad).run_design(&design, 10).is_err());
    }

    #[test]
    fn lunar_designs_are_judged_against_lunar_gravity() {
        // A Moon launch with liftoff TWR 1.3 at 1.62 m/s² has TWR 0.2 at g₀.
        // Monte Carlo used to judge it at g₀ and call every build a failure.
        let problem = Problem::builder()
            .payload(Mass::kg(300_000.0))
            .target(Velocity::mps(4_000.0))
            .engine(EngineDatabase::builtin().get("merlin-1d").unwrap().clone())
            .constraints(
                Constraints::default()
                    .with_surface_gravity(1.62)
                    .with_booster_isp(IspModel::Vacuum)
                    .with_max_engines(30)
                    .with_margin(0.03),
            )
            .build()
            .unwrap();
        let results = MonteCarloRunner::new(Uncertainty::default())
            .with_seed(3)
            .run(&problem, 500)
            .unwrap();
        assert_eq!(results.failures(), 0, "builds judged unable to lift off");
        assert!(results.success_probability() > 0.9);
    }

    #[test]
    fn progress_hears_every_build() {
        #[derive(Default)]
        struct Last(AtomicU64);
        impl Progress for Last {
            fn advance(&self, done: u64) {
                self.0.fetch_max(done, Ordering::Relaxed);
            }
        }
        let last = Arc::new(Last::default());
        struct Shared(Arc<Last>);
        impl Progress for Shared {
            fn advance(&self, done: u64) {
                self.0.advance(done);
            }
        }
        let design = AnalyticalOptimizer.optimize(&simple_problem()).unwrap();
        MonteCarloRunner::new(Uncertainty::default())
            .with_progress(Shared(last.clone()))
            .run_design(&design, 300)
            .unwrap();
        assert_eq!(last.0.load(Ordering::Relaxed), 300);
    }

    #[test]
    fn percentile_calculation() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!((percentile_of(&samples, 0.0) - 1.0).abs() < 0.1);
        assert!((percentile_of(&samples, 50.0) - 5.5).abs() < 1.0);
        assert!((percentile_of(&samples, 100.0) - 10.0).abs() < 0.1);
        assert_eq!(percentile_of(&[], 50.0), 0.0);
    }

    #[test]
    fn statistics() {
        let samples = [9400.0, 9500.0, 9600.0, 9700.0, 9800.0];
        assert!((mean(&samples) - 9600.0).abs() < 1e-9);
        assert!((std_dev(&samples) - 158.113_883).abs() < 1e-3);
        assert_eq!(std_dev(&[1.0]), 0.0);
    }
}
