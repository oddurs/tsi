use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

use super::Ratio;

/// Mass in kilograms.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mass(f64);

impl Mass {
    pub fn kg(value: f64) -> Self {
        Mass(value)
    }

    pub fn tonnes(value: f64) -> Self {
        Mass(value * 1000.0)
    }

    pub fn as_kg(&self) -> f64 {
        self.0
    }

    pub fn as_tonnes(&self) -> f64 {
        self.0 / 1000.0
    }
}

impl Add for Mass {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Mass(self.0 + rhs.0)
    }
}

impl Sub for Mass {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Mass(self.0 - rhs.0)
    }
}

impl Mul<f64> for Mass {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Mass(self.0 * rhs)
    }
}

impl Mul<u32> for Mass {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self {
        Mass(self.0 * rhs as f64)
    }
}

impl Div for Mass {
    type Output = Ratio;
    fn div(self, rhs: Self) -> Ratio {
        Ratio::new(self.0 / rhs.0)
    }
}

impl fmt::Display for Mass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1000.0 {
            write!(f, "{:.0} kg", self.0)
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
        assert_eq!(format!("{}", m), "1500 kg");
    }

    #[test]
    fn mass_display_small() {
        let m = Mass::kg(50.5);
        assert_eq!(format!("{}", m), "50.5 kg");
    }
}
