use std::fmt;
use std::ops::{Div, Mul};

/// Dimensionless ratio (e.g., mass ratio, TWR).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Ratio(f64);

impl Ratio {
    pub fn new(value: f64) -> Self {
        Ratio(value)
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

impl Mul for Ratio {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Ratio(self.0 * rhs.0)
    }
}

impl Mul<f64> for Ratio {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Ratio(self.0 * rhs)
    }
}

impl Div for Ratio {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Ratio(self.0 / rhs.0)
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_construction() {
        let r = Ratio::new(3.5);
        assert_eq!(r.as_f64(), 3.5);
    }

    #[test]
    fn ratio_multiplication() {
        let r1 = Ratio::new(2.0);
        let r2 = Ratio::new(3.0);
        assert_eq!((r1 * r2).as_f64(), 6.0);
    }

    #[test]
    fn ratio_scalar_multiplication() {
        let r = Ratio::new(2.0);
        assert_eq!((r * 3.0).as_f64(), 6.0);
    }

    #[test]
    fn ratio_division() {
        let r1 = Ratio::new(6.0);
        let r2 = Ratio::new(2.0);
        assert_eq!((r1 / r2).as_f64(), 3.0);
    }

    #[test]
    fn ratio_display() {
        let r = Ratio::new(3.14159);
        assert_eq!(format!("{}", r), "3.142");
    }
}
