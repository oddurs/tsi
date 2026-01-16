//! Multi-stage rocket assembly and performance aggregation.
//!
//! A rocket is a complete launch vehicle: multiple stages stacked together
//! with a payload on top. This module aggregates stage performance to
//! calculate total delta-v, mass, and payload fraction.
//!
//! # Stage Ordering
//!
//! Stages are stored bottom-to-top (index 0 = first stage/booster).
//! This matches the physical stacking order and firing sequence:
//!
//! ```text
//!     ┌─────────┐
//!     │ Payload │
//!     ├─────────┤
//!     │ Stage 2 │  ← Upper stage (fires second)
//!     ├─────────┤
//!     │ Stage 1 │  ← First stage (fires first, index 0)
//!     └─────────┘
//! ```
//!
//! # Delta-V Aggregation
//!
//! Total delta-v is the sum of individual stage delta-vs. Each stage
//! must carry all stages above it plus the payload, which affects its
//! effective mass ratio.
//!
//! # Example
//!
//! ```
//! use tsi::stage::{Stage, Rocket};
//! use tsi::engine::EngineDatabase;
//! use tsi::units::Mass;
//!
//! let db = EngineDatabase::load_embedded().expect("failed to load database");
//! let raptor = db.get("raptor-2").expect("engine not found");
//!
//! // Create a two-stage rocket
//! let stage1 = Stage::with_structural_ratio(
//!     raptor.clone(), 9, Mass::kg(1_000_000.0), 0.05
//! );
//! let stage2 = Stage::with_structural_ratio(
//!     raptor.clone(), 1, Mass::kg(100_000.0), 0.08
//! );
//!
//! let rocket = Rocket::new(vec![stage1, stage2], Mass::kg(100_000.0));
//!
//! println!("Total delta-v: {}", rocket.total_delta_v());
//! println!("Total mass: {}", rocket.total_mass());
//! println!("Payload fraction: {:.2}%", rocket.payload_fraction().as_f64() * 100.0);
//! ```

use crate::physics::{twr, G0};
use crate::units::{Mass, Ratio, Time, Velocity};

use super::Stage;

/// A complete multi-stage rocket with payload.
///
/// Rockets achieve orbit by staging: discarding empty tanks and engines
/// to improve mass ratio for subsequent burns. A well-designed rocket
/// balances delta-v distribution across stages to maximize payload.
///
/// # Typical Configurations
///
/// | Vehicle | Stages | Total Δv | Payload Fraction |
/// |---------|--------|----------|------------------|
/// | Falcon 9 | 2 | ~15,000 m/s | ~4% to LEO |
/// | Saturn V | 3 | ~18,000 m/s | ~4% to TLI |
/// | Electron | 2 | ~10,000 m/s | ~2% to LEO |
///
/// # Payload Fraction
///
/// The payload fraction (payload mass / total mass) is the key efficiency
/// metric. Higher is better, but practical limits exist:
///
/// - 2-3%: Small launchers (Electron, RocketLab)
/// - 3-5%: Medium launchers (Falcon 9, Atlas V)
/// - 4-5%: Heavy lift (Saturn V, SLS)
#[derive(Debug, Clone)]
pub struct Rocket {
    /// Stages from bottom to top (index 0 = first stage)
    stages: Vec<Stage>,
    /// Payload mass carried to final orbit
    payload: Mass,
}

impl Rocket {
    /// Create a new rocket from stages and payload.
    ///
    /// # Arguments
    ///
    /// * `stages` - Stages ordered bottom-to-top (first stage at index 0)
    /// * `payload` - Mass delivered to final orbit
    ///
    /// # Panics
    ///
    /// Panics if `stages` is empty.
    pub fn new(stages: Vec<Stage>, payload: Mass) -> Self {
        assert!(!stages.is_empty(), "Rocket must have at least one stage");
        Self { stages, payload }
    }

    /// Get the stages (bottom to top).
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    /// Get the payload mass.
    pub fn payload(&self) -> Mass {
        self.payload
    }

