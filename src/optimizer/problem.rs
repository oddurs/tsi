//! Optimization problem definition and constraints.
//!
//! This module defines the input to the optimizer: the problem to solve
//! and the constraints that must be satisfied.
//!
//! # Problem Definition
//!
//! An optimization problem specifies:
//! - **Payload**: Mass to deliver to orbit
//! - **Target delta-v**: Required velocity change
//! - **Available engines**: Which engines can be used
//! - **Constraints**: TWR limits, stage count, structural ratio
//!
//! # Constraints
//!
//! Constraints limit the solution space:
//!
//! - **Minimum TWR**: Ensures the rocket can accelerate adequately
//! - **Maximum stages**: Limits complexity (typically 2-3)
//! - **Structural ratio**: Mass fraction for tanks/structure
//!
//! # Example
//!
//! ```
//! use tsi::optimizer::{Problem, Constraints};
//! use tsi::engine::EngineDatabase;
//! use tsi::units::{Mass, Velocity, Ratio};
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let raptor = db.get("raptor-2").expect("engine not found");
//!
//! let problem = Problem::new(
//!     Mass::kg(5_000.0),           // 5 tonne payload
//!     Velocity::mps(9_400.0),      // LEO delta-v
//!     vec![raptor.clone()],
//!     Constraints::default(),
//! );
//!
//! assert!(problem.is_valid().is_ok());
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
///
/// # Typical Values
///
/// | Constraint | Typical Range | Notes |
/// |------------|---------------|-------|
/// | min_twr | 1.2-1.5 | First stage liftoff margin |
/// | min_upper_twr | 0.5-0.8 | Upper stages can be lower |
/// | max_stages | 2-3 | More stages = more complexity |
/// | structural_ratio | 0.05-0.12 | Lower is better |
/// | margin | 0-0.05 | Extra delta-v to design for |
///
/// # Safety Margins
///
/// Real rockets include margins beyond these minimums:
/// - TWR margin for wind/gusts
/// - Propellant reserves (1-2%)
/// - Structural safety factors (1.25-1.5×)
#[derive(Debug, Clone)]
pub struct Constraints {
    /// Minimum TWR at first stage liftoff (must be > 1.0)
    pub min_liftoff_twr: Ratio,

    /// Minimum TWR for upper stages (can be < 1.0 in vacuum)
    pub min_stage_twr: Ratio,

    /// Maximum number of stages allowed
    pub max_stages: u32,

    /// Structural mass as fraction of propellant mass
    pub structural_ratio: Ratio,

    /// Maximum engines per stage
    pub max_engines_per_stage: u32,

    /// Extra delta-v to design for, as a fraction of the target.
    ///
    /// Zero (the default) sizes the rocket to hit the target exactly. A margin
    /// of 0.02 designs for 1.02 × target. Margin is how a design survives the
    /// uncertainties that Monte Carlo analysis measures.
    pub margin: Ratio,

    /// Surface gravity in m/s² used for every TWR check (default: g₀).
    pub surface_gravity: f64,

    /// How the first stage's Isp is evaluated (default: ascent-averaged).
    ///
    /// Use [`IspModel::Vacuum`] for launches from airless bodies.
    pub booster_isp: IspModel,

    /// Least delta-v the first stage must deliver before an upper stage lights,
    /// for Earth launches with more than one stage (default: 2,000 m/s).
    ///
    /// Upper stages are modelled with vacuum Isp, which is only honest if they
    /// ignite above the atmosphere. Without this floor the optimizer finds a
    /// loophole: a "first stage" of engines with no propellant supplies the
    /// liftoff thrust, and a high-Isp upper stage does all the work from sea
    /// level. Real first stages deliver 2.5-4.5 km/s of ideal delta-v
    /// (Falcon 9 about 3.8, Saturn V's S-IC about 3.8, the Shuttle's boosters
    /// about 2.5) and carry the upper stage above 99% of the atmosphere.
    /// 2,000 m/s sits safely below all of them.
    ///
    /// Not applied when [`booster_isp`](Self::booster_isp) is
    /// [`IspModel::Vacuum`], since there is no atmosphere to climb out of.
    pub min_booster_delta_v: Velocity,
}

