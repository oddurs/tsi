use std::fmt;
use std::ops::{Add, Mul, Sub};

/// Velocity in meters per second.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f64);

impl Velocity {
    pub fn mps(value: f64) -> Self {
        Velocity(value)
    }

    pub fn kmps(value: f64) -> Self {
        Velocity(value * 1000.0)
    }

    pub fn as_mps(&self) -> f64 {
        self.0
    }

    pub fn as_kmps(&self) -> f64 {
        self.0 / 1000.0
    }
}

impl Add for Velocity {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Velocity(self.0 + rhs.0)
    }
}

impl Sub for Velocity {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Velocity(self.0 - rhs.0)
    }
}

impl Mul<f64> for Velocity {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Velocity(self.0 * rhs)
    }
}

impl fmt::Display for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.0} m/s", self.0)
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
        assert_eq!(format!("{}", v), "9400 m/s");
    }

    #[test]
    fn velocity_conversion() {
        let v = Velocity::kmps(9.4);
        assert_eq!(v.as_mps(), 9400.0);
        assert_eq!(v.as_kmps(), 9.4);
    }
}
