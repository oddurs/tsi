use std::fmt;
use std::ops::{Add, Mul, Sub};

/// Time duration in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Time(f64);

impl Time {
    pub fn seconds(value: f64) -> Self {
        Time(value)
    }

    pub fn minutes(value: f64) -> Self {
        Time(value * 60.0)
    }

    pub fn as_seconds(&self) -> f64 {
        self.0
    }

    pub fn as_minutes(&self) -> f64 {
        self.0 / 60.0
    }
}

impl Add for Time {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Time(self.0 + rhs.0)
    }
}

impl Sub for Time {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Time(self.0 - rhs.0)
    }
}

impl Mul<f64> for Time {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Time(self.0 * rhs)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 60.0 {
            let mins = (self.0 / 60.0).floor();
            let secs = self.0 % 60.0;
            if secs > 0.0 {
                write!(f, "{}m {:.0}s", mins, secs)
            } else {
                write!(f, "{}m", mins)
            }
        } else {
            write!(f, "{:.1}s", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_construction_seconds() {
        let t = Time::seconds(120.0);
        assert_eq!(t.as_seconds(), 120.0);
    }

    #[test]
    fn time_construction_minutes() {
        let t1 = Time::seconds(120.0);
        let t2 = Time::minutes(2.0);
        assert_eq!(t1.as_seconds(), t2.as_seconds());
    }

    #[test]
    fn time_addition() {
        let t1 = Time::seconds(60.0);
        let t2 = Time::seconds(30.0);
        assert_eq!((t1 + t2).as_seconds(), 90.0);
    }

    #[test]
    fn time_subtraction() {
        let t1 = Time::seconds(90.0);
        let t2 = Time::seconds(30.0);
        assert_eq!((t1 - t2).as_seconds(), 60.0);
    }

    #[test]
    fn time_scalar_multiplication() {
        let t = Time::seconds(30.0);
        assert_eq!((t * 2.0).as_seconds(), 60.0);
    }

    #[test]
    fn time_display_seconds() {
        let t = Time::seconds(45.0);
        assert_eq!(format!("{}", t), "45.0s");
    }

    #[test]
    fn time_display_minutes() {
        let t = Time::seconds(162.0);
        assert_eq!(format!("{}", t), "2m 42s");
    }

    #[test]
    fn time_display_exact_minutes() {
        let t = Time::seconds(120.0);
        assert_eq!(format!("{}", t), "2m");
    }

    #[test]
    fn time_conversion() {
        let t = Time::minutes(2.5);
        assert_eq!(t.as_seconds(), 150.0);
        assert_eq!(t.as_minutes(), 2.5);
    }
}