impl Default for Constraints {
    /// Default constraints suitable for most orbital rockets.
    ///
    /// - Liftoff TWR: 1.2 (safe margin above 1.0)
    /// - Upper stage TWR: 0.5 (can be lower in vacuum)
    /// - Max stages: 3
    /// - Structural ratio: 0.08 (modern aluminum-lithium)
    /// - Max engines: 9 per stage
    /// - Margin: 0 (hit the target exactly)
    /// - Gravity: Earth (g₀), booster Isp ascent-averaged
    fn default() -> Self {
        Self {
            min_liftoff_twr: Ratio::new(1.2),
            min_stage_twr: Ratio::new(0.5),
            max_stages: 3,
            structural_ratio: Ratio::new(0.08),
            max_engines_per_stage: 9,
            margin: Ratio::new(0.0),
            surface_gravity: G0,
            booster_isp: IspModel::AscentAveraged,
            min_booster_delta_v: Velocity::mps(DEFAULT_MIN_BOOSTER_DELTA_V_MPS),
        }
    }
}

impl Constraints {
    /// Create constraints with custom values.
    pub fn new(
        min_liftoff_twr: Ratio,
        min_stage_twr: Ratio,
        max_stages: u32,
        structural_ratio: Ratio,
    ) -> Self {
        Self {
            min_liftoff_twr,
            min_stage_twr,
            max_stages,
            structural_ratio,
            ..Self::default()
        }
    }

    /// Set maximum engines per stage.
    pub fn with_max_engines(mut self, max: u32) -> Self {
        self.max_engines_per_stage = max;
        self
    }

    /// Design for `(1 + margin) × target` delta-v.
    pub fn with_margin(mut self, margin: Ratio) -> Self {
        self.margin = margin;
        self
    }

    /// Set the surface gravity (m/s²) used for TWR checks.
    pub fn with_surface_gravity(mut self, gravity: f64) -> Self {
        self.surface_gravity = gravity;
        self
    }

    /// Set how the first stage's Isp is evaluated.
    pub fn with_booster_isp(mut self, model: IspModel) -> Self {
        self.booster_isp = model;
        self
    }