    /// Number of stages.
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Total delta-v of the rocket (sum of all stages).
    ///
    /// Each stage's delta-v is calculated with the payload of all
    /// stages above it plus the final payload. This accounts for
    /// the mass each stage must accelerate.
    ///
    /// # Note
    ///
    /// This is the ideal delta-v assuming:
    /// - No gravity losses
    /// - No atmospheric drag
    /// - Complete propellant consumption
    /// - Instantaneous staging
    pub fn total_delta_v(&self) -> Velocity {
        let mut total = Velocity::mps(0.0);

        for i in 0..self.stages.len() {
            let stage_dv = self.stage_delta_v(i);
            total = total + stage_dv;
        }

        total
    }

    /// Delta-v contribution from a specific stage.
    ///
    /// Accounts for payload above this stage (upper stages + payload).
    pub fn stage_delta_v(&self, stage_index: usize) -> Velocity {
        let stage = &self.stages[stage_index];
        let payload_above = self.mass_above_stage(stage_index);
        stage.delta_v_with_payload(payload_above)
    }

    /// Mass above a given stage (upper stages + payload).
    ///
    /// This is what the stage must carry and accelerate.
    pub fn mass_above_stage(&self, stage_index: usize) -> Mass {
        let mut mass = self.payload;

        // Add wet mass of all stages above this one
        for i in (stage_index + 1)..self.stages.len() {
            mass = mass + self.stages[i].wet_mass();
        }

        mass
    }

    /// Total wet mass at liftoff (all stages + payload).
    pub fn total_mass(&self) -> Mass {
        let mut mass = self.payload;
        for stage in &self.stages {
            mass = mass + stage.wet_mass();
        }
        mass
    }

    /// Payload fraction: payload / total mass.
    ///
    /// This is the primary efficiency metric for launch vehicles.
    /// A payload fraction of 0.04 (4%) is excellent for orbital rockets.
    pub fn payload_fraction(&self) -> Ratio {
        self.payload / self.total_mass()
    }

    /// Total burn time across all stages.
    pub fn total_burn_time(&self) -> Time {
        let mut total = Time::seconds(0.0);
        for stage in &self.stages {
            total = total + stage.burn_time();
        }
        total
    }

    /// Thrust-to-weight ratio at liftoff (first stage ignition).
    ///
    /// Must be > 1.0 for the rocket to leave the pad.
    /// Typical values: 1.2 - 1.5 for safety margin.
    pub fn liftoff_twr(&self) -> Ratio {
        let first_stage = &self.stages[0];
        let total_mass = self.total_mass();
        twr(first_stage.thrust_sl(), total_mass, G0)
    }

    /// TWR at ignition of a specific stage (vacuum).
    ///
    /// # Arguments
    ///
    /// * `stage_index` - Which stage (0 = first stage)
    pub fn stage_twr(&self, stage_index: usize) -> Ratio {
        let stage = &self.stages[stage_index];
        let payload_above = self.mass_above_stage(stage_index);
        stage.twr_vac_with_payload(payload_above)
    }

    /// Check if all stage TWRs meet a minimum threshold.
    ///
    /// # Arguments
    ///
    /// * `min_twr` - Minimum acceptable TWR (typically 0.7-1.0 for upper stages)
    /// * `require_liftoff` - If true, first stage must have TWR > 1.0
    pub fn validate_twr(&self, min_twr: Ratio, require_liftoff: bool) -> Result<(), TwrError> {
        // Check liftoff TWR for first stage
        if require_liftoff {
            let liftoff = self.liftoff_twr();
            if liftoff.as_f64() < 1.0 {
                return Err(TwrError::InsufficientLiftoff {
                    twr: liftoff,
                    required: Ratio::new(1.0),
                });
            }
        }

        // Check each stage's vacuum TWR
        for i in 0..self.stages.len() {
            let stage_twr = self.stage_twr(i);
            if stage_twr.as_f64() < min_twr.as_f64() {
                return Err(TwrError::InsufficientStageTwr {
                    stage: i,
                    twr: stage_twr,
                    required: min_twr,
                });
            }
        }

        Ok(())
    }
}

