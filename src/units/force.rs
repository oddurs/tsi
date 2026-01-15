use std::fmt;
use std::ops::{Add, Mul, Sub};

/// Force/thrust in Newtons.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Force(f64);

impl Force {
    pub fn newtons(value: f64) -> Self {
        Force(value)
    }

    pub fn kilonewtons(value: f64) -> Self {
        Force(value * 1000.0)
    }

    pub fn as_newtons(&self) -> f64 {
        self.0
    }

    pub fn as_kilonewtons(&self) -> f64 {
        self.0 / 1000.0
    }
}

impl Add for Force {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Force(self.0 + rhs.0)
    }
}

impl Sub for Force {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Force(self.0 - rhs.0)
    }
}

impl Mul<f64> for Force {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Force(self.0 * rhs)
    }
}

impl Mul<u32> for Force {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self {
        Force(self.0 * rhs as f64)
    }
}

impl fmt::Display for Force {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1_000_000.0 {
            write!(f, "{:.2} MN", self.0 / 1_000_000.0)
        } else if self.0 >= 1000.0 {
            write!(f, "{:.0} kN", self.0 / 1000.0)
        } else {
            write!(f, "{:.0} N", self.0)
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
        let f = Force::newtons(2_256_000.0);
        assert_eq!(format!("{}", f), "2.26 MN");
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
}
