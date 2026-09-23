//! Analytical staging optimizer: Lagrange multipliers, then refinement.
//!
//! The theory is documented on the public type below, where rustdoc shows it.

use std::time::Instant;

use rayon::prelude::*;

use crate::stage::{Rocket, Stage};
use crate::units::Mass;

use super::sizing::{infeasible, size_for_delta_v, Failure, SizedStage, StageEngine};
use super::solution::DELTA_V_TOLERANCE_MPS;
use super::{OptimizeError, Optimizer, Problem, Solution};

/// Largest number of engine-to-stage assignments searched per stage count.
const MAX_ASSIGNMENTS: usize = 200_000;

/// Scan points across each pairwise transfer before golden-section refinement.
const SCAN_POINTS: usize = 40;

/// Golden-section iterations per refinement.
const GOLDEN_ITERATIONS: usize = 48;

/// Sweeps over all stage pairs before giving up on further improvement.
const MAX_SWEEPS: usize = 40;

/// Staging optimizer based on the Lagrange multiplier solution.
///
/// # The staging problem
///
/// A rocket of N stages must deliver a total delta-v Δv to a payload, and we
/// want the lightest rocket that does it. The rocket equation gives each
/// stage's contribution, `Δvᵢ = cᵢ ln Rᵢ`, where cᵢ is its effective exhaust
/// velocity and Rᵢ its mass ratio. The question is how to split Δv between
/// the stages.
///
/// # The classical answer
///
/// Write εᵢ for each stage's structural coefficient: its dry mass as a
/// fraction of its dry-plus-propellant mass. If the εᵢ are constants, the
/// liftoff mass is a product of per-stage factors and the problem can be
/// solved exactly with a Lagrange multiplier η. Setting the gradient of
/// ln(liftoff mass) − η·(Σ cᵢ ln Rᵢ − Δv) to zero gives, for every stage,
///
/// ```text
///        cᵢη − 1
/// Rᵢ  =  ────────        with η fixed by   Σ cᵢ ln Rᵢ = Δv
///         cᵢεᵢη
/// ```
///
/// The left side of the constraint rises steadily with η, so a bisection
/// finds it. For identical stages (same c, same ε) every Rᵢ is equal and
/// **the optimum splits delta-v equally**. This is the textbook result, and
/// it is why the old tsi optimizer used a 50/50 split.
///
/// # Why that is not quite the answer
///
/// Real stages carry engines, and an engine's mass is fixed; it does not
/// scale with propellant load. A stage's ε therefore depends on how big the
/// stage is, which is what the classical solution assumes it does not. A
/// small upper stage with a heavy engine has a large ε. The fix is to move
/// delta-v off the upper stage and onto the booster, which is better at
/// amortizing its engines. Two further effects:
///
/// - An Earth-launched first stage flies through the atmosphere, so its
///   effective Isp is lower than an upper stage's using the same engine (see
///   [`IspModel`](crate::physics::IspModel)). That also moves the optimum.
/// - Engine counts are whole numbers, set by TWR (see the sizing notes on
///   the minimum engine count).
///
/// So this optimizer uses the Lagrange solution as its starting point, then
/// refines the split numerically against the exact mass model, including
/// engine mass and integer engine counts. For the README example
/// (5 t payload, 9,400 m/s, Raptor-2) the best split is about
/// 4,400/5,000 m/s. tsi v0.6 used the equal split, and a brute-force grid
/// search beat it by 11% on this very problem.
///
/// # Scope
///
/// Any number of stages and any set of engines. Every assignment of engines
/// to stages is tried, which grows as (engines)^(stages). Engines can be
/// pinned to particular stages to narrow the search.
///
/// # References
///
/// - Curtis, H.D. *Orbital Mechanics for Engineering Students*, chapter 11,
///   "Optimal staging"
/// - Sutton, G.P. and Biblarz, O. *Rocket Propulsion Elements*, chapter 4 ("Flight performance")
///
/// Fast (milliseconds for typical problems) and exact to within numerical
/// for the theory.
///
/// # When to Use
///
/// Almost always. It handles any stage count and any mix of engines. The
/// [`BruteForceOptimizer`](super::BruteForceOptimizer) searches a grid instead
/// and is useful as an independent cross-check.
///
/// # Example
///
/// ```
/// use tsiolkovsky::optimizer::{AnalyticalOptimizer, Problem, Constraints, Optimizer};
/// use tsiolkovsky::engine::EngineDatabase;
/// use tsiolkovsky::units::{Mass, Velocity};
///
/// let db = EngineDatabase::load_embedded().expect("failed to load database");
/// let raptor = db.get("raptor-2").expect("engine not found");
///
/// let problem = Problem::new(
///     Mass::kg(5_000.0),
///     Velocity::mps(9_400.0),
///     vec![raptor.clone()],
///     Constraints::default(),
/// ).with_stage_count(2);
///
/// let optimizer = AnalyticalOptimizer;
/// let solution = optimizer.optimize(&problem).expect("optimization failed");
///
/// assert!(solution.meets_target());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalyticalOptimizer;

