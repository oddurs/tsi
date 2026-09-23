//! Optimization problem definition and constraints.
//!
//! This module defines the input to the optimizer: the problem to solve
//! and the constraints that must be satisfied.
//!
//! # Problem Definition
//!
//! An optimization problem specifies:
//! - **Payload**: Mass to deliver
//! - **Target delta-v**: Required velocity change
//! - **Available engines**: Which engines can be used
//! - **Constraints**: TWR limits, stage count, structural ratio
//!
//! Problems are made with [`Problem::builder`], which checks everything
//! when you call `build`. A `Problem` you hold is always valid.
//!
//! # Example
//!
//! ```
//! use tsiolkovsky::prelude::*;
//!
//! let db = EngineDatabase::builtin();
//! let raptor = db.get("raptor-2").expect("engine not found");
//!
//! let problem = Problem::builder()
//!     .payload(Mass::tonnes(5.0))
//!     .target(Velocity::mps(9_400.0)) // low Earth orbit
//!     .engine(raptor.clone())
//!     .constraints(Constraints::default().with_margin(0.02))
//!     .build()
//!     .expect("a valid problem");
//!
//! assert_eq!(problem.design_delta_v(), Velocity::mps(9_588.0));
//! ```

use std::collections::BTreeMap;

use crate::engine::Engine;
use crate::physics::{IspModel, G0};
use crate::units::{Mass, Ratio, Velocity};

/// Default for [`Constraints::min_booster_delta_v`].
const DEFAULT_MIN_BOOSTER_DELTA_V_MPS: f64 = 2_000.0;

/// Constraints for staging optimization.
///
/// These constraints define the feasible region for the optimizer.
/// Choosing appropriate constraints is crucial for realistic solutions.
/// Start from [`Constraints::default`] and change what you need:
///
/// ```
/// use tsiolkovsky::optimizer::Constraints;
///
/// let c = Constraints::default()
///     .with_min_liftoff_twr(1.3)
///     .with_structural_ratios([0.04, 0.03]) // booster, upper stage
///     .with_margin(0.02);
/// assert_eq!(c.structural_ratio(1).as_f64(), 0.03);
/// assert_eq!(c.structural_ratio(2).as_f64(), 0.03); // the last one repeats
/// ```
///
/// # Typical Values
///
/// | Constraint | Typical Range | Notes |
/// |------------|---------------|-------|
/// | min liftoff TWR | 1.2-1.5 | First stage liftoff margin |
/// | min stage TWR | 0.5-0.8 | Upper stages can be lower |
/// | max stages | 2-3 | More stages = more complexity |
/// | structural ratio | 0.03-0.11 | Lower is better; see [`Stage`](crate::stage::Stage) for real stages |
/// | margin | 0-0.05 | Extra delta-v to design for |
///
/// # Safety Margins
///
/// Real rockets include margins beyond these minimums:
/// - TWR margin for wind/gusts
/// - Propellant reserves (1-2%)
/// - Structural safety factors (1.25-1.5×)
#[derive(Debug, Clone, PartialEq)]
pub struct Constraints {
    min_liftoff_twr: Ratio,
    min_stage_twr: Ratio,
    max_stages: u32,
    structural_ratios: Vec<Ratio>,
    max_engines_per_stage: u32,
    margin: Ratio,
    surface_gravity: f64,
    booster_isp: IspModel,
    min_booster_delta_v: Velocity,
}

impl Default for Constraints {
    /// Default constraints suitable for most orbital rockets.
    ///
    /// - Liftoff TWR: 1.2 (safe margin above 1.0)
    /// - Upper stage TWR: 0.5 (can be lower in vacuum)
    /// - Max stages: 3
    /// - Structural ratio: 0.08 on every stage
    /// - Max engines: 9 per stage
    /// - Margin: 0 (hit the target exactly)
    /// - Gravity: Earth (g₀), booster Isp ascent-averaged
    /// - First stage delivers at least 2,000 m/s below an upper stage
    fn default() -> Self {
        Self {
            min_liftoff_twr: Ratio::new(1.2),
            min_stage_twr: Ratio::new(0.5),
            max_stages: 3,
            structural_ratios: vec![Ratio::new(0.08)],
            max_engines_per_stage: 9,
            margin: Ratio::new(0.0),
            surface_gravity: G0,
            booster_isp: IspModel::AscentAveraged,
            min_booster_delta_v: Velocity::mps(DEFAULT_MIN_BOOSTER_DELTA_V_MPS),
        }
    }
}

