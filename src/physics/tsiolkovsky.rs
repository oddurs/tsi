//! Tsiolkovsky rocket equation implementation.
//!
//! The Tsiolkovsky rocket equation, derived by Konstantin Tsiolkovsky in 1903,
//! is the fundamental equation of astronautics. It describes the relationship
//! between a rocket's change in velocity (delta-v) and the mass of propellant
//! consumed.
//!
//! # The Equation
//!
//! ```text
//! Δv = Isp × g₀ × ln(m₀/m₁)
//! ```
//!
//! Where:
//! - **Δv** (delta-v): Change in velocity the rocket can achieve (m/s)
//! - **Isp**: Specific impulse - a measure of engine efficiency (seconds)
//! - **g₀**: Standard gravity constant (9.80665 m/s²)
//! - **m₀**: Initial "wet" mass (propellant + structure + payload)
//! - **m₁**: Final "dry" mass (structure + payload, propellant exhausted)
//! - **m₀/m₁**: Mass ratio - always ≥ 1
//!
//! # Physical Intuition
//!
//! The equation comes from conservation of momentum. As a rocket expels mass
//! (exhaust gases) in one direction, it accelerates in the opposite direction.
//! The logarithm appears because as the rocket gets lighter, each kilogram of
//! propellant produces more acceleration - but with diminishing returns.
//!
//! # Practical Implications
//!
//! - **Mass ratio is exponential**: To double delta-v, you need to square the
//!   mass ratio (e.g., from 3:1 to 9:1), which is why rockets are mostly fuel.
//! - **Isp is linear**: Doubling Isp doubles delta-v, making efficient engines
//!   extremely valuable.
//! - **Staging helps**: By discarding empty tanks, each stage starts with a
//!   better mass ratio than a single-stage vehicle could achieve.

use crate::units::{Isp, Ratio, Velocity};

use super::G0;

/// Calculate delta-v using the Tsiolkovsky rocket equation.
///
/// This is the primary equation of astronautics, relating a rocket's
/// performance to its mass ratio and engine efficiency.
///
/// # Formula
///
/// ```text
/// Δv = Isp × g₀ × ln(mass_ratio)
/// ```
///
/// # Arguments
///
/// * `isp` - Specific impulse of the engine in seconds. Higher is better.
///   Typical values: 300-350s (kerosene), 380-450s (hydrogen).
/// * `mass_ratio` - Ratio of wet mass to dry mass (m₀/m₁). Must be ≥ 1.
///   A mass ratio of 10 means the rocket is 90% propellant by mass.
///
/// # Returns
///
/// The theoretical delta-v achievable in a vacuum with no gravity losses.
///
/// # Examples
///
/// ```
/// use tsi::units::{Isp, Ratio};
/// use tsi::physics::delta_v;
///
/// // A stage with Isp 350s and mass ratio 8 (87.5% propellant)
/// let dv = delta_v(Isp::seconds(350.0), Ratio::new(8.0));
/// // Δv = 350 × 9.80665 × ln(8) ≈ 7,138 m/s
/// assert!(dv.as_mps() > 7_000.0);
/// ```
///
/// # Real-World Context
///
/// - Low Earth Orbit requires ~9,400 m/s total delta-v
/// - Falcon 9 first stage: ~8,500 m/s ideal delta-v
/// - The Moon landing (Apollo): ~15,000 m/s total mission delta-v
pub fn delta_v(isp: Isp, mass_ratio: Ratio) -> Velocity {
    // The natural logarithm captures the diminishing returns of adding more
    // propellant: each additional kg of fuel also adds weight that must be
    // accelerated, so you never get linear scaling.
    Velocity::mps(isp.as_seconds() * G0 * mass_ratio.as_f64().ln())
}

