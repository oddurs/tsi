//! Velocity type for delta-v and exhaust velocity calculations.
//!
//! In astronautics, velocity changes (delta-v) are the fundamental currency
//! of mission planning. This module provides type-safe velocity values.

use std::fmt;
use std::ops::{Add, Mul, Sub};

use super::fmt::format_thousands_f64;

/// Velocity in meters per second - the output of the rocket equation.
///
/// Delta-v (change in velocity) is the fundamental measure of a rocket's
/// capability. All mission planning revolves around delta-v budgets.
///
/// # Delta-v Requirements
///
/// | Destination | Approximate Delta-v |
/// |-------------|---------------------|
/// | Low Earth Orbit | 9,400 m/s |
/// | Geostationary Transfer | 13,500 m/s |
/// | Lunar Surface | 16,000 m/s |
/// | Mars Surface | 18,000+ m/s |
///
/// Note: These include gravity and drag losses. Ideal delta-v from the
/// rocket equation is typically 10-20% less.
///
/// # Examples
///
/// ```
/// use tsi::units::Velocity;
///
/// // Falcon 9 stage 1 delta-v
/// let stage1_dv = Velocity::mps(8_500.0);
///
/// // Falcon 9 stage 2 delta-v
/// let stage2_dv = Velocity::mps(11_400.0);
///
/// // Total capability (ideal, vacuum)
/// let total = stage1_dv + stage2_dv;
/// assert!(total.as_mps() > 19_000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f64);

impl Velocity {
    /// Create a velocity value in meters per second.
    ///
    /// This is the SI unit and internal representation.
    pub fn mps(value: f64) -> Self {
        Velocity(value)
    }

    /// Create a velocity value in kilometers per second.
    ///
    /// Convenient for large values: LEO orbital velocity is about 7.8 km/s.
    pub fn kmps(value: f64) -> Self {
        Velocity(value * 1000.0)
    }

    /// Get the velocity in meters per second.
    pub fn as_mps(&self) -> f64 {
        self.0
    }

    /// Get the velocity in kilometers per second.
    pub fn as_kmps(&self) -> f64 {
        self.0 / 1000.0
    }
}

// Velocity + Velocity = Velocity (summing stage delta-vs)
impl Add for Velocity {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Velocity(self.0 + rhs.0)
    }
}

// Velocity - Velocity = Velocity (comparing or finding deficits)
impl Sub for Velocity {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Velocity(self.0 - rhs.0)
    }
}

// Velocity * scalar = Velocity (scaling)
impl Mul<f64> for Velocity {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Velocity(self.0 * rhs)
    }
}

impl fmt::Display for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Always display in m/s with thousands separators for readability
        write!(f, "{} m/s", format_thousands_f64(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_construction_mps() {
        let v = Velocity::mps(3000.0);
        assert_eq!(v.as_mps(), 3000.0);
    }

    #[test]
    fn velocity_construction_kmps() {
        let v1 = Velocity::mps(3000.0);
        let v2 = Velocity::kmps(3.0);
        assert_eq!(v1.as_mps(), v2.as_mps());
    }

    #[test]
    fn velocity_addition() {
        let v1 = Velocity::mps(1000.0);
        let v2 = Velocity::mps(500.0);
        assert_eq!((v1 + v2).as_mps(), 1500.0);
    }

    #[test]
    fn velocity_subtraction() {
        let v1 = Velocity::mps(1000.0);
        let v2 = Velocity::mps(300.0);
        assert_eq!((v1 - v2).as_mps(), 700.0);
    }

    #[test]
    fn velocity_scalar_multiplication() {
        let v = Velocity::mps(1000.0);
        assert_eq!((v * 2.0).as_mps(), 2000.0);
    }

    #[test]
    fn velocity_display() {
        let v = Velocity::mps(9400.0);
        assert_eq!(format!("{}", v), "9,400 m/s");
    }

    #[test]
    fn velocity_display_large() {
        let v = Velocity::mps(12500.0);
        assert_eq!(format!("{}", v), "12,500 m/s");
    }

    #[test]
    fn velocity_conversion() {
        let v = Velocity::kmps(9.4);
        assert_eq!(v.as_mps(), 9400.0);
        assert_eq!(v.as_kmps(), 9.4);
    }
}