impl Constraints {
    /// Minimum TWR at liftoff (must be at least 1).
    pub fn with_min_liftoff_twr(mut self, twr: impl Into<Ratio>) -> Self {
        self.min_liftoff_twr = twr.into();
        self
    }

    /// Minimum TWR at each upper stage's ignition (can be below 1 in vacuum).
    pub fn with_min_stage_twr(mut self, twr: impl Into<Ratio>) -> Self {
        self.min_stage_twr = twr.into();
        self
    }

    /// Most stages a solution may have.
    pub fn with_max_stages(mut self, stages: u32) -> Self {
        self.max_stages = stages;
        self
    }

    /// The same structural ratio on every stage.
    ///
    /// Structural ratio is structural mass (engines excluded) over propellant
    /// mass; see [`Stage`](crate::stage::Stage) for the definition and real values.
    pub fn with_structural_ratio(mut self, ratio: impl Into<Ratio>) -> Self {
        self.structural_ratios = vec![ratio.into()];
        self
    }

    /// Structural ratios stage by stage, first stage first. Stages beyond the
    /// end of the list use its last value, so `[0.04, 0.06]` means 4% for the
    /// booster and 6% for every stage above it.
    ///
    /// An empty list leaves the ratios unchanged.
    pub fn with_structural_ratios<R: Into<Ratio>>(
        mut self,
        ratios: impl IntoIterator<Item = R>,
    ) -> Self {
        let ratios: Vec<Ratio> = ratios.into_iter().map(Into::into).collect();
        if !ratios.is_empty() {
            self.structural_ratios = ratios;
        }
        self
    }

    /// Most engines on any one stage.
    pub fn with_max_engines(mut self, max: u32) -> Self {
        self.max_engines_per_stage = max;
        self
    }

    /// Design for `(1 + margin) × target` delta-v.
    ///
    /// Zero (the default) sizes the rocket to hit the target exactly. A margin
    /// of 0.02 designs for 1.02 × target. Margin is how a design survives the
    /// uncertainties that Monte Carlo analysis measures.
    pub fn with_margin(mut self, margin: impl Into<Ratio>) -> Self {
        self.margin = margin.into();
        self
    }

    /// Surface gravity in m/s² used for every TWR check (default: g₀).
    pub fn with_surface_gravity(mut self, gravity: f64) -> Self {
        self.surface_gravity = gravity;
        self
    }

    /// How the first stage's Isp is evaluated (default: ascent-averaged).
    ///
    /// Use [`IspModel::Vacuum`] for launches from airless bodies.
    pub fn with_booster_isp(mut self, model: IspModel) -> Self {
        self.booster_isp = model;
        self
    }

    /// Least delta-v the first stage must deliver below an upper stage.
    ///
    /// Upper stages are modelled with vacuum Isp, which is only honest if they
    /// ignite above the atmosphere. Without this floor the optimizer finds a
    /// loophole: a "first stage" of engines with no propellant supplies the
    /// liftoff thrust, and a high-Isp upper stage does all the work from sea
    /// level. Real first stages deliver 2.5-4.5 km/s of ideal delta-v
    /// (Falcon 9 about 3.8, Saturn V's S-IC about 3.8, the Shuttle's boosters
    /// about 2.5) and carry the upper stage above 99% of the atmosphere. The
    /// default, 2,000 m/s, sits safely below all of them.
    ///
    /// Only applied to Earth launches (ascent-averaged booster Isp) with more
    /// than one stage.
    pub fn with_min_booster_delta_v(mut self, delta_v: Velocity) -> Self {
        self.min_booster_delta_v = delta_v;
        self
    }