/// Calculate the required mass ratio to achieve a given delta-v.
///
/// This is the inverse of the Tsiolkovsky equation, useful for mission
/// planning: "Given my engine and required delta-v, how much propellant
/// do I need?"
///
/// # Formula
///
/// ```text
/// mass_ratio = e^(Δv / (Isp × g₀))
/// ```
///
/// # Arguments
///
/// * `dv` - Target delta-v to achieve
/// * `isp` - Specific impulse of the engine
///
/// # Returns
///
/// The mass ratio required. Values above ~20 are very challenging to build.
///
/// # Examples
///
/// ```
/// use tsi::units::{Isp, Velocity};
/// use tsi::physics::required_mass_ratio;
///
/// // What mass ratio do we need for 3,000 m/s with a 350s Isp engine?
/// let ratio = required_mass_ratio(Velocity::mps(3000.0), Isp::seconds(350.0));
/// // e^(3000 / (350 × 9.80665)) ≈ 2.41
/// assert!(ratio.as_f64() > 2.0 && ratio.as_f64() < 3.0);
/// ```
///
/// # Design Implications
///
/// - Mass ratio < 5: Relatively easy to build
/// - Mass ratio 5-10: Challenging but achievable (most orbital rockets)
/// - Mass ratio 10-20: Very difficult, requires advanced materials
/// - Mass ratio > 20: Essentially impossible with current technology
pub fn required_mass_ratio(dv: Velocity, isp: Isp) -> Ratio {
    // This is simply solving Δv = Isp × g₀ × ln(R) for R:
    // ln(R) = Δv / (Isp × g₀)
    // R = e^(Δv / (Isp × g₀))
    Ratio::new((dv.as_mps() / (isp.as_seconds() * G0)).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn delta_v_basic() {
        // Δv = Isp × g₀ × ln(mass_ratio)
        // With Isp = 300s and mass_ratio = e (2.718...):
        // Δv = 300 × 9.80665 × 1 = 2941.995 m/s
        let isp = Isp::seconds(300.0);
        let ratio = Ratio::new(std::f64::consts::E);
        let dv = delta_v(isp, ratio);
        assert_relative_eq!(dv.as_mps(), 300.0 * G0, epsilon = 0.01);
    }

    #[test]
    fn delta_v_mass_ratio_one() {
        // ln(1) = 0, so Δv should be 0
        let isp = Isp::seconds(350.0);
        let ratio = Ratio::new(1.0);
        let dv = delta_v(isp, ratio);
        assert_eq!(dv.as_mps(), 0.0);
    }

    #[test]
    fn delta_v_high_mass_ratio() {
        // Sanity check with mass ratio of 10
        let isp = Isp::seconds(450.0);
        let ratio = Ratio::new(10.0);
        let dv = delta_v(isp, ratio);
        let expected = 450.0 * G0 * 10.0_f64.ln();
        assert_relative_eq!(dv.as_mps(), expected, epsilon = 0.01);
    }

    #[test]
    fn required_mass_ratio_inverse() {
        // Verify that required_mass_ratio is the inverse of delta_v
        let isp = Isp::seconds(320.0);
        let original_ratio = Ratio::new(5.0);
        let dv = delta_v(isp, original_ratio);
        let recovered_ratio = required_mass_ratio(dv, isp);
        assert_relative_eq!(recovered_ratio.as_f64(), 5.0, epsilon = 0.0001);
    }

    #[test]
    fn required_mass_ratio_zero_dv() {
        let isp = Isp::seconds(300.0);
        let dv = Velocity::mps(0.0);
        let ratio = required_mass_ratio(dv, isp);
        assert_eq!(ratio.as_f64(), 1.0);
    }

    #[test]
    fn falcon_9_stage_1_delta_v() {
        // Falcon 9 v1.2 (Block 5) first stage
        // Propellant: ~411,000 kg, Dry mass: ~22,200 kg
        // Isp: ~297s average for ascent
        let propellant = 411_000.0;
        let dry = 22_200.0;
        let wet = propellant + dry;
        let mass_ratio = Ratio::new(wet / dry);
        let isp = Isp::seconds(297.0);

        let dv = delta_v(isp, mass_ratio);

        // Ideal delta-v should be around 8,500-9,000 m/s
        assert!(dv.as_mps() > 8000.0);
        assert!(dv.as_mps() < 10000.0);
    }

    #[test]
    fn falcon_9_stage_2_delta_v() {
        // Single Merlin Vacuum
        // Propellant: ~111,500 kg, Dry mass: ~4,000 kg
        // Isp: 348s (vacuum)
        let propellant = 111_500.0;
        let dry = 4_000.0;
        let wet = propellant + dry;
        let mass_ratio = Ratio::new(wet / dry);
        let isp = Isp::seconds(348.0);

        let dv = delta_v(isp, mass_ratio);

        // Should be around 11,000-12,000 m/s ideal
        assert!(dv.as_mps() > 10000.0);
        assert!(dv.as_mps() < 13000.0);
    }
}