/// Errors from TWR validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TwrError {
    /// First stage cannot lift off (TWR < 1.0)
    #[error("Insufficient liftoff TWR: {twr} < {required}")]
    InsufficientLiftoff { twr: Ratio, required: Ratio },

    /// A stage has insufficient TWR
    #[error("Stage {stage} TWR too low: {twr} < {required}")]
    InsufficientStageTwr {
        stage: usize,
        twr: Ratio,
        required: Ratio,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineDatabase;

    fn get_raptor() -> crate::engine::Engine {
        let db = EngineDatabase::default();
        db.get("Raptor-2").unwrap().clone()
    }

    fn get_merlin() -> crate::engine::Engine {
        let db = EngineDatabase::default();
        db.get("Merlin-1D").unwrap().clone()
    }

    fn simple_two_stage() -> Rocket {
        let stage1 = Stage::with_structural_ratio(get_raptor(), 9, Mass::kg(1_000_000.0), 0.05);
        let stage2 = Stage::with_structural_ratio(get_raptor(), 1, Mass::kg(100_000.0), 0.08);
        Rocket::new(vec![stage1, stage2], Mass::kg(50_000.0))
    }

    #[test]
    fn rocket_construction() {
        let rocket = simple_two_stage();
        assert_eq!(rocket.stage_count(), 2);
        assert_eq!(rocket.payload().as_kg(), 50_000.0);
    }

    #[test]
    fn rocket_total_mass() {
        let rocket = simple_two_stage();
        let total = rocket.total_mass();

        // Should be sum of all stage wet masses + payload
        let expected =
            rocket.stages[0].wet_mass().as_kg() + rocket.stages[1].wet_mass().as_kg() + 50_000.0;

        assert!((total.as_kg() - expected).abs() < 1.0);
    }

    #[test]
    fn rocket_mass_above_stage() {
        let rocket = simple_two_stage();

        // Mass above stage 0 = stage 1 wet mass + payload
        let above_0 = rocket.mass_above_stage(0);
        let expected_0 = rocket.stages[1].wet_mass().as_kg() + 50_000.0;
        assert!((above_0.as_kg() - expected_0).abs() < 1.0);

        // Mass above stage 1 = payload only
        let above_1 = rocket.mass_above_stage(1);
        assert_eq!(above_1.as_kg(), 50_000.0);
    }

    #[test]
    fn rocket_total_delta_v() {
        let rocket = simple_two_stage();
        let total_dv = rocket.total_delta_v();

        // With payload, total delta-v is reduced compared to isolated stages
        // Expected ~9,000-10,000 m/s for this configuration
        assert!(total_dv.as_mps() > 8_000.0);
        assert!(total_dv.as_mps() < 12_000.0);
    }

    #[test]
    fn rocket_payload_fraction() {
        let rocket = simple_two_stage();
        let fraction = rocket.payload_fraction();

        // Payload fraction should be reasonable (1-10%)
        assert!(fraction.as_f64() > 0.01);
        assert!(fraction.as_f64() < 0.10);
    }

    #[test]
    fn rocket_liftoff_twr() {
        let rocket = simple_two_stage();
        let twr = rocket.liftoff_twr();

        // Should have positive TWR > 1.0 for liftoff
        assert!(twr.as_f64() > 1.0);
    }

    #[test]
    fn rocket_stage_twr() {
        let rocket = simple_two_stage();

        // Upper stage TWR should be reasonable
        let upper_twr = rocket.stage_twr(1);
        assert!(upper_twr.as_f64() > 0.5);
    }

    #[test]
    fn rocket_validate_twr_passes() {
        let rocket = simple_two_stage();
        let result = rocket.validate_twr(Ratio::new(0.5), true);
        assert!(result.is_ok());
    }

    #[test]
    fn rocket_validate_twr_fails_low_min() {
        // Create a rocket with very low TWR
        let stage1 = Stage::with_structural_ratio(get_merlin(), 1, Mass::kg(1_000_000.0), 0.05);
        let rocket = Rocket::new(vec![stage1], Mass::kg(50_000.0));

        // This should fail - single Merlin can't lift this mass
        let result = rocket.validate_twr(Ratio::new(1.0), true);
        assert!(result.is_err());
    }

    #[test]
    fn rocket_total_burn_time() {
        let rocket = simple_two_stage();
        let burn_time = rocket.total_burn_time();

        // Should be several minutes total
        assert!(burn_time.as_seconds() > 100.0);
        assert!(burn_time.as_seconds() < 1000.0);
    }

    #[test]
    #[should_panic(expected = "must have at least one stage")]
    fn rocket_empty_stages_panics() {
        Rocket::new(vec![], Mass::kg(1000.0));
    }
}