    /// Minimum TWR at liftoff.
    pub fn min_liftoff_twr(&self) -> Ratio {
        self.min_liftoff_twr
    }

    /// Minimum TWR at upper-stage ignition.
    pub fn min_stage_twr(&self) -> Ratio {
        self.min_stage_twr
    }

    /// Most stages a solution may have.
    pub fn max_stages(&self) -> u32 {
        self.max_stages
    }

    /// Structural ratio for a stage (0 = first stage).
    pub fn structural_ratio(&self, stage_index: usize) -> Ratio {
        let last = self.structural_ratios.len() - 1;
        self.structural_ratios[stage_index.min(last)]
    }

    /// The structural ratios as given, first stage first.
    pub fn structural_ratios(&self) -> &[Ratio] {
        &self.structural_ratios
    }

    /// Most engines on any one stage.
    pub fn max_engines_per_stage(&self) -> u32 {
        self.max_engines_per_stage
    }

    /// Extra delta-v designed in, as a fraction of the target.
    pub fn margin(&self) -> Ratio {
        self.margin
    }

    /// Surface gravity for TWR checks, m/s².
    pub fn surface_gravity(&self) -> f64 {
        self.surface_gravity
    }

    /// How the first stage's Isp is evaluated.
    pub fn booster_isp(&self) -> IspModel {
        self.booster_isp
    }

    /// Least delta-v a first stage must deliver below an upper stage.
    pub fn min_booster_delta_v(&self) -> Velocity {
        self.min_booster_delta_v
    }

    /// The first-stage delta-v floor that applies to a rocket of
    /// `stage_count` stages: zero for single stages and airless launches.
    pub fn booster_delta_v_floor(&self, stage_count: usize) -> Velocity {
        if stage_count > 1 && self.booster_isp == IspModel::AscentAveraged {
            self.min_booster_delta_v
        } else {
            Velocity::mps(0.0)
        }
    }

    /// Validate that constraints are physically reasonable.
    ///
    /// Comparisons are written so that NaN fails them: `!(x >= 1.0)` is true
    /// for NaN where `x < 1.0` is not.
    pub fn validate(&self) -> Result<(), ConstraintError> {
        let liftoff = self.min_liftoff_twr.as_f64();
        if !(liftoff.is_finite() && liftoff >= 1.0) {
            return Err(ConstraintError::InvalidLiftoffTwr(self.min_liftoff_twr));
        }
        let stage = self.min_stage_twr.as_f64();
        if !(stage.is_finite() && stage > 0.0) {
            return Err(ConstraintError::InvalidStageTwr(self.min_stage_twr));
        }
        if self.max_stages == 0 {
            return Err(ConstraintError::ZeroStages);
        }
        if self.max_engines_per_stage == 0 {
            return Err(ConstraintError::ZeroEngines);
        }
        for &ratio in &self.structural_ratios {
            let eps = ratio.as_f64();
            if !(eps > 0.0 && eps < 1.0) {
                return Err(ConstraintError::InvalidStructuralRatio(ratio));
            }
        }
        let margin = self.margin.as_f64();
        if !(margin.is_finite() && margin >= 0.0) {
            return Err(ConstraintError::InvalidMargin(self.margin));
        }
        let g = self.surface_gravity;
        if !(g.is_finite() && g > 0.0) {
            return Err(ConstraintError::InvalidGravity(g));
        }
        let floor = self.min_booster_delta_v.as_mps();
        if !(floor.is_finite() && floor >= 0.0) {
            return Err(ConstraintError::InvalidBoosterDeltaV(
                self.min_booster_delta_v,
            ));
        }
        Ok(())
    }
}

/// An optimization problem to solve.
///
/// The problem defines what the optimizer should achieve: deliver the payload
/// with the required delta-v using the available engines while respecting all
/// constraints. Make one with [`Problem::builder`]; every `Problem` has passed
/// validation.
///
/// # Solution Space
///
/// The optimizer searches over:
/// - Number of stages (1 to max stages, or a fixed count)
/// - Engine selection per stage (or a pinned engine)
/// - Engine count per stage
/// - Propellant mass per stage
///
/// The goal is to minimize total mass (maximize payload fraction).
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    payload: Mass,
    target_delta_v: Velocity,
    engines: Vec<Engine>,
    constraints: Constraints,
    stage_count: Option<u32>,
    pinned_engines: BTreeMap<usize, Engine>,
}

