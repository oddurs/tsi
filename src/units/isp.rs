use std::fmt;
use std::ops::Mul;

/// Specific impulse in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Isp(f64);

impl Isp {
    pub fn seconds(value: f64) -> Self {
        Isp(value)
    }

    pub fn as_seconds(&self) -> f64 {
        self.0
    }
}

impl Mul<f64> for Isp {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Isp(self.0 * rhs)
    }
}

impl fmt::Display for Isp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
