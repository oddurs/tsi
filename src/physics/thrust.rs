//! Thrust-related calculations: TWR and burn time.
//!
//! # Thrust-to-Weight Ratio (TWR)
//!
//! TWR is a dimensionless ratio that determines whether a rocket can lift off
//! and how aggressively it can accelerate. It's defined as:
//!
//! ```text
//! TWR = Thrust / Weight = F / (m × g)
//! ```
//!
//! ## TWR Requirements
//!
//! - **TWR > 1.0**: Required to lift off from a surface. Below this, the
//!   rocket cannot overcome gravity.
//! - **TWR ≈ 1.2-1.5**: Typical for first stages. Higher TWR means faster
//!   acceleration but wastes propellant fighting gravity longer.
//! - **TWR < 1.0**: Acceptable for upper stages that are already moving and
//!   in near-vacuum. Many upper stages have TWR of 0.5-0.8.
//!
//! # Burn Time
//!
//! Burn time is how long the engine can fire before exhausting propellant:
//!
//! ```text
//! t = m_propellant / ṁ
//! ```
//!
//! Where ṁ (mass flow rate) = F / (Isp × g₀)
//!
//! Higher Isp engines have lower mass flow rates for the same thrust,
//! resulting in longer burn times with the same propellant load.

use crate::units::{Force, Isp, Mass, Ratio, Time};

use super::G0;

/// Calculate thrust-to-weight ratio (TWR).
///
/// TWR determines a rocket's ability to overcome gravity and accelerate.
/// A TWR > 1.0 is required for liftoff; higher values mean faster acceleration.
///
/// # Arguments
///
/// * `thrust` - Engine thrust force
/// * `mass` - Total mass being accelerated (stage + payload + propellant)
/// * `gravity` - Local gravitational acceleration (m/s²). Use `G0` for Earth
///   surface, or lower values for other bodies (Moon: 1.62, Mars: 3.72).
///
/// # Returns
///
/// A dimensionless ratio. Values > 1.0 indicate the rocket can accelerate
/// upward against gravity.
///
/// # Examples
///
/// ```
/// use tsi::units::{Force, Mass};
/// use tsi::physics::{twr, G0};
///
/// // Falcon 9 at liftoff: 9 Merlin engines, 549 tonnes total mass
/// let thrust = Force::newtons(9.0 * 845_000.0); // ~7.6 MN
/// let mass = Mass::kg(549_000.0);
/// let ratio = twr(thrust, mass, G0);
/// assert!((ratio.as_f64() - 1.4).abs() < 0.1); // TWR ≈ 1.4
/// ```
///
/// # Design Considerations
///
/// - First stages typically target TWR 1.2-1.5 at liftoff
/// - Too high: Wastes propellant accelerating quickly through dense atmosphere
/// - Too low: Spends too long fighting gravity (gravity losses)
/// - TWR increases during flight as propellant is consumed
pub fn twr(thrust: Force, mass: Mass, gravity: f64) -> Ratio {
    // Weight = mass × gravity
    // TWR = thrust / weight
    Ratio::new(thrust.as_newtons() / (mass.as_kg() * gravity))
}