impl Problem {
    /// Start describing a problem.
    pub fn builder() -> ProblemBuilder {
        ProblemBuilder::default()
    }

    /// A builder holding this problem, to make a variation of it.
    ///
    /// ```
    /// # use tsiolkovsky::prelude::*;
    /// # let raptor = EngineDatabase::builtin().get("raptor-2").unwrap().clone();
    /// let leo = Problem::builder()
    ///     .payload(Mass::tonnes(5.0))
    ///     .target(Velocity::mps(9_400.0))
    ///     .engine(raptor)
    ///     .build()?;
    /// let gto = leo.to_builder().target(Velocity::mps(11_800.0)).build()?;
    /// assert_eq!(gto.payload(), leo.payload());
    /// # Ok::<(), tsiolkovsky::optimizer::ProblemError>(())
    /// ```
    pub fn to_builder(&self) -> ProblemBuilder {
        ProblemBuilder {
            payload: Some(self.payload),
            target_delta_v: Some(self.target_delta_v),
            engines: self.engines.clone(),
            constraints: self.constraints.clone(),
            stage_count: self.stage_count,
            pinned_engines: self.pinned_engines.clone(),
        }
    }

    /// Mass to deliver.
    pub fn payload(&self) -> Mass {
        self.payload
    }

    /// Delta-v the rocket must reach.
    pub fn target_delta_v(&self) -> Velocity {
        self.target_delta_v
    }

    /// Engines any stage may choose from (pinned stages excepted).
    pub fn engines(&self) -> &[Engine] {
        &self.engines
    }

    /// The constraints.
    pub fn constraints(&self) -> &Constraints {
        &self.constraints
    }

    /// Fixed stage count, if one was asked for.
    pub fn stage_count(&self) -> Option<u32> {
        self.stage_count
    }

    /// Engines pinned to stages, keyed by stage index (0 = first stage).
    pub fn pinned_engines(&self) -> &BTreeMap<usize, Engine> {
        &self.pinned_engines
    }

    /// The delta-v the rocket is sized for: target × (1 + margin).
    pub fn design_delta_v(&self) -> Velocity {
        self.target_delta_v * (1.0 + self.constraints.margin.as_f64())
    }

    /// The range of stage counts a solution may have, inclusive.
    ///
    /// A fixed stage count gives exactly that number. Otherwise it runs from
    /// the fewest stages that include every pinned engine up to the maximum.
    pub fn stage_count_range(&self) -> (u32, u32) {
        stage_count_range(self.stage_count, &self.pinned_engines, &self.constraints)
    }

    /// Engines that may be used on a given stage.
    ///
    /// A pinned stage gets only its pinned engine. Otherwise every available
    /// engine is a candidate, except that an Earth-launched first stage can't
    /// use an engine with no sea-level performance (a vacuum nozzle at sea
    /// level would separate its flow and could tear itself apart).
    pub fn engines_for_stage(&self, stage_index: usize) -> Vec<&Engine> {
        engines_for_stage(
            stage_index,
            &self.engines,
            &self.pinned_engines,
            &self.constraints,
        )
    }
}

fn stage_count_range(
    stage_count: Option<u32>,
    pinned: &BTreeMap<usize, Engine>,
    constraints: &Constraints,
) -> (u32, u32) {
    match stage_count {
        Some(n) => (n, n),
        None => {
            let min_for_pins = pinned.keys().next_back().map_or(1, |&i| {
                u32::try_from(i).map_or(u32::MAX, |i| i.saturating_add(1))
            });
            (min_for_pins.max(1), constraints.max_stages)
        }
    }
}