/// The best design found for one engine assignment.
struct Design<'a> {
    stages: Vec<StageEngine<'a>>,
    sized: Vec<SizedStage>,
    total_mass: f64,
}

/// Everything needed to size a rocket for a given delta-v split.
struct Sizer<'a, 'p> {
    stages: &'a [StageEngine<'p>],
    payload: f64,
    tankage: f64,
    gravity: f64,
    max_engines: u32,
    /// Least delta-v stage 0 must deliver
    booster_floor: f64,
}

impl Sizer<'_, '_> {
    /// Size every stage, top down, for the given split (bottom-to-top order).
    fn size(&self, split: &[f64]) -> Result<Vec<SizedStage>, Failure> {
        let mut sized = Vec::with_capacity(split.len());
        let mut mass_above = self.payload;
        for (stage, &dv) in self.stages.iter().zip(split).rev() {
            if dv.is_nan() || dv <= 0.0 {
                return Err(Failure::StructuralLimit);
            }
            if sized.len() + 1 == split.len() && dv < self.booster_floor {
                return Err(Failure::BoosterTooSmall);
            }
            let s = size_for_delta_v(
                stage,
                dv,
                mass_above,
                self.tankage,
                self.gravity,
                self.max_engines,
            )?;
            mass_above = s.stack_mass;
            sized.push(s);
        }
        sized.reverse();
        Ok(sized)
    }

    fn total_mass(&self, split: &[f64]) -> Result<f64, Failure> {
        self.size(split).map(|s| s[0].stack_mass)
    }
}

