//! Optimization solution representation.
//!
//! A solution contains the rocket the optimizer found, along with what it
//! was asked for and how it got there.

use std::fmt;
use std::time::Duration;

use serde::Serialize;

use crate::physics::IspModel;
use crate::stage::Rocket;
use crate::units::{Isp, Mass, Ratio, Time, Velocity};

/// Rounding allowance when checking that a solution reaches its target.
pub(crate) const DELTA_V_TOLERANCE_MPS: f64 = 1e-3;

/// Which optimizer produced a solution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OptimizerKind {
    /// [`AnalyticalOptimizer`](super::AnalyticalOptimizer)
    Analytical,
    /// [`BruteForceOptimizer`](super::BruteForceOptimizer)
    BruteForce,
    /// An optimizer from outside this crate, by name.
    Other(String),
}

impl fmt::Display for OptimizerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizerKind::Analytical => write!(f, "Analytical"),
            OptimizerKind::BruteForce => write!(f, "BruteForce"),
            OptimizerKind::Other(name) => write!(f, "{name}"),
        }
    }
}

/// Result of an optimization run.
///
/// Contains the optimized rocket and metadata about how well it meets the
/// requirements.
///
/// # Margin
///
/// The margin is the delta-v beyond the target. Optimizers size rockets to hit
/// the target exactly unless the problem asks for margin
/// ([`Constraints::with_margin`](super::Constraints::with_margin)), which buys
/// room for the losses and manufacturing variation that ideal delta-v leaves
/// out:
/// - Gravity losses (typically 1,000-1,500 m/s)
/// - Atmospheric drag (typically 100-400 m/s)
/// - Navigation corrections
/// - Propellant reserves
///
/// # Example
///
/// ```
/// use tsiolkovsky::prelude::*;
///
/// let raptor = EngineDatabase::builtin().get("raptor-2").unwrap().clone();
/// let problem = Problem::builder()
///     .payload(Mass::tonnes(5.0))
///     .target(Velocity::mps(9_400.0))
///     .engine(raptor)
///     .build()?;
///
/// let solution = AnalyticalOptimizer.optimize(&problem)?;
///
/// println!("Total mass: {}", solution.rocket().total_mass());
/// println!("Delta-v margin: {:+.0} m/s", solution.margin().as_mps());
/// println!("Payload fraction: {:.2}%", solution.payload_fraction_percent());
/// println!(
///     "{} evaluated {} configurations in {:?}",
///     solution.optimizer(),
///     solution.iterations(),
///     solution.runtime()
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct Solution {
    rocket: Rocket,
    target_delta_v: Velocity,
    margin: Velocity,
    iterations: u64,
    runtime: Duration,
    optimizer: OptimizerKind,
}

impl Solution {
    /// Wrap a rocket as the solution to a problem with the given target.
    ///
    /// Custom [`Optimizer`](super::Optimizer) implementations use this to
    /// report what they found.
    pub fn new(
        rocket: Rocket,
        target_delta_v: Velocity,
        iterations: u64,
        runtime: Duration,
        optimizer: OptimizerKind,
    ) -> Self {
        let margin = rocket.total_delta_v() - target_delta_v;
        Self {
            rocket,
            target_delta_v,
            margin,
            iterations,
            runtime,
            optimizer,
        }
    }

    /// The optimized rocket.
    pub fn rocket(&self) -> &Rocket {
        &self.rocket
    }

    /// Take the rocket out of the solution.
    pub fn into_rocket(self) -> Rocket {
        self.rocket
    }

    /// The delta-v the solution was asked to reach.
    pub fn target_delta_v(&self) -> Velocity {
        self.target_delta_v
    }

    /// Delta-v beyond the target (positive = excess capacity).
    pub fn margin(&self) -> Velocity {
        self.margin
    }

    /// Configurations the optimizer evaluated.
    pub fn iterations(&self) -> u64 {
        self.iterations
    }

    /// Time the optimization took.
    pub fn runtime(&self) -> Duration {
        self.runtime
    }

    /// Which optimizer found this.
    pub fn optimizer(&self) -> &OptimizerKind {
        &self.optimizer
    }

    /// Payload fraction as a percentage.
    pub fn payload_fraction_percent(&self) -> f64 {
        self.rocket.payload_fraction().as_f64() * 100.0
    }

    /// Whether the solution meets or exceeds the target delta-v.
    ///
    /// Optimizers size rockets to hit the target exactly, so this allows for
    /// floating-point rounding of a millimetre per second.
    pub fn meets_target(&self) -> bool {
        self.margin.as_mps() >= -DELTA_V_TOLERANCE_MPS
    }

    /// Margin as a percentage of the target delta-v.
    pub fn margin_percent(&self) -> f64 {
        self.margin.as_mps() / self.target_delta_v.as_mps() * 100.0
    }