/// Calculate burn time from propellant mass, thrust, and Isp.
///
/// Determines how long an engine can fire before exhausting its propellant.
/// This assumes constant thrust (no throttling) and complete propellant
/// consumption (no reserves).
///
/// # Formula
///
/// ```text
/// burn_time = propellant_mass / mass_flow_rate
/// mass_flow_rate = thrust / (Isp × g₀)
/// ```
///
/// Combined: `t = (m_prop × Isp × g₀) / F`
///
/// # Arguments
///
/// * `propellant` - Mass of propellant available
/// * `thrust` - Engine thrust force
/// * `isp` - Specific impulse of the engine
///
/// # Returns
///
/// Time until propellant is exhausted.
///
/// # Examples
///
/// ```
/// use tsi::units::{Force, Isp, Mass};
/// use tsi::physics::burn_time;
///
/// // Merlin-1D: 845 kN thrust, 311s Isp, 45 tonnes propellant
/// let time = burn_time(
///     Mass::kg(45_000.0),
///     Force::newtons(845_000.0),
///     Isp::seconds(311.0)
/// );
/// assert!((time.as_seconds() - 162.0).abs() < 5.0); // ~2.7 minutes
/// ```
///
/// # Physical Insight
///
/// Higher Isp engines achieve the same thrust with less mass flow, so they
/// burn longer with the same propellant. This is why hydrogen engines
/// (Isp ~450s) have much longer burn times than kerosene engines (Isp ~310s)
/// for similar thrust levels.
pub fn burn_time(propellant: Mass, thrust: Force, isp: Isp) -> Time {
    // Mass flow rate: how quickly propellant is consumed
    // ṁ = F / v_e = F / (Isp × g₀)
    let mass_flow = thrust.as_newtons() / (isp.as_seconds() * G0);

    // Burn time: total propellant divided by consumption rate
    Time::seconds(propellant.as_kg() / mass_flow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn twr_calculation() {
        // 1,000,000 N thrust, 100,000 kg mass, Earth gravity
        let thrust = Force::newtons(1_000_000.0);
        let mass = Mass::kg(100_000.0);
        let ratio = twr(thrust, mass, G0);
        assert_relative_eq!(ratio.as_f64(), 1.0197, epsilon = 0.001);
    }

    #[test]
    fn twr_exactly_one() {
        // Thrust equals weight
        let mass = Mass::kg(1000.0);
        let weight = 1000.0 * G0;
        let thrust = Force::newtons(weight);
        let ratio = twr(thrust, mass, G0);
        assert_relative_eq!(ratio.as_f64(), 1.0, epsilon = 0.0001);
    }

    #[test]
    fn twr_high_thrust() {
        // 9 × Merlin-1D at liftoff
        let thrust = Force::newtons(845_000.0 * 9.0); // 7.6 MN
        let mass = Mass::kg(550_000.0); // Falcon 9 liftoff mass
        let ratio = twr(thrust, mass, G0);
        // Should be around 1.4
        assert!(ratio.as_f64() > 1.3);
        assert!(ratio.as_f64() < 1.5);
    }

    #[test]
    fn burn_time_calculation() {
        // 10,000 kg propellant, 100,000 N thrust, 300s Isp
        // mass_flow = 100,000 / (300 × 9.80665) = 33.99 kg/s
        // burn_time = 10,000 / 33.99 = 294.2 s
        let propellant = Mass::kg(10_000.0);
        let thrust = Force::newtons(100_000.0);
        let isp = Isp::seconds(300.0);
        let time = burn_time(propellant, thrust, isp);
        assert_relative_eq!(time.as_seconds(), 294.2, epsilon = 0.5);
    }

    #[test]
    fn burn_time_merlin() {
        // Single Merlin-1D: 845 kN thrust, 311s Isp (vac)
        // With 45,000 kg propellant
        let propellant = Mass::kg(45_000.0);
        let thrust = Force::newtons(845_000.0);
        let isp = Isp::seconds(311.0);
        let time = burn_time(propellant, thrust, isp);

        // mass_flow = 845000 / (311 × 9.80665) ≈ 277 kg/s
        // burn_time = 45000 / 277 ≈ 162 s
        assert_relative_eq!(time.as_seconds(), 162.0, epsilon = 2.0);
    }

    #[test]
    fn burn_time_high_isp() {
        // RL-10C: 106 kN thrust, 453s Isp
        let propellant = Mass::kg(20_000.0);
        let thrust = Force::newtons(106_000.0);
        let isp = Isp::seconds(453.0);
        let time = burn_time(propellant, thrust, isp);

        // Higher Isp means lower mass flow, longer burn
        // mass_flow = 106000 / (453 × 9.80665) ≈ 23.9 kg/s
        // burn_time = 20000 / 23.9 ≈ 838 s
        assert_relative_eq!(time.as_seconds(), 838.0, epsilon = 5.0);
    }
}
