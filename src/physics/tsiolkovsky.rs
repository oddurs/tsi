use crate::units::{Isp, Ratio, Velocity};

use super::G0;

/// Calculate delta-v using the Tsiolkovsky rocket equation.
///
/// Δv = Isp × g₀ × ln(mass_ratio)
pub fn delta_v(isp: Isp, mass_ratio: Ratio) -> Velocity {
    Velocity::mps(isp.as_seconds() * G0 * mass_ratio.as_f64().ln())
}

/// Calculate required mass ratio to achieve a given delta-v.
///
/// Inverse of the Tsiolkovsky equation.
pub fn required_mass_ratio(dv: Velocity, isp: Isp) -> Ratio {
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