/// Solve the classical staging problem (constant structural coefficients)
/// for the delta-v split. Returns `None` if the target is beyond the
/// structural limit of these stages.
///
/// `epsilon` is the textbook structural coefficient, dry / (dry + propellant).
fn lagrange_split(exhaust_velocities: &[f64], epsilon: f64, delta_v: f64) -> Option<Vec<f64>> {
    // Σ cᵢ ln Rᵢ as a function of the multiplier η
    let achieved = |eta: f64| -> f64 {
        exhaust_velocities
            .iter()
            .map(|&c| c * ((c * eta - 1.0) / (c * epsilon * eta)).ln())
            .sum()
    };

    // As η → ∞, Rᵢ → 1/ε: the most any stage can do with tanks that heavy.
    let ceiling: f64 = exhaust_velocities
        .iter()
        .map(|&c| c * (1.0 / epsilon).ln())
        .sum();
    if delta_v.is_nan() || delta_v >= ceiling {
        return None;
    }

    // Rᵢ > 0 requires η > 1/cᵢ for every stage.
    let floor = exhaust_velocities
        .iter()
        .map(|&c| 1.0 / c)
        .fold(0.0, f64::max);
    let mut lo = floor * (1.0 + 1e-12);
    let mut hi = floor * 2.0;
    while achieved(hi) < delta_v {
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if achieved(mid) < delta_v {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let eta = 0.5 * (lo + hi);

    let split: Vec<f64> = exhaust_velocities
        .iter()
        .map(|&c| c * ((c * eta - 1.0) / (c * epsilon * eta)).ln())
        .collect();

    // A stage whose exhaust is too slow to pull its weight gets a negative
    // share; the refinement will sort it out from a positive start.
    if split.iter().all(|&dv| dv > 0.0) {
        Some(split)
    } else {
        None
    }
}

/// Minimize liftoff mass over the delta-v split for one engine assignment.
///
/// Returns the best design and the number of rockets sized along the way.
fn optimize_assignment<'p>(
    stages: Vec<StageEngine<'p>>,
    problem: &Problem,
) -> (Result<Design<'p>, Failure>, u64) {
    let constraints = &problem.constraints;
    let tankage = constraints.structural_ratio.as_f64();
    let design_dv = problem.design_delta_v().as_mps();
    let sizer = Sizer {
        stages: &stages,
        payload: problem.payload.as_kg(),
        tankage,
        gravity: constraints.surface_gravity,
        max_engines: constraints.max_engines_per_stage,
        booster_floor: constraints.booster_delta_v_floor(stages.len()).as_mps(),
    };
    let n = stages.len();
    let mut evaluations = 0u64;
    let mut eval = |split: &[f64]| -> f64 {
        evaluations += 1;
        sizer.total_mass(split).unwrap_or(f64::INFINITY)
    };

    // No split can beat the structural ceiling: every stage at the largest
    // mass ratio its tanks allow, 1 + 1/ε, carrying nothing. Checking it
    // first gives a precise diagnosis instead of whatever the search tripped on.
    let exhaust: Vec<f64> = stages.iter().map(|s| s.exhaust_velocity).collect();
    let ceiling: f64 = exhaust.iter().map(|c| c * (1.0 + 1.0 / tankage).ln()).sum();
    if design_dv >= ceiling {
        return (Err(Failure::StructuralLimit), 0);
    }

    // Starting point: the classical solution, which ignores engine mass.
    let epsilon = tankage / (1.0 + tankage);
    let mut split = lagrange_split(&exhaust, epsilon, design_dv)
        .unwrap_or_else(|| vec![design_dv / n as f64; n]);
    // If the classical split leaves the first stage below its floor, start
    // from the floor instead, taking the difference evenly from the others.
    if n > 1 && split[0] < sizer.booster_floor {
        let shortfall = sizer.booster_floor * (1.0 + 1e-9) - split[0];
        split[0] += shortfall;
        for dv in &mut split[1..] {
            *dv -= shortfall / (n - 1) as f64;
        }
    }
    // If even the refined search finds nothing, the reason to report is the
    // one at this starting split: the scan also visits extreme splits, whose
    // failures describe those extremes rather than the problem.
    let start_failure = sizer.total_mass(&split).err();
    let mut best = eval(&split);

    // Refine by moving delta-v between each pair of stages in turn. Integer
    // engine counts make the mass a piecewise function of the split, so each
    // move is a coarse scan followed by golden-section search around the
    // best scan point.
    for _ in 0..MAX_SWEEPS {
        let mut improved = false;
        for i in 0..n {
            for j in (i + 1)..n {
                // Transfer t from stage j to stage i: t ∈ (−splitᵢ, splitⱼ)
                let lo = -split[i];
                let hi = split[j];
                let step = (hi - lo) / SCAN_POINTS as f64;
                let moved = |t: f64| {
                    let mut s = split.clone();
                    s[i] += t;
                    s[j] -= t;
                    s
                };

                let mut best_t = 0.0;
                let mut best_mass = best;
                for k in 0..SCAN_POINTS {
                    let t = lo + step * (k as f64 + 0.5);
                    let mass = eval(&moved(t));
                    if mass < best_mass {
                        best_mass = mass;
                        best_t = t;
                    }
                }

                // Golden-section search in the bracket around the best point
                let (mut a, mut b) = ((best_t - step).max(lo), (best_t + step).min(hi));
                const INV_PHI: f64 = 0.618_033_988_749_895;
                let mut x1 = b - INV_PHI * (b - a);
                let mut x2 = a + INV_PHI * (b - a);
                let mut f1 = eval(&moved(x1));
                let mut f2 = eval(&moved(x2));
                for _ in 0..GOLDEN_ITERATIONS {
                    if f1 < best_mass {
                        best_mass = f1;
                        best_t = x1;
                    }
                    if f2 < best_mass {
                        best_mass = f2;
                        best_t = x2;
                    }
                    if f1 <= f2 {
                        b = x2;
                        x2 = x1;
                        f2 = f1;
                        x1 = b - INV_PHI * (b - a);
                        f1 = eval(&moved(x1));
                    } else {
                        a = x1;
                        x1 = x2;
                        f1 = f2;
                        x2 = a + INV_PHI * (b - a);
                        f2 = eval(&moved(x2));
                    }
                }

                if best_mass < best * (1.0 - 1e-12) {
                    split = moved(best_t);
                    best = best_mass;
                    improved = true;
                }
            }
        }
        if !improved {
            break;
        }
    }

    if !best.is_finite() {
        return (
            Err(start_failure.unwrap_or(Failure::StructuralLimit)),
            evaluations,
        );
    }
    let sized = sizer
        .size(&split)
        .expect("the best split was sized successfully above");
    (
        Ok(Design {
            stages,
            sized,
            total_mass: best,
        }),
        evaluations,
    )
}

/// Every way of choosing one engine per stage, bottom to top.
fn assignments(candidates: &[Vec<&crate::engine::Engine>]) -> Vec<Vec<usize>> {
    let mut out = vec![vec![]];
    for options in candidates {
        out = out
            .into_iter()
            .flat_map(|prefix| {
                (0..options.len()).map(move |k| {
                    let mut next = prefix.clone();
                    next.push(k);
                    next
                })
            })
            .collect();
    }
    out
}

impl Optimizer for AnalyticalOptimizer {
    fn optimize(&self, problem: &Problem) -> Result<Solution, OptimizeError> {
        let start = Instant::now();
        problem.is_valid()?;

        let constraints = &problem.constraints;
        let (min_stages, max_stages) = problem.stage_count_range();
        let mut best: Option<Design> = None;
        let mut worst_failure = None;
        let mut evaluations = 0u64;
        let mut too_many = None;

        for stage_count in min_stages..=max_stages {
            let candidates: Vec<Vec<_>> = (0..stage_count as usize)
                .map(|i| problem.engines_for_stage(i))
                .collect();
            if candidates.iter().any(Vec::is_empty) {
                continue;
            }
            let combinations: usize = candidates.iter().map(Vec::len).product();
            if combinations > MAX_ASSIGNMENTS {
                // Every larger stage count is larger still, so stop here and
                // keep whatever the smaller counts found.
                too_many = Some((stage_count, combinations));
                break;
            }

            let results: Vec<_> = assignments(&candidates)
                .into_par_iter()
                .map(|choice| {
                    let stages = choice
                        .iter()
                        .enumerate()
                        .map(|(i, &k)| StageEngine::new(candidates[i][k], i, problem))
                        .collect();
                    optimize_assignment(stages, problem)
                })
                .collect();

            for (result, evals) in results {
                evaluations += evals;
                match result {
                    Ok(design) => {
                        if best
                            .as_ref()
                            .is_none_or(|b| design.total_mass < b.total_mass)
                        {
                            best = Some(design);
                        }
                    }
                    Err(f) => worst_failure = worst_failure.max(Some(f)),
                }
            }
        }

        let design = match (best, too_many) {
            (Some(design), _) => design,
            (None, Some((stage_count, combinations))) => {
                return Err(OptimizeError::Unsupported {
                    reason: format!(
                        "{stage_count} stages with these engines is {combinations} \
                        engine-to-stage combinations, more than the {MAX_ASSIGNMENTS} \
                        tsi will search; pin engines to stages or offer fewer engines"
                    ),
                })
            }
            (None, None) => {
                return Err(infeasible(
                    problem,
                    worst_failure.unwrap_or(Failure::StructuralLimit),
                ))
            }
        };

        let tankage = constraints.structural_ratio.as_f64();
        let stages = design
            .stages
            .iter()
            .zip(&design.sized)
            .map(|(s, sized)| {
                Stage::with_structural_ratio(
                    s.engine.clone(),
                    sized.engine_count,
                    Mass::kg(sized.propellant),
                    tankage,
                )
            })
            .collect();
        let rocket = Rocket::new(stages, problem.payload)
            .with_booster_isp(constraints.booster_isp)
            .with_surface_gravity(problem.constraints.surface_gravity);

        let solution = Solution::with_metadata(
            rocket,
            problem.target_delta_v,
            evaluations,
            start.elapsed(),
            "Analytical",
        );

        // The sizing model and the Rocket type compute the same physics two
        // different ways; if they ever disagree, fail loudly rather than
        // return a rocket that does not do what we claim.
        let achieved = solution.rocket.total_delta_v().as_mps();
        let design_dv = problem.design_delta_v().as_mps();
        if achieved < design_dv - DELTA_V_TOLERANCE_MPS {
            return Err(OptimizeError::Infeasible {
                reason: format!(
                    "Internal consistency check failed: sized for {design_dv:.3} m/s \
                    but the rocket delivers {achieved:.3} m/s"
                ),
            });
        }

        Ok(solution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, EngineDatabase, Propellant};
    use crate::optimizer::{BruteForceOptimizer, Constraints};
    use crate::physics::{IspModel, G0};
    use crate::units::{Force, Isp, Ratio, Velocity};

    fn get(name: &str) -> Engine {
        EngineDatabase::default().get(name).unwrap().clone()
    }

    fn two_stage(engines: Vec<Engine>, payload: f64, dv: f64) -> Problem {
        Problem::new(
            Mass::kg(payload),
            Velocity::mps(dv),
            engines,
            Constraints::default(),
        )
        .with_stage_count(2)
    }

    #[test]
    fn lagrange_splits_identical_stages_equally() {
        let c = 350.0 * G0;
        let split = lagrange_split(&[c, c, c], 0.08 / 1.08, 9_000.0).unwrap();
        for dv in &split {
            assert!((dv - 3_000.0).abs() < 1e-6, "{split:?}");
        }
    }

    #[test]
    fn lagrange_gives_more_delta_v_to_the_better_stage() {
        // Hydrogen upper stage over a kerosene booster
        let split = lagrange_split(&[300.0 * G0, 450.0 * G0], 0.07, 9_400.0).unwrap();
        assert!(split[1] > split[0], "{split:?}");
        assert!((split.iter().sum::<f64>() - 9_400.0).abs() < 1e-6);
    }

    #[test]
    fn lagrange_rejects_targets_past_the_structural_limit() {
        // ε = 0.2 caps each stage at c·ln 5 ≈ 5,500 m/s for a 350 s engine
        assert!(lagrange_split(&[350.0 * G0; 2], 0.2, 12_000.0).is_none());
    }

    #[test]
    fn equal_split_when_engine_mass_is_negligible() {
        // With a feather-light engine in vacuum, the classical theory holds
        // exactly: identical stages split delta-v equally.
        let feather = Engine::new(
            "Feather",
            Force::kilonewtons(2_000.0),
            Force::kilonewtons(2_000.0),
            Isp::seconds(350.0),
            Isp::seconds(350.0),
            Mass::kg(0.001),
            Propellant::LoxCh4,
        );
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![feather],
            Constraints::default()
                .with_booster_isp(IspModel::Vacuum)
                .with_max_engines(100),
        )
        .with_stage_count(2);

        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        let s1 = solution.rocket.stage_delta_v(0).as_mps();
        let s2 = solution.rocket.stage_delta_v(1).as_mps();
        assert!((s1 - s2).abs() < 1.0, "S1={s1:.1} S2={s2:.1}");
    }

    #[test]
    fn hits_target_exactly_with_zero_margin() {
        let problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert!(solution.meets_target());
        assert!(solution.margin.as_mps().abs() < 0.01, "{}", solution.margin);
    }

    #[test]
    fn honours_requested_margin() {
        let mut problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0);
        problem.constraints.margin = Ratio::new(0.02);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert!((solution.margin.as_mps() - 188.0).abs() < 0.01);
    }

    #[test]
    fn readme_example_beats_brute_force() {
        // The v0.6 optimizer returned 205.4 t here; brute force found 182.8 t.
        let problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0);
        let analytical = AnalyticalOptimizer.optimize(&problem).unwrap();
        let brute = BruteForceOptimizer::default()
            .with_progress(false)
            .optimize(&problem)
            .unwrap();
        let a = analytical.rocket.total_mass().as_kg();
        let b = brute.rocket.total_mass().as_kg();
        assert!(
            a <= b * 1.001,
            "analytical {a:.0} kg vs brute force {b:.0} kg"
        );
    }

    #[test]
    fn uses_fewest_engines_that_meet_twr() {
        let problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        let rocket = &solution.rocket;
        let s1 = &rocket.stages()[0];
        // One engine fewer would fail the liftoff TWR (or leave none at all).
        let without_one = (s1.engine_count() - 1) as f64 * s1.engine().thrust_sl().as_newtons()
            / ((rocket.total_mass().as_kg() - s1.engine().dry_mass().as_kg()) * G0);
        assert!(s1.engine_count() == 1 || without_one < 1.2);
        assert!(rocket.liftoff_twr().as_f64() >= 1.2);
    }

    #[test]
    fn optimizes_stage_count_when_not_fixed() {
        // 3,000 m/s is easy for a single stage; a second stage is dead weight.
        let problem = Problem::new(
            Mass::kg(1_000.0),
            Velocity::mps(3_000.0),
            vec![get("raptor-2")],
            Constraints::default(),
        );
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert_eq!(solution.rocket.stage_count(), 1);
    }

    #[test]
    fn three_stages() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(12_000.0),
            vec![get("raptor-2")],
            Constraints::default(),
        )
        .with_stage_count(3);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert_eq!(solution.rocket.stage_count(), 3);
        assert!(solution.meets_target());
    }

    #[test]
    fn picks_hydrogen_for_the_upper_stage() {
        let problem = two_stage(vec![get("merlin-1d"), get("rl-10c")], 5_000.0, 9_400.0);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        let stages = solution.rocket.stages();
        assert_eq!(stages[0].engine().name, "Merlin-1D");
        assert_eq!(stages[1].engine().name, "RL-10C");
    }

    #[test]
    fn pinned_engine_is_respected() {
        let problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0)
            .with_pinned_engine(0, get("merlin-1d"));
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert_eq!(solution.rocket.stages()[0].engine().name, "Merlin-1D");
        assert_eq!(solution.rocket.stages()[1].engine().name, "Raptor-2");
    }

    #[test]
    fn respects_twr_constraints() {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_000.0),
            vec![get("raptor-2")],
            Constraints::new(Ratio::new(1.3), Ratio::new(0.7), 3, Ratio::new(0.08)),
        )
        .with_stage_count(2);
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert!(solution.rocket.liftoff_twr().as_f64() >= 1.3);
        assert!(solution.rocket.stage_twr(1).as_f64() >= 0.7);
    }

    #[test]
    fn infeasible_delta_v_is_reported() {
        let problem = two_stage(vec![get("raptor-2")], 100_000.0, 50_000.0);
        let result = AnalyticalOptimizer.optimize(&problem);
        assert!(matches!(result, Err(OptimizeError::Infeasible { .. })));
    }

    #[test]
    fn too_many_combinations_keeps_smaller_stage_counts() {
        // 60 engines: 2 stages is 3,600 combinations, 3 stages is 216,000,
        // over the cap. The 2-stage answer must survive the 3-stage refusal.
        let base = get("raptor-2");
        let engines: Vec<Engine> = (0..60)
            .map(|i| {
                Engine::new(
                    format!("Raptor-{i}"),
                    base.thrust_sl() * (1.0 + i as f64 * 1e-3),
                    base.thrust_vac(),
                    base.isp_sl(),
                    base.isp_vac(),
                    base.dry_mass(),
                    base.propellant,
                )
            })
            .collect();
        let mut problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_400.0),
            engines,
            Constraints::default(),
        );
        problem.constraints.max_stages = 3;
        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        assert_eq!(solution.rocket.stage_count(), 2);
    }

    #[test]
    fn upper_stage_twr_failure_names_the_upper_stage_flag() {
        let mut problem = two_stage(vec![get("raptor-2")], 5_000.0, 9_400.0);
        problem.constraints.min_stage_twr = Ratio::new(200.0);
        match AnalyticalOptimizer.optimize(&problem) {
            Err(OptimizeError::Infeasible { reason }) => {
                assert!(reason.contains("--min-upper-twr"), "{reason}");
                assert!(reason.contains("stage 2"), "{reason}");
            }
            other => panic!("expected infeasible, got {other:?}"),
        }
    }

    #[test]
    fn engine_limit_is_reported() {
        let mut problem = two_stage(vec![get("rutherford")], 50_000.0, 9_000.0);
        problem.constraints.max_engines_per_stage = 2;
        match AnalyticalOptimizer.optimize(&problem) {
            Err(OptimizeError::Infeasible { reason }) => {
                assert!(reason.contains("--max-engines"), "{reason}")
            }
            other => panic!("expected infeasible, got {other:?}"),
        }
    }
}
