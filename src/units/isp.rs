//! Specific impulse (Isp) type for rocket engine efficiency.
//!
//! Specific impulse is the key measure of rocket engine efficiency,
//! representing how much thrust is produced per unit of propellant
//! consumed per second.

use std::fmt;
use std::ops::Mul;

/// Specific impulse (Isp) measured in seconds.
///
/// Isp is the most important engine efficiency metric. It represents the
/// exhaust velocity divided by standard gravity, giving units of seconds.
///
/// # Physical Meaning
///
/// Isp answers: "How many seconds can this engine produce 1 Newton of thrust
/// using 1 kg of propellant?" Higher is better.
///
/// Equivalently: `Isp × g₀ = exhaust velocity (m/s)`
///
/// # Typical Values
///
/// | Propellant | Isp Range | Example Engine |
/// |------------|-----------|----------------|
/// | LOX/RP-1   | 280-350s  | Merlin-1D (311s vac) |
/// | LOX/CH4    | 330-380s  | Raptor-2 (350s vac) |
/// | LOX/LH2    | 420-460s  | RS-25 (452s vac) |
/// | Solid      | 240-290s  | Shuttle SRB (268s) |
///
/// # Atmospheric Effects
///
/// Isp varies with atmospheric pressure:
/// - **Sea level**: Lower Isp due to back-pressure on exhaust
/// - **Vacuum**: Higher Isp, exhaust expands more efficiently
///
/// The difference can be significant (10-20%), which is why upper stages
/// use vacuum-optimized engines with large nozzles.
///
/// # Examples
///
/// ```
/// use tsi::units::Isp;
///
/// let merlin_sl = Isp::seconds(282.0);  // Merlin at sea level
/// let merlin_vac = Isp::seconds(311.0); // Merlin in vacuum
///
/// // Vacuum Isp is about 10% higher
/// let improvement = merlin_vac.as_seconds() / merlin_sl.as_seconds();
/// assert!((improvement - 1.10).abs() < 0.01);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Isp(f64);

impl Isp {
    /// Create an Isp value in seconds.
    ///
    /// Seconds are the standard unit for specific impulse in aerospace.
    pub fn seconds(value: f64) -> Self {
        Isp(value)
    }

    /// Get the Isp value in seconds.
    pub fn as_seconds(&self) -> f64 {
        self.0
    }
}

// Isp * scalar = Isp (for interpolation calculations)
impl Mul<f64> for Isp {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Isp(self.0 * rhs)
    }
}

impl fmt::Display for Isp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as integer seconds (e.g., "311s")
        write!(f, "{:.0}s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isp_construction() {
        let isp = Isp::seconds(311.0);
        assert_eq!(isp.as_seconds(), 311.0);
    }

    #[test]
    fn isp_scalar_multiplication() {
        let isp = Isp::seconds(300.0);
        assert_eq!((isp * 1.1).as_seconds(), 330.0);
    }

    #[test]
    fn isp_display() {
        let isp = Isp::seconds(311.0);
        assert_eq!(format!("{}", isp), "311s");
    }

    #[test]
    fn isp_comparison() {
        let isp1 = Isp::seconds(282.0); // Merlin SL
        let isp2 = Isp::seconds(311.0); // Merlin vac
        assert!(isp2 > isp1);
    }

    #[test]
    fn isp_typical_values() {
        // Verify common engine Isp values are representable
        let merlin_sl = Isp::seconds(282.0);
        let merlin_vac = Isp::seconds(311.0);
        let raptor_vac = Isp::seconds(350.0);
        let rs25_vac = Isp::seconds(452.0);

        assert!(merlin_sl.as_seconds() < merlin_vac.as_seconds());
        assert!(merlin_vac.as_seconds() < raptor_vac.as_seconds());
        assert!(raptor_vac.as_seconds() < rs25_vac.as_seconds());
    }
}
