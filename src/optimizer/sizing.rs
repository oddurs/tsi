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
use crate::stage::{Rocket, Stage};
use crate::units::Mass;

use super::{Infeasibility, Problem};

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

/// One stage's engine and constraints, reduced to the numbers sizing needs.
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
    /// Structural ratio: structure (engines excluded) / propellant
    pub tankage: f64,
    /// Surface gravity for the TWR check (m/s²)
    pub gravity: f64,
    /// Most engines this stage may carry
    pub max_engines: u32,
}

impl<'a> StageEngine<'a> {
    /// Prepare `engine` for use on stage `index` of `problem`.
    ///
    /// The first stage uses the problem's booster Isp model and, when that
    /// model is ascent-averaged, sea-level thrust. Every stage above uses
    /// vacuum values.
    pub fn new(engine: &'a Engine, index: usize, problem: &Problem) -> Self {
        let c = problem.constraints();
        let (model, min_twr) = if index == 0 {
            (c.booster_isp(), c.min_liftoff_twr())
        } else {
            (IspModel::Vacuum, c.min_stage_twr())
        };
        Self {
            engine,
            index,
            exhaust_velocity: engine.isp_for(model).as_seconds() * G0,
            thrust: engine.thrust_for(model).as_newtons(),
            engine_mass: engine.dry_mass().as_kg(),
            min_twr: min_twr.as_f64(),
            tankage: c.structural_ratio(index).as_f64(),
            gravity: c.surface_gravity(),
            max_engines: c.max_engines_per_stage(),
        }
    }

    /// The fewest engines that give this stage its minimum TWR.
    ///
    /// `fixed_mass` is everything the engines must lift except themselves,
    /// before the growth factor `growth` is applied.
    pub fn min_engine_count(&self, fixed_mass: f64, growth: f64) -> Result<u32, Failure> {
        let k = self.min_twr * self.gravity * growth;
        let net_thrust = self.thrust - k * self.engine_mass;
        if net_thrust.is_nan() || net_thrust <= 0.0 {
            return Err(Failure::EnginesTooHeavy { stage: self.index });
        }
        // Shave a hair off before rounding up, so a requirement that lands a
        // rounding error above a whole number doesn't cost a whole engine.
        let needed = (k * fixed_mass / net_thrust * (1.0 - TWR_SLACK)).ceil();
        if needed.is_nan() || needed > f64::from(self.max_engines) {
            return Err(Failure::EngineLimit { stage: self.index });
        }
        Ok((needed as u32).max(1))
    }

    /// The most delta-v this stage could give, carrying nothing: the mass
    /// ratio its tanks cap it at, 1 + 1/ε.
    pub fn structural_ceiling(&self) -> f64 {
        self.exhaust_velocity * (1.0 + 1.0 / self.tankage).ln()
    }

    /// Size this stage to deliver `delta_v` while carrying `mass_above`.
    pub fn size_for_delta_v(&self, delta_v: f64, mass_above: f64) -> Result<SizedStage, Failure> {
        let r = (delta_v / self.exhaust_velocity).exp();
        let denominator = 1.0 - self.tankage * (r - 1.0);
        if denominator.is_nan() || denominator <= 0.0 {
            return Err(Failure::StructuralLimit);
        }
        let growth = r / denominator;
        let engine_count = self.min_engine_count(mass_above, growth)?;
        let fixed = mass_above + engine_count as f64 * self.engine_mass;
        Ok(SizedStage {
            engine_count,
            propellant: fixed * (r - 1.0) / denominator,
            stack_mass: fixed * growth,
        })
    }

    /// Size this stage around a given propellant load, returning the sized
    /// stage and the delta-v it delivers.
    pub fn size_for_propellant(
        &self,
        propellant: f64,
        mass_above: f64,
    ) -> Result<(SizedStage, f64), Failure> {
        let fixed = mass_above + propellant * (1.0 + self.tankage);
        let engine_count = self.min_engine_count(fixed, 1.0)?;
        let wet = fixed + engine_count as f64 * self.engine_mass;
        let dry = wet - propellant;
        Ok((
            SizedStage {
                engine_count,
                propellant,
                stack_mass: wet,
            },
            self.exhaust_velocity * (wet / dry).ln(),
        ))
    }

