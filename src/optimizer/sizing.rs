//! Stage sizing shared by the optimizers.
//!
//! Both optimizers build rockets from the top down: the payload is known, so
//! the top stage can be sized, then the stage below it (whose payload is the
//! top stage plus the real payload), and so on to the ground.
//!
//! Two results here do most of the work.
//!
//! # Mass growth per stage
//!
//! A stage with tankage ratio ε (structure / propellant), engine mass E, and
//! mass ratio R that carries a mass `m` above it has propellant
//!
//! ```text
//!         (m + E)(R − 1)
//! m_p  =  ──────────────
//!          1 − ε(R − 1)
//! ```
//!
//! and everything from its engines upward, fully loaded, weighs
//!
//! ```text
//!                       R
//! m + stage = (m + E) · ──────────── = (m + E) · G(R)
//!                       1 − ε(R − 1)
//! ```
//!
//! G(R) is the stage's *growth factor*. It goes to infinity as R approaches
//! 1 + 1/ε. That is the structural limit: no amount of propellant can give a
//! stage more mass ratio than its own tanks allow.
//!
//! # The fewest engines that satisfy TWR
//!
//! Adding an engine adds thrust F and mass mₑ. The stage needs
//! n·F ≥ TWR·g·(m + n·mₑ)·G, which gives
//!
//! ```text
//!          TWR·g·G·m
//! n  ≥  ───────────────
//!       F − TWR·g·G·mₑ
//! ```
//!
//! If the denominator is zero or negative, each extra engine adds more weight
//! than thrust and no number of them will do. Extra engines only add dry mass
//! and so cost delta-v, which means the smallest n that meets the bound is
//! always the best choice. The optimizers never need to search engine counts.

use crate::engine::Engine;
use crate::physics::{IspModel, G0};

use super::{OptimizeError, Problem};

/// Relative tolerance on the TWR bound. A stage may come out up to one part
/// in a billion under its minimum TWR rather than carry an extra engine
/// because of floating-point rounding.
pub(crate) const TWR_SLACK: f64 = 1e-9;

/// Why a candidate stage (or rocket) could not be sized.
///
/// Ordered from least to most actionable, for choosing between the failures
/// of different engine assignments: running out of engines says more about
/// what to change than a structural limit that more stages would lift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Failure {
    /// The requested mass ratio is beyond what the tankage ratio allows.
    StructuralLimit,
    /// The first stage would hand over to an upper stage too low in the
    /// atmosphere (see [`Constraints::min_booster_delta_v`](super::Constraints::min_booster_delta_v)).
    BoosterTooSmall,
    /// Engines on this stage are too heavy for their thrust: no count meets the TWR.
    EnginesTooHeavy { stage: usize },
    /// The TWR on this stage needs more engines than a stage may carry.
    EngineLimit { stage: usize },
}

/// One stage's engine, reduced to the numbers sizing needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StageEngine<'a> {
    pub engine: &'a Engine,
    /// Stage index, 0 = first stage
    pub index: usize,
    /// Effective exhaust velocity c = Isp × g₀ (m/s)
    pub exhaust_velocity: f64,
    /// Thrust per engine used for the TWR check (N)
    pub thrust: f64,
    /// Dry mass per engine (kg)
    pub engine_mass: f64,
    /// Minimum TWR at this stage's ignition
    pub min_twr: f64,
}

impl<'a> StageEngine<'a> {
    /// Prepare `engine` for use on stage `index` of `problem`.
    ///
    /// The first stage uses the problem's booster Isp model and, when that
    /// model is ascent-averaged, sea-level thrust. Every stage above uses
    /// vacuum values.
    pub fn new(engine: &'a Engine, index: usize, problem: &Problem) -> Self {
        let constraints = &problem.constraints;
        let (model, min_twr) = if index == 0 {
            (constraints.booster_isp, constraints.min_liftoff_twr)
        } else {
            (IspModel::Vacuum, constraints.min_stage_twr)
        };
        let thrust = engine.thrust_for(model);
        Self {
            engine,
            index,
            exhaust_velocity: engine.isp_for(model).as_seconds() * G0,
            thrust: thrust.as_newtons(),
            engine_mass: engine.dry_mass().as_kg(),
            min_twr: min_twr.as_f64(),
        }
    }