fn engines_for_stage<'a>(
    stage_index: usize,
    engines: &'a [Engine],
    pinned: &'a BTreeMap<usize, Engine>,
    constraints: &Constraints,
) -> Vec<&'a Engine> {
    if let Some(engine) = pinned.get(&stage_index) {
        return vec![engine];
    }
    let from_sea_level = stage_index == 0 && constraints.booster_isp == IspModel::AscentAveraged;
    engines
        .iter()
        .filter(|e| !(from_sea_level && e.is_upper_stage_only()))
        .collect()
}

/// Builds a [`Problem`], checking it when you call [`build`](Self::build).
///
/// Payload, target and at least one engine (offered or pinned) are required;
/// everything else has a default.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct ProblemBuilder {
    payload: Option<Mass>,
    target_delta_v: Option<Velocity>,
    engines: Vec<Engine>,
    constraints: Constraints,
    stage_count: Option<u32>,
    pinned_engines: BTreeMap<usize, Engine>,
}

impl ProblemBuilder {
    /// Mass to deliver (required).
    pub fn payload(mut self, payload: Mass) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Delta-v to reach (required).
    pub fn target(mut self, delta_v: Velocity) -> Self {
        self.target_delta_v = Some(delta_v);
        self
    }

    /// Offer an engine to every stage.
    pub fn engine(mut self, engine: Engine) -> Self {
        self.engines.push(engine);
        self
    }

    /// Offer several engines to every stage.
    pub fn engines(mut self, engines: impl IntoIterator<Item = Engine>) -> Self {
        self.engines.extend(engines);
        self
    }

    /// Use these constraints (default: [`Constraints::default`]).
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Use exactly this many stages. Without it, every count from 1 up to
    /// [`Constraints::max_stages`] is tried and the lightest wins.
    pub fn stages(mut self, count: u32) -> Self {
        self.stage_count = Some(count);
        self
    }

    /// Require a stage to use a particular engine.
    ///
    /// `stage_index` counts from the bottom: 0 is the first stage. Solutions
    /// always have enough stages to include every pinned one.
    pub fn pin(mut self, stage_index: usize, engine: Engine) -> Self {
        self.pinned_engines.insert(stage_index, engine);
        self
    }

    /// Check the problem and build it.
    ///
    /// # Errors
    ///
    /// [`ProblemError`] for a missing or non-positive payload or target, no
    /// engines, invalid constraints, a stage count out of range, or engines
    /// that can't fly the stages they'd have to (see the variants).
    pub fn build(self) -> Result<Problem, ProblemError> {
        let payload = self.payload.ok_or(ProblemError::MissingPayload)?;
        let target_delta_v = self.target_delta_v.ok_or(ProblemError::MissingTarget)?;

        let kg = payload.as_kg();
        if !(kg.is_finite() && kg > 0.0) {
            return Err(ProblemError::InvalidPayload(payload));
        }
        let dv = target_delta_v.as_mps();
        if !(dv.is_finite() && dv > 0.0) {
            return Err(ProblemError::InvalidDeltaV(target_delta_v));
        }
        if self.engines.is_empty() && self.pinned_engines.is_empty() {
            return Err(ProblemError::NoEngines);
        }
        self.constraints.validate()?;

        // Every pinned stage must fit within the allowed stage count
        let (min, max) =
            stage_count_range(self.stage_count, &self.pinned_engines, &self.constraints);
        if let Some(count) = self.stage_count {
            if count == 0 || count > self.constraints.max_stages {
                return Err(ProblemError::InvalidStageCount {
                    requested: count,
                    max: self.constraints.max_stages,
                });
            }
        }
        if let Some(&stage) = self.pinned_engines.keys().next_back() {
            if stage >= max as usize {
                return Err(ProblemError::PinnedStageOutOfRange { stage, stages: max });
            }
        }
        if min > max {
            return Err(ProblemError::InvalidStageCount {
                requested: min,
                max,
            });
        }

        // A vacuum-only engine can't be pinned under an Earth launch
        let from_sea_level = self.constraints.booster_isp == IspModel::AscentAveraged;
        if let Some(engine) = self.pinned_engines.get(&0) {
            if from_sea_level && engine.is_upper_stage_only() {
                return Err(ProblemError::VacuumEngineOnBooster {
                    engine: engine.name().to_string(),
                });
            }
        }

        // Some allowed stage count must have a candidate engine for every stage
        let candidates =
            |i| engines_for_stage(i, &self.engines, &self.pinned_engines, &self.constraints);
        if candidates(0).is_empty() {
            return Err(ProblemError::NoFirstStageEngine {
                offered: self.engines.iter().map(|e| e.name().to_string()).collect(),
            });
        }
        let buildable = (min..=max).any(|n| (0..n as usize).all(|i| !candidates(i).is_empty()));
        if !buildable {
            let stage = (0..min as usize)
                .find(|&i| candidates(i).is_empty())
                .unwrap_or(0);
            return Err(ProblemError::NoEngineForStage { stage });
        }

        Ok(Problem {
            payload,
            target_delta_v,
            engines: self.engines,
            constraints: self.constraints,
            stage_count: self.stage_count,
            pinned_engines: self.pinned_engines,
        })
    }
}

