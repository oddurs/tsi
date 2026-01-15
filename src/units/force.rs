//! Force/thrust type for rocket engine output.
//!
//! Thrust is the force produced by a rocket engine, measured in Newtons.
//! This module provides type-safe force values with convenient constructors.

use std::fmt;
use std::ops::{Add, Mul, Sub};

use super::fmt::format_thousands_f64;

/// Force (thrust) in Newtons - what pushes the rocket.
///
/// Thrust determines a rocket's ability to accelerate and overcome gravity.
/// Combined with mass, it gives thrust-to-weight ratio (TWR).
///
/// # Scale Reference
///
/// | Engine | Thrust (vacuum) |
/// |--------|-----------------|
/// | Rutherford | 25 kN |
/// | Merlin-1D | 914 kN |
/// | Raptor-2 | 2,450 kN |
/// | RS-25 | 2,279 kN |
/// | F-1 | 7,770 kN |
/// | Full Falcon 9 (9×Merlin) | 8,226 kN |
/// | Saturn V (5×F-1) | 35,100 kN |
///
/// # Sea Level vs Vacuum
///
/// Thrust varies with atmospheric pressure:
/// - **Sea level**: Lower thrust due to back-pressure on nozzle
/// - **Vacuum**: Higher thrust, exhaust expands more completely
///
/// Example: Merlin-1D produces 845 kN at sea level, 914 kN in vacuum.
///
/// # Examples
///
/// ```
/// use tsi::units::Force;
///
/// // Single Raptor-2 engine
/// let raptor = Force::kilonewtons(2_450.0);
///
/// // 33 Raptors on Super Heavy
/// let super_heavy = raptor * 33u32;
/// assert!(super_heavy.as_newtons() > 80_000_000.0); // ~80 MN
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Force(f64);

impl Force {
    /// Create a force value in Newtons.
    ///
    /// The Newton is the SI unit of force: 1 N = 1 kg⋅m/s².
    pub fn newtons(value: f64) -> Self {
        Force(value)
    }

    /// Create a force value in kilonewtons.
    ///
    /// Most rocket engine thrust is quoted in kN. A Merlin-1D produces 914 kN.
    pub fn kilonewtons(value: f64) -> Self {
        Force(value * 1000.0)
    }

    /// Get the force in Newtons.
    pub fn as_newtons(&self) -> f64 {
        self.0
    }

    /// Get the force in kilonewtons.
    pub fn as_kilonewtons(&self) -> f64 {
        self.0 / 1000.0
    }
}

// Force + Force = Force (combining engine thrust)
impl Add for Force {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Force(self.0 + rhs.0)
    }
}

// Force - Force = Force (comparing thrust levels)
impl Sub for Force {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Force(self.0 - rhs.0)
    }
}

// Force * scalar = Force (throttling)
impl Mul<f64> for Force {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Force(self.0 * rhs)
    }
}

// Force * u32 = Force (multiple engines: single_engine_thrust × engine_count)
impl Mul<u32> for Force {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self {
        Force(self.0 * rhs as f64)
    }
}

impl fmt::Display for Force {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use appropriate unit based on magnitude:
        // - MN for very large values (≥ 10 MN, like Saturn V total thrust)
        // - kN for typical rocket engines (most common)
        // - N for small values (attitude thrusters)
        if self.0 >= 10_000_000.0 {
            write!(f, "{:.2} MN", self.0 / 1_000_000.0)
        } else if self.0 >= 1000.0 {
            write!(f, "{} kN", format_thousands_f64(self.0 / 1000.0))
        } else {
            write!(f, "{} N", format_thousands_f64(self.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_construction_newtons() {
        let f = Force::newtons(1_000_000.0);
        assert_eq!(f.as_newtons(), 1_000_000.0);
    }

    #[test]
    fn force_construction_kilonewtons() {
        let f1 = Force::newtons(1000.0);
        let f2 = Force::kilonewtons(1.0);
        assert_eq!(f1.as_newtons(), f2.as_newtons());
    }

    #[test]
    fn force_addition() {
        let f1 = Force::newtons(500_000.0);
        let f2 = Force::newtons(300_000.0);
        assert_eq!((f1 + f2).as_newtons(), 800_000.0);
    }

    #[test]
    fn force_subtraction() {
        let f1 = Force::newtons(500_000.0);
        let f2 = Force::newtons(200_000.0);
        assert_eq!((f1 - f2).as_newtons(), 300_000.0);
    }

    #[test]
    fn force_scalar_multiplication() {
        let f = Force::newtons(100_000.0);
        assert_eq!((f * 9.0).as_newtons(), 900_000.0);
    }

    #[test]
    fn force_u32_multiplication() {
        let f = Force::newtons(845_000.0);
        assert_eq!((f * 9u32).as_newtons(), 7_605_000.0);
    }

    #[test]
    fn force_display_meganewtons() {
        // Very large thrust (e.g., Saturn V F-1 cluster)
        let f = Force::newtons(35_000_000.0);
        assert_eq!(format!("{}", f), "35.00 MN");
    }

    #[test]
    fn force_display_kilonewtons() {
        let f = Force::newtons(845_000.0);
        assert_eq!(format!("{}", f), "845 kN");
    }

    #[test]
    fn force_display_newtons() {
        let f = Force::newtons(500.0);
        assert_eq!(format!("{}", f), "500 N");
    }

    #[test]
    fn force_display_large_kilonewtons() {
        // 9 × Merlin-1D = 8,226 kN (but that's > 1MN, so use smaller example)
        let f = Force::kilonewtons(2450.0);
        assert_eq!(format!("{}", f), "2,450 kN");
    }
}