    /// Build the [`Stage`] this sizing describes.
    pub fn build(&self, sized: &SizedStage) -> Stage {
        Stage::from_parts(
            self.engine.clone(),
            sized.engine_count,
            Mass::kg(sized.propellant),
            Mass::kg(sized.propellant * self.tankage),
        )
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

/// Assemble sized stages into the rocket for `problem`.
pub(crate) fn assemble(problem: &Problem, stages: Vec<Stage>) -> Rocket {
    Rocket::from_parts(stages, problem.payload())
        .with_booster_isp(problem.constraints().booster_isp())
        .with_surface_gravity(problem.constraints().surface_gravity())
}

/// Describe a sizing failure as the constraint that binds.
pub(crate) fn infeasibility(problem: &Problem, failure: Failure) -> Infeasibility {
    let c = problem.constraints();
    let required_twr = |stage: usize| {
        if stage == 0 {
            c.min_liftoff_twr()
        } else {
            c.min_stage_twr()
        }
    };
    match failure {
        Failure::StructuralLimit => Infeasibility::StructuralLimit {
            target: problem.design_delta_v(),
            max_stages: problem.stage_count_range().1,
        },
        Failure::BoosterTooSmall => Infeasibility::BoosterTooSmall {
            floor: c.min_booster_delta_v(),
        },
        Failure::EngineLimit { stage } => Infeasibility::EngineLimit {
            stage,
            required_twr: required_twr(stage),
            max_engines: c.max_engines_per_stage(),
        },
        Failure::EnginesTooHeavy { stage } => Infeasibility::EnginesTooHeavy {
            stage,
            required_twr: required_twr(stage),
            gravity: c.surface_gravity(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::optimizer::Constraints;
    use crate::units::Velocity;

    fn problem(constraints: Constraints) -> Problem {
        Problem::builder()
            .payload(Mass::kg(5_000.0))
            .target(Velocity::mps(9_400.0))
            .engine(EngineDatabase::builtin().get("raptor-2").unwrap().clone())
            .constraints(constraints)
            .build()
            .unwrap()
    }

    #[test]
    fn sizing_matches_rocket_physics() {
        // Sizing for a delta-v and then building the stages must give back
        // exactly that delta-v and a TWR at the bound or above.
        let problem = problem(Constraints::default());
        let engine = &problem.engines()[0];
        let upper = StageEngine::new(engine, 1, &problem);
        let booster = StageEngine::new(engine, 0, &problem);

        let s2 = upper.size_for_delta_v(5_000.0, 5_000.0).unwrap();
        let s1 = booster.size_for_delta_v(4_400.0, s2.stack_mass).unwrap();
        let rocket = assemble(&problem, vec![booster.build(&s1), upper.build(&s2)]);

        assert!((rocket.stage_delta_v(1).as_mps() - 5_000.0).abs() < 1e-6);
        assert!((rocket.stage_delta_v(0).as_mps() - 4_400.0).abs() < 1e-6);
        assert!((rocket.total_mass().as_kg() - s1.stack_mass).abs() < 1e-6);
        assert!(rocket.liftoff_twr().as_f64() >= 1.2 * (1.0 - TWR_SLACK));
        assert!(rocket.stage_twr(1).as_f64() >= 0.5 * (1.0 - TWR_SLACK));
    }

    #[test]
    fn per_stage_tankage_is_used() {
        let problem = problem(Constraints::default().with_structural_ratios([0.04, 0.11]));
        let engine = &problem.engines()[0];
        assert_eq!(StageEngine::new(engine, 0, &problem).tankage, 0.04);
        assert_eq!(StageEngine::new(engine, 1, &problem).tankage, 0.11);
    }

    #[test]
    fn min_engine_count_is_minimal() {
        let problem = problem(Constraints::default().with_max_engines(40));
        let booster = StageEngine::new(&problem.engines()[0], 0, &problem);
        let s = booster.size_for_delta_v(4_000.0, 50_000.0).unwrap();
        // One engine fewer must fail the TWR bound.
        let fewer = s.engine_count - 1;
        let mass = s.stack_mass - booster.engine_mass;
        let twr = fewer as f64 * booster.thrust / (mass * G0);
        assert!(fewer == 0 || twr < 1.2, "twr with one fewer engine: {twr}");
    }

    #[test]
    fn requirement_just_over_a_whole_number_is_not_rounded_up() {
        let problem = problem(Constraints::default());
        let booster = StageEngine::new(&problem.engines()[0], 0, &problem);
        // Choose the fixed mass so that exactly 3 engines meet the TWR, then
        // nudge it up by a rounding error.
        let k = booster.min_twr * G0;
        let exact = 3.0 * (booster.thrust - k * booster.engine_mass) / k;
        let n = booster
            .min_engine_count(exact * (1.0 + 1e-12), 1.0)
            .unwrap();
        assert_eq!(n, 3);
    }

    #[test]
    fn structural_limit_detected() {
        // ε = 0.25 caps the mass ratio at 5: ln 5 × 3,432 m/s ≈ 5,500 m/s.
        let problem = problem(Constraints::default().with_structural_ratio(0.25));
        let upper = StageEngine::new(&problem.engines()[0], 1, &problem);
        let result = upper.size_for_delta_v(6_000.0, 1_000.0);
        assert_eq!(result.unwrap_err(), Failure::StructuralLimit);
    }

    #[test]
    fn engine_limit_detected() {
        let problem = problem(Constraints::default());
        let booster = StageEngine::new(&problem.engines()[0], 0, &problem);
        let result = booster.size_for_delta_v(4_000.0, 5_000_000.0);
        assert_eq!(result.unwrap_err(), Failure::EngineLimit { stage: 0 });
    }
}
