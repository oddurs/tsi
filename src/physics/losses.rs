//! Atmospheric and gravity loss estimation.
//!
//! Real rocket launches lose delta-v to two main sources:
//!
//! 1. **Gravity losses** - Fighting Earth's gravity during vertical ascent
//! 2. **Drag losses** - Air resistance during atmospheric flight
//!
//! These losses reduce the effective delta-v available for orbit insertion.
//!
//! # Estimation Approach
//!
//! Full trajectory simulation requires numerical integration (ODE solvers),
//! but useful estimates can be derived from empirical models based on
//! burn time, thrust-to-weight ratio, and vehicle characteristics.
//!
//! # Typical Values (Earth to LEO)
//!
//! | Loss Type | Range | Notes |
//! |-----------|-------|-------|
//! | Gravity | 1,000-1,800 m/s | Higher for low-TWR vehicles |
//! | Drag | 50-400 m/s | Higher for dense vehicles |
//! | Steering | 50-150 m/s | Pitch/yaw maneuvering |
//!
//! # Example
//!
//! ```
//! use tsiolkovsky::physics::losses::total_losses;
//! use tsiolkovsky::units::{Time, Ratio};
//!
//! // First stage: 200s burn, TWR 1.3
//! let estimate = total_losses(Time::seconds(200.0), Ratio::new(1.3));
//! println!("Gravity loss: {}", estimate.gravity);
//! println!("Drag loss: {}", estimate.drag);
//! println!("Total: {}", estimate.total());
//! ```
//!
//! # References
//!
//! - Humble, R. et al. "Space Propulsion Analysis and Design" (1995)
//! - Sutton, G. "Rocket Propulsion Elements" (8th ed.)

use crate::stage::Rocket;
use crate::units::{Ratio, Time, Velocity};

use super::G0;

/// Orbital velocity in a 200 km circular low Earth orbit, m/s.
const LEO_ORBITAL_VELOCITY_MPS: f64 = 7_800.0;

/// Margin added on top of losses in [`leo_delta_v_requirement`], m/s.
const LEO_MARGIN_MPS: f64 = 150.0;

/// Estimated delta-v losses for a launch to low Earth orbit.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LossEstimate {
    /// Delta-v spent holding the rocket up against gravity
    pub gravity: Velocity,
    /// Delta-v lost to air resistance
    pub drag: Velocity,
    /// Delta-v spent turning
    pub steering: Velocity,
}

impl LossEstimate {
    /// A loss estimate from its components.
    pub fn new(gravity: Velocity, drag: Velocity, steering: Velocity) -> Self {
        Self {
            gravity,
            drag,
            steering,
        }
    }

    /// No losses.
    pub fn zero() -> Self {
        let zero = Velocity::mps(0.0);
        Self::new(zero, zero, zero)
    }

    /// All losses together.
    pub fn total(&self) -> Velocity {
        self.gravity + self.drag + self.steering
    }
}

/// Estimate gravity losses during a first stage's ascent from Earth.
///
/// Gravity loss is the delta-v spent holding the rocket up rather than
/// speeding it along. While the rocket climbs at flight path angle θ,
/// gravity takes g·sin θ per second from it:
///
/// ```text
/// Δv_gravity = ∫ g · sin θ dt ≈ g₀ · t_burn · ⟨sin θ⟩
/// ```
///
/// A rocket with more thrust per unit weight pitches over sooner, so the
/// average climb angle falls as liftoff TWR rises.
///
/// # Model and origin
///
/// tsi uses ⟨sin θ⟩ ≈ 0.85 / √TWR. This is an **empirical fit, not a
/// derivation**: it was chosen so that typical first stages (150-200 s burns,
/// liftoff TWR 1.2-1.5) come out in the 1,000-1,500 m/s range commonly quoted
/// for launches to low Earth orbit. TWR is clamped to 1-10.
///
/// # Where it breaks down
///
/// - It knows nothing about the actual trajectory: a lofted or depressed
///   ascent changes the answer by hundreds of m/s.
/// - It covers the first stage only. Upper stages also lose delta-v to
///   gravity, typically a few hundred m/s, but how much depends on the
///   trajectory's shape, which a closed-form model can't know. Trajectory
///   simulation (planned after 1.0) will cover every stage.
/// - It assumes Earth's gravity.
///
/// ```
/// use tsiolkovsky::physics::losses::gravity_loss;
/// use tsiolkovsky::units::{Ratio, Time};
///
/// let loss = gravity_loss(Time::seconds(150.0), Ratio::new(1.3));
/// assert!((1_000.0..1_200.0).contains(&loss.as_mps()));
/// ```
pub fn gravity_loss(burn_time: Time, liftoff_twr: Ratio) -> Velocity {
    let twr = liftoff_twr.as_f64().clamp(1.0, 10.0);
    Velocity::mps(G0 * burn_time.as_seconds() * 0.85 / twr.sqrt())
}

/// Estimate atmospheric drag losses during ascent from Earth.
///
/// # Model and origin
///
/// ```text
/// Δv_drag ≈ 150 m/s × (1 + 0.5 / TWR)
/// ```
///
/// An **empirical fit**: a 150 m/s baseline for a brisk climb, rising toward
/// 225 m/s for a rocket that lingers in the dense lower atmosphere. It sits
/// inside the 100-400 m/s range quoted for medium and heavy launchers (Humble
/// et al., *Space Propulsion Analysis and Design*, 1995). TWR is clamped to 1-10.
///
/// # Where it breaks down
///
/// Drag really depends on the rocket's size, shape and mass (its ballistic
/// coefficient), which this ignores. A small, light rocket such as Electron
/// loses proportionally more to drag than a Saturn V; this model gives them
/// the same.
pub fn drag_loss(liftoff_twr: Ratio) -> Velocity {
    let twr = liftoff_twr.as_f64().clamp(1.0, 10.0);
    Velocity::mps(150.0 * (1.0 + 0.5 / twr))
}