    /// Set the least delta-v a first stage must deliver below an upper stage.
    pub fn with_min_booster_delta_v(mut self, delta_v: Velocity) -> Self {
        self.min_booster_delta_v = delta_v;
        self
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
        let eps = self.structural_ratio.as_f64();
        if !(eps > 0.0 && eps < 1.0) {
            return Err(ConstraintError::InvalidStructuralRatio(
                self.structural_ratio,
            ));
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
/// The problem defines what the optimizer should achieve:
/// deliver the payload with the required delta-v using the
/// available engines while respecting all constraints.
///
/// # Solution Space
///
/// The optimizer searches over:
/// - Number of stages (1 to max_stages)
/// - Engine selection per stage
/// - Engine count per stage
/// - Propellant mass per stage
///
/// The goal is typically to minimize total mass (maximize payload fraction).
#[derive(Debug, Clone)]
pub struct Problem {
    /// Payload mass to deliver
    pub payload: Mass,

    /// Required delta-v (velocity change)
    pub target_delta_v: Velocity,

    /// Engines available for use
    pub available_engines: Vec<Engine>,

    /// Optimization constraints
    pub constraints: Constraints,

    /// Fixed stage count (None = optimize this too)
    pub stage_count: Option<u32>,

    /// Engines pinned to particular stages, keyed by stage index
    /// (0 = first stage). A pinned stage uses only that engine.
    pub pinned_engines: BTreeMap<usize, Engine>,
}

impl Problem {
    /// Create a new optimization problem.
    pub fn new(
        payload: Mass,
        target_delta_v: Velocity,
        available_engines: Vec<Engine>,
        constraints: Constraints,
    ) -> Self {
        Self {
            payload,
            target_delta_v,
            available_engines,
            constraints,
            stage_count: None,
            pinned_engines: BTreeMap::new(),
        }
    }

    /// Set a fixed number of stages.
    pub fn with_stage_count(mut self, count: u32) -> Self {
        self.stage_count = Some(count);
        self
    }

    /// Require a stage to use a particular engine.
    ///
    /// `stage_index` counts from the bottom: 0 is the first stage. Solutions
    /// always have enough stages to include every pinned one.
    pub fn with_pinned_engine(mut self, stage_index: usize, engine: Engine) -> Self {
        self.pinned_engines.insert(stage_index, engine);
        self
    }

    /// The delta-v the rocket is sized for: target × (1 + margin).
    pub fn design_delta_v(&self) -> Velocity {
        self.target_delta_v * (1.0 + self.constraints.margin.as_f64())
    }

    /// The range of stage counts a solution may have, inclusive.
    ///
    /// A fixed [`stage_count`](Self::stage_count) gives exactly that number.
    /// Otherwise it runs from the fewest stages that include every pinned
    /// engine up to [`Constraints::max_stages`].
    pub fn stage_count_range(&self) -> (u32, u32) {
        match self.stage_count {
            Some(n) => (n, n),
            None => {
                let min_for_pins = self
                    .pinned_engines
                    .keys()
                    .next_back()
                    .map_or(1, |&i| i as u32 + 1);
                (min_for_pins.max(1), self.constraints.max_stages)
            }
        }
    }

    /// Engines that may be used on a given stage.
    ///
    /// A pinned stage gets only its pinned engine. Otherwise every available
    /// engine is a candidate, except that an Earth-launched first stage can't
    /// use an engine with no sea-level performance (a vacuum nozzle at sea
    /// level would separate its flow and could tear itself apart).
    pub fn engines_for_stage(&self, stage_index: usize) -> Vec<&Engine> {
        if let Some(engine) = self.pinned_engines.get(&stage_index) {
            return vec![engine];
        }
        let from_sea_level =
            stage_index == 0 && self.constraints.booster_isp == IspModel::AscentAveraged;
        self.available_engines
            .iter()
            .filter(|e| !(from_sea_level && e.is_upper_stage_only()))
            .collect()
    }

    /// Validate that the problem is well-formed.
    ///
    /// Rejects NaN, infinite and non-positive payloads and targets by name,
    /// before any optimizer sees them.
    pub fn is_valid(&self) -> Result<(), ProblemError> {
        // Check payload
        let payload = self.payload.as_kg();
        if !(payload.is_finite() && payload > 0.0) {
            return Err(ProblemError::InvalidPayload(self.payload));
        }

        // Check delta-v
        let dv = self.target_delta_v.as_mps();
        if !(dv.is_finite() && dv > 0.0) {
            return Err(ProblemError::InvalidDeltaV(self.target_delta_v));
        }

        // Check engines
        if self.available_engines.is_empty() && self.pinned_engines.is_empty() {
            return Err(ProblemError::NoEngines);
        }

        // Check stage count
        if let Some(count) = self.stage_count {
            if count == 0 || count > self.constraints.max_stages {
                return Err(ProblemError::InvalidStageCount {
                    requested: count,
                    max: self.constraints.max_stages,
                });
            }
        }

        // Every pinned stage must fit within the allowed stage count
        let (min, max) = self.stage_count_range();
        if let Some(&stage) = self.pinned_engines.keys().next_back() {
            if stage as u32 >= max {
                return Err(ProblemError::PinnedStageOutOfRange { stage, stages: max });
            }
        }
        if min > max {
            return Err(ProblemError::InvalidStageCount {
                requested: min,
                max,
            });
        }

        // Validate constraints
        self.constraints.validate()?;

        Ok(())
    }

    /// Check if this is a single-engine problem (analytical solution possible).
    pub fn is_single_engine(&self) -> bool {
        self.available_engines.len() == 1
    }

    /// Get the single available engine (if only one).
    pub fn single_engine(&self) -> Option<&Engine> {
        if self.available_engines.len() == 1 {
            self.available_engines.first()
        } else {
            None
        }
    }
}

/// Errors in constraint specification.
#[derive(Debug, Clone, thiserror::Error)]
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
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProblemError {
    #[error("Payload mass must be positive, got {0}")]
    InvalidPayload(Mass),

    #[error("Target delta-v must be positive, got {0}")]
    InvalidDeltaV(Velocity),

    #[error("At least one engine must be available")]
    NoEngines,

    #[error("Stage count {requested} invalid (max {max})")]
    InvalidStageCount { requested: u32, max: u32 },

    #[error("Engine pinned to stage {} but the rocket has at most {stages} stages", stage + 1)]
    PinnedStageOutOfRange { stage: usize, stages: u32 },

    #[error("Constraint error: {0}")]
    Constraint(#[from] ConstraintError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;

    fn get_raptor() -> Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    #[test]
    fn default_constraints() {
        let c = Constraints::default();
        assert_eq!(c.min_liftoff_twr.as_f64(), 1.2);
        assert_eq!(c.min_stage_twr.as_f64(), 0.5);
        assert_eq!(c.max_stages, 3);
        assert!((c.structural_ratio.as_f64() - 0.08).abs() < 0.001);
    }

    #[test]
    fn constraints_validation_passes() {
        let c = Constraints::default();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn constraints_validation_fails_low_liftoff_twr() {
        let c = Constraints::new(
            Ratio::new(0.9), // Invalid: < 1.0
            Ratio::new(0.5),
            2,
            Ratio::new(0.1),
        );
        assert!(matches!(
            c.validate(),
            Err(ConstraintError::InvalidLiftoffTwr(_))
        ));
    }

    #[test]
    fn constraints_validation_fails_zero_stages() {
        let c = Constraints::new(Ratio::new(1.2), Ratio::new(0.5), 0, Ratio::new(0.1));
        assert!(matches!(c.validate(), Err(ConstraintError::ZeroStages)));
    }

    #[test]
    fn problem_construction() {
        let problem = Problem::new(
            Mass::kg(5000.0),
            Velocity::mps(9400.0),
            vec![get_raptor()],
            Constraints::default(),
        );
        assert!(problem.is_valid().is_ok());
    }

    #[test]
    fn problem_is_single_engine() {
        let problem = Problem::new(
            Mass::kg(5000.0),
            Velocity::mps(9400.0),
            vec![get_raptor()],
            Constraints::default(),
        );
        assert!(problem.is_single_engine());
        assert!(problem.single_engine().is_some());
    }

    #[test]
    fn problem_validation_fails_no_engines() {
        let problem = Problem::new(
            Mass::kg(5000.0),
            Velocity::mps(9400.0),
            vec![], // No engines
            Constraints::default(),
        );
        assert!(matches!(problem.is_valid(), Err(ProblemError::NoEngines)));
    }

    #[test]
    fn problem_validation_fails_invalid_payload() {
        let problem = Problem::new(
            Mass::kg(-100.0), // Invalid
            Velocity::mps(9400.0),
            vec![get_raptor()],
            Constraints::default(),
        );
        assert!(matches!(
            problem.is_valid(),
            Err(ProblemError::InvalidPayload(_))
        ));
    }

    #[test]
    fn problem_with_stage_count() {
        let problem = Problem::new(
            Mass::kg(5000.0),
            Velocity::mps(9400.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(2);

        assert_eq!(problem.stage_count, Some(2));
        assert!(problem.is_valid().is_ok());
    }

    #[test]
    fn problem_validation_fails_invalid_stage_count() {
        let problem = Problem::new(
            Mass::kg(5000.0),
            Velocity::mps(9400.0),
            vec![get_raptor()],
            Constraints::default(),
        )
        .with_stage_count(10); // Exceeds max_stages (3)

        assert!(matches!(
            problem.is_valid(),
            Err(ProblemError::InvalidStageCount { .. })
        ));
    }
}