    /// The fewest engines that give this stage its minimum TWR.
    ///
    /// `fixed_mass` is everything the engines must lift except themselves,
    /// before the growth factor `growth` is applied.
    pub fn min_engine_count(
        &self,
        fixed_mass: f64,
        growth: f64,
        gravity: f64,
        max_engines: u32,
    ) -> Result<u32, Failure> {
        let k = self.min_twr * gravity * growth;
        let net_thrust = self.thrust - k * self.engine_mass;
        if net_thrust.is_nan() || net_thrust <= 0.0 {
            return Err(Failure::EnginesTooHeavy { stage: self.index });
        }
        // Shave a hair off before rounding up, so a requirement that lands a
        // rounding error above a whole number doesn't cost a whole engine.
        let needed = (k * fixed_mass / net_thrust * (1.0 - TWR_SLACK)).ceil();
        if needed.is_nan() || needed > f64::from(max_engines) {
            return Err(Failure::EngineLimit { stage: self.index });
        }
        Ok((needed as u32).max(1))
    }
}

/// The result of sizing one stage.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SizedStage {
    pub engine_count: u32,
    pub propellant: f64,
    /// Mass of this stage plus everything above it, fully loaded (kg)
    pub stack_mass: f64,
}

/// Size a stage to deliver `delta_v` while carrying `mass_above`.
pub(crate) fn size_for_delta_v(
    stage: &StageEngine<'_>,
    delta_v: f64,
    mass_above: f64,
    tankage: f64,
    gravity: f64,
    max_engines: u32,
) -> Result<SizedStage, Failure> {
    let r = (delta_v / stage.exhaust_velocity).exp();
    let denominator = 1.0 - tankage * (r - 1.0);
    if denominator.is_nan() || denominator <= 0.0 {
        return Err(Failure::StructuralLimit);
    }
    let growth = r / denominator;
    let engine_count = stage.min_engine_count(mass_above, growth, gravity, max_engines)?;
    let fixed = mass_above + engine_count as f64 * stage.engine_mass;
    Ok(SizedStage {
        engine_count,
        propellant: fixed * (r - 1.0) / denominator,
        stack_mass: fixed * growth,
    })
}

/// Size a stage that carries a given propellant load, returning the sized
/// stage and the delta-v it delivers.
pub(crate) fn size_for_propellant(
    stage: &StageEngine<'_>,
    propellant: f64,
    mass_above: f64,
    tankage: f64,
    gravity: f64,
    max_engines: u32,
) -> Result<(SizedStage, f64), Failure> {
    let structure = tankage * propellant;
    let fixed = mass_above + propellant + structure;
    let engine_count = stage.min_engine_count(fixed, 1.0, gravity, max_engines)?;
    let engines = engine_count as f64 * stage.engine_mass;
    let wet = fixed + engines;
    let dry = wet - propellant;
    Ok((
        SizedStage {
            engine_count,
            propellant,
            stack_mass: wet,
        },
        stage.exhaust_velocity * (wet / dry).ln(),
    ))
}

/// Turn a sizing failure into an error that says what to change.
pub(crate) fn infeasible(problem: &Problem, failure: Failure) -> OptimizeError {
    let c = &problem.constraints;
    let (_, max_stages) = problem.stage_count_range();
    let reason = match failure {
        Failure::StructuralLimit => format!(
            "Structural ratio {:.0}% is too high to reach {:.0} m/s with up to {} stages.\n\n\
            Suggestions:\n  \
            - Lower --structural-ratio (currently {:.0}%)\n  \
            - Allow more stages with --max-stages\n  \
            - Use an engine with higher ISP\n  \
            - Reduce target delta-v",
            c.structural_ratio.as_f64() * 100.0,
            problem.design_delta_v().as_mps(),
            max_stages,
            c.structural_ratio.as_f64() * 100.0,
        ),
        Failure::EngineLimit { stage } => {
            let (flag, twr) = twr_limit(problem, stage);
            format!(
                "Reaching TWR {twr:.2} on stage {} needs more than {} engines.\n\n\
                Suggestions:\n  \
                - Allow more engines with --max-engines\n  \
                - Lower {flag} (currently {twr:.2})\n  \
                - Use an engine with higher thrust",
                stage + 1,
                c.max_engines_per_stage,
            )
        }
        Failure::BoosterTooSmall => format!(
            "The first stage can't deliver the {:.0} m/s it needs to carry an upper \
            stage above the atmosphere.\n\n\
            Suggestions:\n  \
            - Increase target delta-v, or use a single stage\n  \
            - Use a first-stage engine with more thrust",
            c.min_booster_delta_v.as_mps(),
        ),
        Failure::EnginesTooHeavy { stage } => {
            let (flag, twr) = twr_limit(problem, stage);
            format!(
                "No number of engines can give stage {} a TWR of {twr:.2}: each engine \
                weighs too much for its thrust at {:.2} m/s².\n\n\
                Suggestions:\n  \
                - Lower {flag} (currently {twr:.2})\n  \
                - Use an engine with a better thrust-to-weight ratio",
                stage + 1,
                c.surface_gravity,
            )
        }
    };
    OptimizeError::Infeasible { reason }
}