/// Errors in constraint specification.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ConstraintError {
    #[error("Liftoff TWR must be >= 1.0, got {0}")]
    InvalidLiftoffTwr(Ratio),

    #[error("Stage TWR must be > 0.0, got {0}")]
    InvalidStageTwr(Ratio),

    #[error("Must have at least 1 stage")]
    ZeroStages,

    #[error("Structural ratio must be between 0 and 1, got {0}")]
    InvalidStructuralRatio(Ratio),

    #[error("Must allow at least 1 engine per stage")]
    ZeroEngines,

    #[error("Delta-v margin must be zero or positive, got {0}")]
    InvalidMargin(Ratio),

    #[error("Surface gravity must be positive, got {0} m/s²")]
    InvalidGravity(f64),

    #[error("Minimum booster delta-v must be zero or positive, got {0}")]
    InvalidBoosterDeltaV(Velocity),
}

/// Errors in problem specification.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProblemError {
    #[error("No payload given")]
    MissingPayload,

    #[error("No target delta-v given")]
    MissingTarget,

    #[error("Payload mass must be positive, got {0}")]
    InvalidPayload(Mass),

    #[error("Target delta-v must be positive, got {0}")]
    InvalidDeltaV(Velocity),

    #[error("At least one engine must be available")]
    NoEngines,

    #[error("Stage count {requested} invalid (max {max})")]
    InvalidStageCount { requested: u32, max: u32 },

    #[error("Engine pinned to stage {} but the rocket has at most {stages} stages", stage.saturating_add(1))]
    PinnedStageOutOfRange { stage: usize, stages: u32 },

    #[error(
        "{engine} has no sea-level rating, so it can't fly an Earth-launched first stage. \
        Pin a sea-level engine such as merlin-1d or raptor-2 to stage 1."
    )]
    VacuumEngineOnBooster { engine: String },

    #[error(
        "None of the engines offered ({}) can fly from sea level: they're vacuum-only. \
        Add a first-stage engine such as merlin-1d or raptor-2, or pin one to stage 1.",
        offered.join(", ")
    )]
    NoFirstStageEngine { offered: Vec<String> },

    #[error("No engine is available for stage {}: offer more engines or pin one to it", stage + 1)]
    NoEngineForStage { stage: usize },

    #[error("Constraint error: {0}")]
    Constraint(#[from] ConstraintError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;

    fn get(name: &str) -> Engine {
        EngineDatabase::builtin().get(name).unwrap().clone()
    }

    fn leo() -> ProblemBuilder {
        Problem::builder()
            .payload(Mass::kg(5000.0))
            .target(Velocity::mps(9400.0))
            .engine(get("raptor-2"))
    }

    #[test]
    fn default_constraints() {
        let c = Constraints::default();
        assert_eq!(c.min_liftoff_twr().as_f64(), 1.2);
        assert_eq!(c.min_stage_twr().as_f64(), 0.5);
        assert_eq!(c.max_stages(), 3);
        assert_eq!(c.structural_ratio(0).as_f64(), 0.08);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn low_liftoff_twr_is_rejected() {
        let c = Constraints::default().with_min_liftoff_twr(0.9);
        assert!(matches!(
            c.validate(),
            Err(ConstraintError::InvalidLiftoffTwr(_))
        ));
    }

    #[test]
    fn zero_stages_is_rejected() {
        let c = Constraints::default().with_max_stages(0);
        assert!(matches!(c.validate(), Err(ConstraintError::ZeroStages)));
    }

    #[test]
    fn per_stage_structural_ratios_repeat_the_last() {
        let c = Constraints::default().with_structural_ratios([0.04, 0.06]);
        assert_eq!(c.structural_ratio(0).as_f64(), 0.04);
        assert_eq!(c.structural_ratio(1).as_f64(), 0.06);
        assert_eq!(c.structural_ratio(5).as_f64(), 0.06);
        let bad = Constraints::default().with_structural_ratios([0.04, 1.5]);
        assert!(matches!(
            bad.validate(),
            Err(ConstraintError::InvalidStructuralRatio(_))
        ));
    }

    #[test]
    fn builder_builds_a_valid_problem() {
        let problem = leo().stages(2).build().unwrap();
        assert_eq!(problem.stage_count(), Some(2));
        assert_eq!(problem.engines().len(), 1);
    }

    #[test]
    fn builder_requires_payload_and_target() {
        let no_payload = Problem::builder()
            .target(Velocity::mps(9400.0))
            .engine(get("raptor-2"))
            .build();
        assert_eq!(no_payload.unwrap_err(), ProblemError::MissingPayload);
        let no_target = Problem::builder()
            .payload(Mass::kg(1.0))
            .engine(get("raptor-2"))
            .build();
        assert_eq!(no_target.unwrap_err(), ProblemError::MissingTarget);
    }

    #[test]
    fn no_engines_is_rejected() {
        let result = Problem::builder()
            .payload(Mass::kg(5000.0))
            .target(Velocity::mps(9400.0))
            .build();
        assert_eq!(result.unwrap_err(), ProblemError::NoEngines);
    }

    #[test]
    fn negative_and_nan_payloads_are_rejected() {
        for kg in [-100.0, 0.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                leo().payload(Mass::kg(kg)).build(),
                Err(ProblemError::InvalidPayload(_))
            ));
        }
    }

    #[test]
    fn stage_count_beyond_max_is_rejected() {
        assert!(matches!(
            leo().stages(10).build(),
            Err(ProblemError::InvalidStageCount { .. })
        ));
    }

    #[test]
    fn huge_pinned_stage_index_is_rejected_not_wrapped() {
        // 1 << 32 must not truncate to stage 0, and usize::MAX must not overflow.
        for index in [usize::MAX, 1usize << 32.min(usize::BITS - 1)] {
            assert!(matches!(
                leo().pin(index, get("raptor-2")).build(),
                Err(ProblemError::PinnedStageOutOfRange { .. })
            ));
        }
    }

    #[test]
    fn vacuum_engine_pinned_to_booster_is_rejected() {
        assert!(matches!(
            leo().pin(0, get("rl-10c")).build(),
            Err(ProblemError::VacuumEngineOnBooster { .. })
        ));
    }

    #[test]
    fn only_vacuum_engines_is_rejected() {
        let rl10 = || {
            Problem::builder()
                .payload(Mass::kg(5000.0))
                .target(Velocity::mps(9400.0))
                .engine(get("rl-10c"))
        };
        assert!(matches!(
            rl10().build(),
            Err(ProblemError::NoFirstStageEngine { .. })
        ));
        // ...but fine from the Moon, where there is no sea level to fly from.
        let lunar = rl10()
            .constraints(Constraints::default().with_booster_isp(IspModel::Vacuum))
            .build();
        assert!(lunar.is_ok());
    }

    #[test]
    fn to_builder_round_trips() {
        let problem = leo()
            .stages(2)
            .pin(1, get("raptor-vacuum"))
            .build()
            .unwrap();
        assert_eq!(problem.to_builder().build().unwrap(), problem);
    }
}
