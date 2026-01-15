//! Mass type for type-safe mass calculations.
//!
//! Mass is fundamental to rocket equation calculations. This module provides
//! a newtype wrapper around f64 that prevents accidental mixing of mass values
//! with other quantities.

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

use super::fmt::format_thousands_f64;
use super::Ratio;

/// Mass in kilograms - a fundamental quantity in rocket calculations.
///
/// Mass appears throughout rocket equations:
/// - **Wet mass (m₀)**: Total mass including propellant
/// - **Dry mass (m₁)**: Mass after propellant is exhausted
/// - **Propellant mass**: Wet mass minus dry mass
/// - **Structural mass**: Tanks, engines, plumbing (dry mass minus payload)
///
/// # Type Safety
///
/// Using a dedicated `Mass` type prevents common errors like accidentally
/// adding mass to velocity or mixing up units.
///
/// # Examples
///
/// ```
/// use tsi::units::Mass;
///
/// // Falcon 9 first stage masses
/// let propellant = Mass::kg(411_000.0);
/// let dry_mass = Mass::kg(22_200.0);
/// let wet_mass = propellant + dry_mass;
///
/// // Calculate mass ratio (wet/dry)
/// let ratio = wet_mass / dry_mass;
/// assert!((ratio.as_f64() - 19.5).abs() < 0.1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mass(f64);

impl Mass {
    /// Create a mass value in kilograms.
    ///
    /// Kilograms are the SI unit and the internal representation.
    pub fn kg(value: f64) -> Self {
        Mass(value)
    }

    /// Create a mass value in metric tonnes (1000 kg).
    ///
    /// Convenient for large rocket masses. A Falcon 9 has a liftoff
    /// mass of about 549 tonnes.
    pub fn tonnes(value: f64) -> Self {
        Mass(value * 1000.0)
    }

    /// Get the mass value in kilograms.
    pub fn as_kg(&self) -> f64 {
        self.0
    }

    /// Get the mass value in metric tonnes.
    pub fn as_tonnes(&self) -> f64 {
        self.0 / 1000.0
    }
}

// Mass + Mass = Mass (adding propellant to dry mass gives wet mass)
impl Add for Mass {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Mass(self.0 + rhs.0)
    }
}

// Mass - Mass = Mass (wet mass minus dry mass gives propellant mass)
impl Sub for Mass {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Mass(self.0 - rhs.0)
    }
}

// Mass * scalar = Mass (scaling mass, e.g., for multiple engines)
impl Mul<f64> for Mass {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Mass(self.0 * rhs)
    }
}

// Mass * u32 = Mass (e.g., single engine mass × engine count)
impl Mul<u32> for Mass {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self {
        Mass(self.0 * rhs as f64)
    }
}

// Mass / Mass = Ratio (the fundamental mass ratio calculation)
// This is the key operation for the rocket equation!
impl Div for Mass {
    type Output = Ratio;
    fn div(self, rhs: Self) -> Ratio {
        Ratio::new(self.0 / rhs.0)
    }
}

impl fmt::Display for Mass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use thousands separators for readability (e.g., "142,300 kg")
        if self.0 >= 1000.0 {
            write!(f, "{} kg", format_thousands_f64(self.0))
        } else {
            write!(f, "{:.1} kg", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_construction_kg() {
        let m = Mass::kg(1000.0);
        assert_eq!(m.as_kg(), 1000.0);
    }

    #[test]
    fn mass_construction_tonnes() {
        let m1 = Mass::kg(1000.0);
        let m2 = Mass::tonnes(1.0);
        assert_eq!(m1.as_kg(), m2.as_kg());
    }

    #[test]
    fn mass_addition() {
        let m1 = Mass::kg(100.0);
        let m2 = Mass::kg(50.0);
        let sum = m1 + m2;
        assert_eq!(sum.as_kg(), 150.0);
    }

    #[test]
    fn mass_subtraction() {
        let m1 = Mass::kg(100.0);
        let m2 = Mass::kg(30.0);
        let diff = m1 - m2;
        assert_eq!(diff.as_kg(), 70.0);
    }

    #[test]
    fn mass_ratio() {
        let wet = Mass::kg(100.0);
        let dry = Mass::kg(25.0);
        let ratio = wet / dry;
        assert_eq!(ratio.as_f64(), 4.0);
    }

    #[test]
    fn mass_scalar_multiplication() {
        let m = Mass::kg(100.0);
        assert_eq!((m * 3.0).as_kg(), 300.0);
    }

    #[test]
    fn mass_u32_multiplication() {
        let m = Mass::kg(100.0);
        assert_eq!((m * 3u32).as_kg(), 300.0);
    }

    #[test]
    fn mass_zero() {
        let m = Mass::kg(0.0);
        assert_eq!(m.as_kg(), 0.0);
    }

    #[test]
    fn mass_large_values() {
        let m = Mass::kg(1_000_000_000.0);
        assert_eq!(m.as_tonnes(), 1_000_000.0);
    }

    #[test]
    fn mass_display() {
        let m = Mass::kg(1500.0);
        assert_eq!(format!("{}", m), "1,500 kg");
    }

    #[test]
    fn mass_display_large() {
        let m = Mass::kg(142300.0);
        assert_eq!(format!("{}", m), "142,300 kg");
    }

    #[test]
    fn mass_display_small() {
        let m = Mass::kg(50.5);
        assert_eq!(format!("{}", m), "50.5 kg");
    }
}
