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

use super::Problem;

/// Relative slack so that a rocket sized exactly to a TWR bound still
/// passes the check after floating-point rounding.
const TWR_SLACK: f64 = 1e-9;

/// Why a candidate stage (or rocket) could not be sized.
///
/// Ordered from least to most informative. A search that fails everywhere
/// reports the greatest failure it saw: running out of engines says more
/// about what to change than a scan point that strayed past a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Failure {
    /// The requested mass ratio is beyond what the tankage ratio allows.
    StructuralLimit,
    /// The first stage would hand over to an upper stage too low in the
    /// atmosphere (see [`Constraints::min_booster_delta_v`](super::Constraints::min_booster_delta_v)).
    BoosterTooSmall,
    /// Engines are too heavy for their thrust: no count meets the TWR.
    EnginesTooHeavy,
    /// The TWR needs more engines than a stage may carry.
    EngineLimit,
}

/// One stage's engine, reduced to the numbers sizing needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StageEngine<'a> {
    pub engine: &'a Engine,
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
        let thrust = match model {
            IspModel::AscentAveraged => engine.thrust_sl(),
            IspModel::Vacuum => engine.thrust_vac(),
        };
        Self {
            engine,
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
            return Err(Failure::EnginesTooHeavy);
        }
        let needed = (k * fixed_mass / net_thrust * (1.0 + TWR_SLACK)).ceil();
        if needed.is_nan() || needed > f64::from(max_engines) {
            return Err(Failure::EngineLimit);
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
        assert_eq!(result.unwrap_err(), Failure::EngineLimit);
    }
}
