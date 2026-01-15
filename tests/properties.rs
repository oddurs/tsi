//! Property-based tests using proptest.
//!
//! These tests verify invariants that should hold for any valid input,
//! catching edge cases that example-based tests might miss.

use proptest::prelude::*;
use tsi::physics::{delta_v, required_mass_ratio};
use tsi::units::{Isp, Mass, Ratio, Velocity};

proptest! {
    /// Mass addition is commutative: a + b = b + a
    #[test]
    fn mass_addition_commutative(a in 0.0..1e9_f64, b in 0.0..1e9_f64) {
        let m1 = Mass::kg(a);
        let m2 = Mass::kg(b);
        prop_assert!((((m1 + m2).as_kg()) - ((m2 + m1).as_kg())).abs() < 1e-9);
    }

    /// Mass subtraction: (a + b) - b = a
    #[test]
    fn mass_addition_subtraction_inverse(a in 0.0..1e9_f64, b in 0.0..1e9_f64) {
        let m1 = Mass::kg(a);
        let m2 = Mass::kg(b);
        let result = (m1 + m2) - m2;
        prop_assert!((result.as_kg() - a).abs() < 1e-6);
    }

    /// Delta-v is always positive for mass_ratio > 1
    #[test]
    fn delta_v_positive_for_ratio_above_one(isp in 100.0..500.0_f64, ratio in 1.001..100.0_f64) {
        let dv = delta_v(Isp::seconds(isp), Ratio::new(ratio));
        prop_assert!(dv.as_mps() > 0.0);
    }

    /// Delta-v is zero when mass_ratio = 1
    #[test]
    fn delta_v_zero_for_ratio_one(isp in 100.0..500.0_f64) {
        let dv = delta_v(Isp::seconds(isp), Ratio::new(1.0));
        prop_assert!((dv.as_mps() - 0.0).abs() < 1e-10);
    }

    /// Delta-v increases monotonically with mass ratio (higher ratio = more delta-v)
    #[test]
    fn delta_v_monotonic_with_mass_ratio(
        isp in 100.0..500.0_f64,
        r1 in 1.001..50.0_f64,
        delta in 0.001..10.0_f64
    ) {
        let ratio1 = Ratio::new(r1);
        let ratio2 = Ratio::new(r1 + delta);
        let dv1 = delta_v(Isp::seconds(isp), ratio1);
        let dv2 = delta_v(Isp::seconds(isp), ratio2);
        prop_assert!(dv2.as_mps() > dv1.as_mps());
    }

    /// Delta-v increases monotonically with Isp (higher Isp = more delta-v)
    #[test]
    fn delta_v_monotonic_with_isp(
        ratio in 1.5..10.0_f64,
        isp1 in 100.0..400.0_f64,
        delta in 1.0..100.0_f64
    ) {
        let dv1 = delta_v(Isp::seconds(isp1), Ratio::new(ratio));
        let dv2 = delta_v(Isp::seconds(isp1 + delta), Ratio::new(ratio));
        prop_assert!(dv2.as_mps() > dv1.as_mps());
    }

    /// required_mass_ratio is the inverse of delta_v:
    /// delta_v(isp, required_mass_ratio(dv, isp)) ≈ dv
    #[test]
    fn mass_ratio_round_trip(isp in 200.0..450.0_f64, ratio in 1.5..20.0_f64) {
        let original = Ratio::new(ratio);
        let dv = delta_v(Isp::seconds(isp), original);
        let recovered = required_mass_ratio(dv, Isp::seconds(isp));
        prop_assert!((recovered.as_f64() - ratio).abs() < 0.0001);
    }

    /// Velocity conversion round-trip: m/s -> km/s -> m/s
    #[test]
    fn velocity_conversion_round_trip(v in 0.0..100000.0_f64) {
        let vel = Velocity::mps(v);
        let kms = vel.as_kmps();
        let back = Velocity::kmps(kms);
        prop_assert!((back.as_mps() - v).abs() < 1e-6);
    }

    /// Mass conversion round-trip: kg -> tonnes -> kg
    #[test]
    fn mass_conversion_round_trip(m in 0.0..1e12_f64) {
        let mass = Mass::kg(m);
        let tonnes = mass.as_tonnes();
        let back = Mass::tonnes(tonnes);
        prop_assert!((back.as_kg() - m).abs() < 1e-3);
    }

    /// Ratio multiplication: (a/b) * b ≈ a (within floating point tolerance)
    #[test]
    fn ratio_scaling(wet in 1000.0..1e9_f64, dry in 100.0..1e8_f64) {
        prop_assume!(wet > dry); // Wet mass must be greater than dry mass
        let wet_mass = Mass::kg(wet);
        let dry_mass = Mass::kg(dry);
        let ratio = wet_mass / dry_mass;
        // ratio * dry should approximately equal wet
        let recovered = ratio.as_f64() * dry;
        prop_assert!((recovered - wet).abs() / wet < 1e-10);
    }
}