/// Estimate steering losses during ascent: a flat 100 m/s.
///
/// Steering losses come from pointing thrust away from the velocity vector,
/// for the gravity turn, guidance corrections, and any dog-leg to change
/// inclination. 100 m/s is the middle of the 50-150 m/s usually quoted for a
/// direct ascent to low Earth orbit. It is not a model: a launch that has to
/// change plane can lose far more.
pub fn steering_loss() -> Velocity {
    Velocity::mps(100.0)
}

/// Estimated losses for an Earth-to-orbit ascent, from the first stage's
/// burn time and liftoff TWR.
///
/// ```
/// use tsiolkovsky::physics::losses::total_losses;
/// use tsiolkovsky::units::{Ratio, Time};
///
/// // Falcon 9: a ~170 s first stage burn, liftoff TWR ~1.28
/// let losses = total_losses(Time::seconds(170.0), Ratio::new(1.28));
/// assert!((1_400.0..2_000.0).contains(&losses.total().as_mps()));
/// ```
pub fn total_losses(first_stage_burn: Time, liftoff_twr: Ratio) -> LossEstimate {
    LossEstimate::new(
        gravity_loss(first_stage_burn, liftoff_twr),
        drag_loss(liftoff_twr),
        steering_loss(),
    )
}

/// Estimated ascent losses for a rocket, from its first stage.
pub fn ascent_losses(rocket: &Rocket) -> LossEstimate {
    let first = &rocket.stages()[0];
    total_losses(first.burn_time(), rocket.liftoff_twr())
}

/// Delta-v a rocket needs to reach low Earth orbit from sea level: orbital
/// velocity (7,800 m/s), plus estimated losses, plus 150 m/s of margin.
///
/// This is where the usual "9,400 m/s to LEO" comes from.
pub fn leo_delta_v_requirement(first_stage_burn: Time, liftoff_twr: Ratio) -> Velocity {
    Velocity::mps(LEO_ORBITAL_VELOCITY_MPS + LEO_MARGIN_MPS)
        + total_losses(first_stage_burn, liftoff_twr).total()
}

/// Orbital velocity in a 200 km circular low Earth orbit.
pub fn leo_orbital_velocity() -> Velocity {
    Velocity::mps(LEO_ORBITAL_VELOCITY_MPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_loss_reasonable_range() {
        // Typical first stage burn: 150-200s, TWR 1.2-1.5
        let burn = Time::seconds(170.0);
        let twr = Ratio::new(1.3);

        let loss = gravity_loss(burn, twr).as_mps();

        // Should be in the 1000-1500 m/s range
        assert!(
            loss > 1000.0 && loss < 1500.0,
            "gravity loss {} out of expected range",
            loss
        );
    }

    #[test]
    fn gravity_loss_increases_with_burn_time() {
        let twr = Ratio::new(1.3);

        let loss_short = gravity_loss(Time::seconds(100.0), twr).as_mps();
        let loss_long = gravity_loss(Time::seconds(200.0), twr).as_mps();

        assert!(
            loss_long > loss_short,
            "longer burn should have more gravity loss"
        );
    }

    #[test]
    fn gravity_loss_decreases_with_higher_twr() {
        let burn = Time::seconds(150.0);

        let loss_low_twr = gravity_loss(burn, Ratio::new(1.2)).as_mps();
        let loss_high_twr = gravity_loss(burn, Ratio::new(1.8)).as_mps();

        assert!(
            loss_high_twr < loss_low_twr,
            "higher TWR should have less gravity loss"
        );
    }

    #[test]
    fn drag_loss_reasonable_range() {
        let loss = drag_loss(Ratio::new(1.3)).as_mps();

        // Should be in the 150-250 m/s range
        assert!(
            loss > 100.0 && loss < 300.0,
            "drag loss {} out of expected range",
            loss
        );
    }

    #[test]
    fn drag_loss_decreases_with_higher_twr() {
        let loss_low = drag_loss(Ratio::new(1.2)).as_mps();
        let loss_high = drag_loss(Ratio::new(2.0)).as_mps();

        assert!(
            loss_high < loss_low,
            "higher TWR should have less drag loss"
        );
    }

    #[test]
    fn total_losses_falcon9_like() {
        // Falcon 9 first stage: ~170s burn, ~1.28 TWR
        let losses = total_losses(Time::seconds(170.0), Ratio::new(1.28));

        // Total should be around 1500-1800 m/s
        assert!(
            (1400.0..2000.0).contains(&losses.total().as_mps()),
            "total losses {} out of expected range",
            losses.total()
        );
    }

    #[test]
    fn leo_dv_requirement_reasonable() {
        let dv = leo_delta_v_requirement(Time::seconds(170.0), Ratio::new(1.3)).as_mps();

        // LEO typically needs 9,200-9,600 m/s
        assert!(
            dv > 9200.0 && dv < 9800.0,
            "LEO delta-v {} out of expected range",
            dv
        );
    }

    #[test]
    fn loss_estimate_components_sum() {
        let estimate = LossEstimate::new(
            Velocity::mps(1000.0),
            Velocity::mps(200.0),
            Velocity::mps(100.0),
        );
        assert!((estimate.total().as_mps() - 1300.0).abs() < 1e-9);
        assert_eq!(LossEstimate::zero().total().as_mps(), 0.0);
    }
}
