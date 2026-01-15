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

use crate::engine::Engine;
use crate::units::{Mass, Ratio, Velocity};

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

    /// Maximum engines per stage (for brute force search)
    pub max_engines_per_stage: u32,
}

impl Default for Constraints {
    /// Default constraints suitable for most orbital rockets.
    ///
    /// - Liftoff TWR: 1.2 (safe margin above 1.0)
    /// - Upper stage TWR: 0.5 (can be lower in vacuum)
    /// - Max stages: 3
    /// - Structural ratio: 0.08 (modern aluminum-lithium)
    /// - Max engines: 9 per stage
    fn default() -> Self {
        Self {
            min_liftoff_twr: Ratio::new(1.2),
            min_stage_twr: Ratio::new(0.5),
            max_stages: 3,
            structural_ratio: Ratio::new(0.08),
            max_engines_per_stage: 9,
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
            max_engines_per_stage: 9,
        }
    }

    /// Set maximum engines per stage (for brute force optimizer).
    pub fn with_max_engines(mut self, max: u32) -> Self {
        self.max_engines_per_stage = max;
        self
    }

    /// Validate that constraints are physically reasonable.
    pub fn validate(&self) -> Result<(), ConstraintError> {
        if self.min_liftoff_twr.as_f64() < 1.0 {
            return Err(ConstraintError::InvalidLiftoffTwr(self.min_liftoff_twr));
        }
        if self.min_stage_twr.as_f64() <= 0.0 {
            return Err(ConstraintError::InvalidStageTwr(self.min_stage_twr));
        }
        if self.max_stages == 0 {
            return Err(ConstraintError::ZeroStages);
        }
        if self.structural_ratio.as_f64() <= 0.0 || self.structural_ratio.as_f64() >= 1.0 {
            return Err(ConstraintError::InvalidStructuralRatio(
                self.structural_ratio,
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
        }
    }

    /// Set a fixed number of stages.
    pub fn with_stage_count(mut self, count: u32) -> Self {
        self.stage_count = Some(count);
        self
    }

    /// Validate that the problem is well-formed.
    pub fn is_valid(&self) -> Result<(), ProblemError> {
        // Check payload
        if self.payload.as_kg() <= 0.0 {
            return Err(ProblemError::InvalidPayload(self.payload));
        }

        // Check delta-v
        if self.target_delta_v.as_mps() <= 0.0 {
            return Err(ProblemError::InvalidDeltaV(self.target_delta_v));
        }

        // Check engines
        if self.available_engines.is_empty() {
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
