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
//! use tsi::physics::losses::{gravity_loss, drag_loss, total_losses, LossEstimate};
//! use tsi::units::{Mass, Force, Time, Ratio};
//!
//! // First stage: 200s burn, TWR 1.3
//! let burn_time = Time::seconds(200.0);
//! let twr = Ratio::new(1.3);
//!
//! // Estimate losses
//! let estimate = total_losses(burn_time, twr);
//! println!("Gravity loss: {} m/s", estimate.gravity_loss_mps);
//! println!("Drag loss: {} m/s", estimate.drag_loss_mps);
//! println!("Total: {} m/s", estimate.total_loss_mps);
//! ```
//!
//! # References
//!
//! - Humble, R. et al. "Space Propulsion Analysis and Design" (1995)
//! - Sutton, G. "Rocket Propulsion Elements" (8th ed.)

use crate::units::{Ratio, Time};

/// Estimated delta-v losses for a launch.
#[derive(Debug, Clone, Copy)]
pub struct LossEstimate {
    /// Gravity drag loss in m/s.
    pub gravity_loss_mps: f64,

    /// Atmospheric drag loss in m/s.
    pub drag_loss_mps: f64,

    /// Steering/maneuvering loss in m/s.
    pub steering_loss_mps: f64,

    /// Total losses (sum of all components) in m/s.
    pub total_loss_mps: f64,
}

impl LossEstimate {
    /// Create a new loss estimate from components.
    pub fn new(gravity: f64, drag: f64, steering: f64) -> Self {
        Self {
            gravity_loss_mps: gravity,
            drag_loss_mps: drag,
            steering_loss_mps: steering,
            total_loss_mps: gravity + drag + steering,
        }
    }