/// The CLI flag and value of the TWR limit that applies to a stage.
fn twr_limit(problem: &Problem, stage: usize) -> (&'static str, f64) {
    if stage == 0 {
        ("--min-twr", problem.constraints.min_liftoff_twr.as_f64())
    } else {
        (
            "--min-upper-twr",
            problem.constraints.min_stage_twr.as_f64(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::optimizer::Constraints;
    use crate::stage::{Rocket, Stage};
    use crate::units::{Mass, Velocity};

    fn problem() -> Problem {
        let db = EngineDatabase::default();
        Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(9_400.0),
            vec![db.get("raptor-2").unwrap().clone()],
            Constraints::default(),
        )
    }

    #[test]
    fn sizing_matches_rocket_physics() {
        // Sizing for a delta-v and then building the Stage must give back
        // exactly that delta-v and a TWR at the bound or above.
        let problem = problem();
        let engine = &problem.available_engines[0];
        let upper = StageEngine::new(engine, 1, &problem);
        let booster = StageEngine::new(engine, 0, &problem);

        let s2 = size_for_delta_v(&upper, 5_000.0, 5_000.0, 0.08, G0, 9).unwrap();
        let s1 = size_for_delta_v(&booster, 4_400.0, s2.stack_mass, 0.08, G0, 9).unwrap();

        let rocket = Rocket::new(
            vec![
                Stage::with_structural_ratio(
                    engine.clone(),
                    s1.engine_count,
                    Mass::kg(s1.propellant),
                    0.08,
                ),
                Stage::with_structural_ratio(
                    engine.clone(),
                    s2.engine_count,
                    Mass::kg(s2.propellant),
                    0.08,
                ),
            ],
            Mass::kg(5_000.0),
        );

        assert!((rocket.stage_delta_v(1).as_mps() - 5_000.0).abs() < 1e-6);
        assert!((rocket.stage_delta_v(0).as_mps() - 4_400.0).abs() < 1e-6);
        assert!((rocket.total_mass().as_kg() - s1.stack_mass).abs() < 1e-6);
        assert!(rocket.liftoff_twr().as_f64() >= 1.2);
        assert!(rocket.stage_twr(1).as_f64() >= 0.5);
    }

    #[test]
    fn min_engine_count_is_minimal() {
        let problem = problem();
        let engine = &problem.available_engines[0];
        let booster = StageEngine::new(engine, 0, &problem);
        let s = size_for_delta_v(&booster, 4_000.0, 50_000.0, 0.08, G0, 40).unwrap();
        // One engine fewer must fail the TWR bound.
        let fewer = s.engine_count - 1;
        let mass = s.stack_mass - booster.engine_mass;
        let twr = fewer as f64 * booster.thrust / (mass * G0);
        assert!(fewer == 0 || twr < 1.2, "twr with one fewer engine: {twr}");
    }

    #[test]
    fn requirement_just_over_a_whole_number_is_not_rounded_up() {
        let problem = problem();
        let booster = StageEngine::new(&problem.available_engines[0], 0, &problem);
        // Choose the fixed mass so that exactly 3 engines meet the TWR, then
        // nudge it up by a rounding error.
        let k = booster.min_twr * G0;
        let exact = 3.0 * (booster.thrust - k * booster.engine_mass) / k;
        let n = booster
            .min_engine_count(exact * (1.0 + 1e-12), 1.0, G0, 9)
            .unwrap();
        assert_eq!(n, 3);
    }

    #[test]
    fn structural_limit_detected() {
        let problem = problem();
        let upper = StageEngine::new(&problem.available_engines[0], 1, &problem);
        // ε = 0.25 caps the mass ratio at 5: ln 5 × 3,432 m/s ≈ 5,500 m/s.
        let result = size_for_delta_v(&upper, 6_000.0, 1_000.0, 0.25, G0, 9);
        assert_eq!(result.unwrap_err(), Failure::StructuralLimit);
    }

    #[test]
    fn engine_limit_detected() {
        let problem = problem();
        let booster = StageEngine::new(&problem.available_engines[0], 0, &problem);
        let result = size_for_delta_v(&booster, 4_000.0, 5_000_000.0, 0.08, G0, 9);
        assert_eq!(result.unwrap_err(), Failure::EngineLimit { stage: 0 });
    }
}