    /// Everything about the solution, computed and ready to serialize.
    ///
    /// Units are in the field names: `total_mass_kg` is kilograms,
    /// `delta_v_mps` metres per second. `Solution` serializes as its report.
    pub fn report(&self) -> SolutionReport {
        let rocket = &self.rocket;
        let stages = rocket
            .stages()
            .iter()
            .enumerate()
            .map(|(i, stage)| StageReport {
                stage: i + 1,
                engine: stage.engine().name().to_string(),
                engine_count: stage.engine_count(),
                propellant_kg: stage.propellant_mass(),
                structural_mass_kg: stage.structural_mass(),
                dry_mass_kg: stage.dry_mass(),
                wet_mass_kg: stage.wet_mass(),
                delta_v_mps: rocket.stage_delta_v(i),
                isp_s: stage.engine().isp_for(rocket.isp_model(i)),
                burn_time_s: stage.burn_time(),
                twr_ignition: rocket.stage_twr(i),
                twr_liftoff: (i == 0).then(|| rocket.liftoff_twr()),
            })
            .collect();
        SolutionReport {
            target_delta_v_mps: self.target_delta_v,
            payload_kg: rocket.payload(),
            total_mass_kg: rocket.total_mass(),
            total_delta_v_mps: rocket.total_delta_v(),
            payload_fraction: rocket.payload_fraction(),
            margin_mps: self.margin,
            margin_percent: self.margin_percent(),
            booster_isp_model: rocket.booster_isp(),
            surface_gravity_mps2: rocket.surface_gravity(),
            stages,
            metadata: Metadata {
                optimizer: self.optimizer.to_string(),
                iterations: self.iterations,
                runtime_ms: self.runtime.as_millis() as u64,
            },
        }
    }
}

impl Serialize for Solution {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.report().serialize(serializer)
    }
}

/// A [`Solution`] with every derived number filled in, for serialization.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct SolutionReport {
    pub target_delta_v_mps: Velocity,
    pub payload_kg: Mass,
    pub total_mass_kg: Mass,
    pub total_delta_v_mps: Velocity,
    pub payload_fraction: Ratio,
    pub margin_mps: Velocity,
    pub margin_percent: f64,
    pub booster_isp_model: IspModel,
    pub surface_gravity_mps2: f64,
    pub stages: Vec<StageReport>,
    pub metadata: Metadata,
}

/// One stage of a [`SolutionReport`], first stage first.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct StageReport {
    /// Stage number, 1 = first stage
    pub stage: usize,
    pub engine: String,
    pub engine_count: u32,
    pub propellant_kg: Mass,
    pub structural_mass_kg: Mass,
    pub dry_mass_kg: Mass,
    pub wet_mass_kg: Mass,
    pub delta_v_mps: Velocity,
    /// Effective Isp for this stage's burn (ascent-averaged for an Earth first stage)
    pub isp_s: Isp,
    pub burn_time_s: Time,
    /// Vacuum thrust over everything above and including this stage, at ignition
    pub twr_ignition: Ratio,
    /// What gets the rocket off the pad (first stage only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twr_liftoff: Option<Ratio>,
}

/// How a solution was found.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Metadata {
    pub optimizer: String,
    pub iterations: u64,
    pub runtime_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;
    use crate::stage::Stage;

    fn simple_rocket() -> Rocket {
        let raptor = EngineDatabase::builtin().get("Raptor-2").unwrap().clone();
        let stage1 =
            Stage::with_structural_ratio(raptor.clone(), 9, Mass::kg(1_000_000.0), 0.05).unwrap();
        let stage2 = Stage::with_structural_ratio(raptor, 1, Mass::kg(100_000.0), 0.08).unwrap();
        Rocket::new(vec![stage1, stage2], Mass::kg(50_000.0)).unwrap()
    }

    fn solution(target: f64) -> Solution {
        Solution::new(
            simple_rocket(),
            Velocity::mps(target),
            100,
            Duration::ZERO,
            OptimizerKind::Other("test".into()),
        )
    }

    #[test]
    fn margin_is_achieved_minus_target() {
        let s = solution(8_000.0);
        assert!(s.meets_target());
        let achieved = s.rocket().total_delta_v().as_mps();
        assert!((s.margin().as_mps() - (achieved - 8_000.0)).abs() < 1e-9);
        assert!(s.margin_percent() > 0.0);
    }

    #[test]
    fn falls_short_of_an_unreachable_target() {
        assert!(!solution(50_000.0).meets_target());
    }

    #[test]
    fn payload_fraction() {
        let pct = solution(8_000.0).payload_fraction_percent();
        assert!((1.0..10.0).contains(&pct));
    }

    #[test]
    fn optimizer_names() {
        assert_eq!(OptimizerKind::Analytical.to_string(), "Analytical");
        assert_eq!(
            OptimizerKind::Other("Genetic".into()).to_string(),
            "Genetic"
        );
    }

    #[test]
    fn report_has_liftoff_twr_only_on_stage_one() {
        let report = solution(8_000.0).report();
        assert!(report.stages[0].twr_liftoff.is_some());
        assert!(report.stages[1].twr_liftoff.is_none());
        assert_eq!(report.stages[1].stage, 2);
    }
}