    /// Create a zero loss estimate.
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

/// Estimate gravity losses during ascent.
///
/// Gravity loss is the delta-v spent fighting gravity during the vertical
/// portion of ascent. It depends on:
///
/// - **Burn time**: Longer burns mean more time fighting gravity
/// - **Initial TWR**: Higher TWR means faster acceleration, less gravity loss
///
/// # Model
///
/// Uses an empirical model:
/// ```text
/// Δv_gravity ≈ g₀ × t_burn × sin(θ_avg)
/// ```
///
/// Where θ_avg is the average pitch angle during burn. For a typical
/// gravity turn, this is approximately:
/// ```text
/// sin(θ_avg) ≈ 0.85 / sqrt(TWR)
/// ```
///
/// # Arguments
///
/// * `burn_time` - Total burn time for the first stage
/// * `twr` - Initial thrust-to-weight ratio at liftoff
///
/// # Returns
///
/// Estimated gravity loss in m/s.
///
/// # Example
///
/// ```
/// use tsi::physics::losses::gravity_loss;
/// use tsi::units::{Time, Ratio};
///
/// let burn = Time::seconds(150.0);
/// let twr = Ratio::new(1.3);
///
/// let loss = gravity_loss(burn, twr);
/// println!("Gravity loss: {:.0} m/s", loss);  // ~1,100 m/s
/// ```
pub fn gravity_loss(burn_time: Time, twr: Ratio) -> f64 {
    const G0: f64 = 9.80665;

    // Clamp TWR to reasonable range to avoid numerical issues
    let twr_val = twr.as_f64().clamp(1.0, 10.0);

    // Empirical model: average sin(pitch) decreases with higher TWR
    // Higher TWR means faster pitchover, less time vertical
    let avg_sin_pitch = 0.85 / twr_val.sqrt();

    G0 * burn_time.as_seconds() * avg_sin_pitch
}

/// Estimate atmospheric drag losses.
///
/// Drag loss depends on:
///
/// - **Vehicle size and shape** (ballistic coefficient)
/// - **Velocity through atmosphere**
/// - **Time in dense atmosphere**
///
/// # Model
///
/// Uses a simplified empirical model based on typical launch vehicles:
/// ```text
/// Δv_drag ≈ 150 × (1 + 0.5 / TWR)
/// ```
///
/// This accounts for:
/// - Baseline drag of ~150 m/s for high-TWR vehicles
/// - Increased drag for low-TWR vehicles (longer time in atmosphere)
///
/// # Arguments
///
/// * `twr` - Initial thrust-to-weight ratio at liftoff
///
/// # Returns
///
/// Estimated drag loss in m/s.
///
/// # Limitations
///
/// This model does not account for:
/// - Vehicle-specific drag coefficients
/// - Fairing size and shape
/// - Launch site altitude
///
/// For more accurate estimates, trajectory simulation is required.
pub fn drag_loss(twr: Ratio) -> f64 {
    // Clamp TWR to reasonable range
    let twr_val = twr.as_f64().clamp(1.0, 10.0);

    // Empirical model: higher TWR = faster through max-q, less drag
    // Baseline ~150 m/s, up to ~250 m/s for low-TWR vehicles
    150.0 * (1.0 + 0.5 / twr_val)
}

/// Estimate steering losses during ascent.
///
/// Steering losses come from:
/// - Gravity turn maneuvering
/// - Pitch/yaw corrections
/// - Dog-leg maneuvers for inclination changes
///
/// For direct ascent to LEO, this is typically 50-150 m/s.
///
/// # Arguments
///
/// * `_burn_time` - Total burn time (currently unused, for future refinement)
///
/// # Returns
///
/// Estimated steering loss in m/s.
pub fn steering_loss(_burn_time: Time) -> f64 {
    // Conservative estimate for standard gravity turn
    100.0
}

/// Calculate total estimated losses for Earth to LEO ascent.
///
/// Combines gravity, drag, and steering losses into a single estimate.
///
/// # Arguments
///
/// * `first_stage_burn` - Burn time of the first stage
/// * `liftoff_twr` - Thrust-to-weight ratio at liftoff
///
/// # Returns
///
/// A [`LossEstimate`] containing individual and total losses.
///
/// # Example
///
/// ```
/// use tsi::physics::losses::total_losses;
/// use tsi::units::{Time, Ratio};
///
/// let burn = Time::seconds(170.0);  // Falcon 9 first stage
/// let twr = Ratio::new(1.28);       // F9 liftoff TWR
///
/// let losses = total_losses(burn, twr);
/// println!("Total losses: {:.0} m/s", losses.total_loss_mps);
/// // Approximately 1,500-1,700 m/s
/// ```
pub fn total_losses(first_stage_burn: Time, liftoff_twr: Ratio) -> LossEstimate {
    let gravity = gravity_loss(first_stage_burn, liftoff_twr);
    let drag = drag_loss(liftoff_twr);
    let steering = steering_loss(first_stage_burn);

    LossEstimate::new(gravity, drag, steering)
}

/// Estimate required delta-v for LEO (Low Earth Orbit) from sea level.
///
/// LEO requires approximately 9,400 m/s of delta-v:
/// - Orbital velocity: ~7,800 m/s
/// - Gravity losses: ~1,200 m/s
/// - Drag losses: ~150 m/s
/// - Steering losses: ~100 m/s
/// - Margin: ~150 m/s
///
/// # Arguments
///
/// * `first_stage_burn` - First stage burn time
/// * `liftoff_twr` - Thrust-to-weight ratio at liftoff
///
/// # Returns
///
/// Required delta-v in m/s for LEO insertion.
pub fn leo_delta_v_requirement(first_stage_burn: Time, liftoff_twr: Ratio) -> f64 {
    const ORBITAL_VELOCITY_LEO: f64 = 7_800.0;
    const MARGIN: f64 = 150.0;

    let losses = total_losses(first_stage_burn, liftoff_twr);

    ORBITAL_VELOCITY_LEO + losses.total_loss_mps + MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_loss_reasonable_range() {
        // Typical first stage burn: 150-200s, TWR 1.2-1.5
        let burn = Time::seconds(170.0);
        let twr = Ratio::new(1.3);

        let loss = gravity_loss(burn, twr);

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

        let loss_short = gravity_loss(Time::seconds(100.0), twr);
        let loss_long = gravity_loss(Time::seconds(200.0), twr);

        assert!(
            loss_long > loss_short,
            "longer burn should have more gravity loss"
        );
    }

    #[test]
    fn gravity_loss_decreases_with_higher_twr() {
        let burn = Time::seconds(150.0);

        let loss_low_twr = gravity_loss(burn, Ratio::new(1.2));
        let loss_high_twr = gravity_loss(burn, Ratio::new(1.8));

        assert!(
            loss_high_twr < loss_low_twr,
            "higher TWR should have less gravity loss"
        );
    }

    #[test]
    fn drag_loss_reasonable_range() {
        let loss = drag_loss(Ratio::new(1.3));

        // Should be in the 150-250 m/s range
        assert!(
            loss > 100.0 && loss < 300.0,
            "drag loss {} out of expected range",
            loss
        );
    }

    #[test]
    fn drag_loss_decreases_with_higher_twr() {
        let loss_low = drag_loss(Ratio::new(1.2));
        let loss_high = drag_loss(Ratio::new(2.0));

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
            losses.total_loss_mps > 1400.0 && losses.total_loss_mps < 2000.0,
            "total losses {} out of expected range",
            losses.total_loss_mps
        );
    }

    #[test]
    fn leo_dv_requirement_reasonable() {
        let dv = leo_delta_v_requirement(Time::seconds(170.0), Ratio::new(1.3));

        // LEO typically needs 9,200-9,600 m/s
        assert!(
            dv > 9200.0 && dv < 9800.0,
            "LEO delta-v {} out of expected range",
            dv
        );
    }

    #[test]
    fn loss_estimate_components_sum() {
        let estimate = LossEstimate::new(1000.0, 200.0, 100.0);

        assert!(
            (estimate.total_loss_mps - 1300.0).abs() < 0.001,
            "total should be sum of components"
        );
    }
}
