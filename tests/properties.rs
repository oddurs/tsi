//! Property-based tests using proptest.
//!
//! These tests verify invariants that should hold for any valid input,
//! catching edge cases that example-based tests might miss.

use proptest::prelude::*;
use tsiolkovsky::engine::Propellant;
use tsiolkovsky::engine::{Engine, EngineDatabase};
use tsiolkovsky::optimizer::{
    AnalyticalOptimizer, BruteForceOptimizer, Constraints, OptimizeError, Optimizer, Problem,
    Solution, Uncertainty,
};
use tsiolkovsky::physics::{delta_v, required_mass_ratio};
use tsiolkovsky::stage::{Rocket, Stage};
use tsiolkovsky::units::Force;
use tsiolkovsky::units::{Isp, Mass, Ratio, Velocity};

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

// ============================================================================
// Optimizer invariants
// ============================================================================

/// Engines that can fly a first stage from sea level.
const BOOSTER_ENGINES: [&str; 5] = ["raptor-2", "merlin-1d", "rs-25", "be-4", "rd-180"];

fn engine(name: &str) -> Engine {
    EngineDatabase::default().get(name).unwrap().clone()
}

fn problem(engine_name: &str, payload: f64, dv: f64, eps: f64, stages: u32) -> Problem {
    let constraints = Constraints::default()
        .with_max_engines(20)
        .with_structural_ratio(eps);
    Problem::builder()
        .payload(Mass::kg(payload))
        .target(Velocity::mps(dv))
        .engine(engine(engine_name))
        .constraints(constraints)
        .stages(stages)
        .build()
        .unwrap()
}

/// Optimize, treating "no rocket can do this" as a reason to skip the case.
fn solve(problem: &Problem) -> Option<Solution> {
    match AnalyticalOptimizer.optimize(problem) {
        Ok(solution) => Some(solution),
        Err(OptimizeError::Infeasible(_)) => None,
        Err(e) => panic!("unexpected error: {e}"),
    }
}

fn mass(solution: &Solution) -> f64 {
    solution.rocket().total_mass().as_kg()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Every solution reaches its target and satisfies every TWR constraint.
    #[test]
    fn solutions_meet_target_and_twr(
        engine_name in prop::sample::select(BOOSTER_ENGINES.to_vec()),
        payload in 500.0..50_000.0_f64,
        dv in 6_000.0..11_000.0_f64,
        eps in 0.04..0.12_f64,
        stages in 2u32..=3,
    ) {
        let problem = problem(engine_name, payload, dv, eps, stages);
        let Some(solution) = solve(&problem) else { return Ok(()) };
        let rocket = &solution.rocket();
        let c = problem.constraints();

        prop_assert!(solution.meets_target(), "margin {}", solution.margin());
        prop_assert!(rocket.liftoff_twr().as_f64() >= c.min_liftoff_twr().as_f64() * (1.0 - 1e-9));
        for i in 1..rocket.stage_count() {
            prop_assert!(rocket.stage_twr(i).as_f64() >= c.min_stage_twr().as_f64() * (1.0 - 1e-9));
        }
        for stage in rocket.stages() {
            prop_assert!(stage.engine_count() <= c.max_engines_per_stage());
        }
    }

    /// A heavier payload never needs a lighter rocket.
    #[test]
    fn mass_rises_with_payload(
        engine_name in prop::sample::select(BOOSTER_ENGINES.to_vec()),
        payload in 500.0..40_000.0_f64,
        extra in 1.05..2.0_f64,
        dv in 6_000.0..10_000.0_f64,
    ) {
        let light = problem(engine_name, payload, dv, 0.08, 2);
        let heavy = problem(engine_name, payload * extra, dv, 0.08, 2);
        let (Some(a), Some(b)) = (solve(&light), solve(&heavy)) else { return Ok(()) };
        prop_assert!(mass(&b) >= mass(&a) * (1.0 - 1e-6), "{} then {}", mass(&a), mass(&b));
    }

    /// More delta-v always costs mass, so payload fraction strictly falls.
    #[test]
    fn payload_fraction_falls_as_delta_v_rises(
        engine_name in prop::sample::select(BOOSTER_ENGINES.to_vec()),
        payload in 500.0..40_000.0_f64,
        dv in 6_000.0..10_000.0_f64,
        extra in 100.0..1_000.0_f64,
    ) {
        let easy = problem(engine_name, payload, dv, 0.08, 2);
        let hard = problem(engine_name, payload, dv + extra, 0.08, 2);
        let (Some(a), Some(b)) = (solve(&easy), solve(&hard)) else { return Ok(()) };
        prop_assert!(mass(&b) > mass(&a));
        prop_assert!(
            b.rocket().payload_fraction().as_f64() < a.rocket().payload_fraction().as_f64()
        );
    }

    /// Validation catches every non-finite or non-positive input, and
    /// whatever gets past it produces a finite rocket or a clean error.
    #[test]
    fn arbitrary_inputs_never_panic_or_go_non_finite(
        payload in prop::num::f64::ANY,
        dv in prop::num::f64::ANY,
    ) {
        let built = Problem::builder()
            .payload(Mass::kg(payload))
            .target(Velocity::mps(dv))
            .engine(engine("raptor-2"))
            .stages(2)
            .build();
        let sane = payload.is_finite() && payload > 0.0 && dv.is_finite() && dv > 0.0;
        prop_assert_eq!(built.is_ok(), sane);
        if let Ok(problem) = built {
            if let Ok(solution) = AnalyticalOptimizer.optimize(&problem) {
                prop_assert!(mass(&solution).is_finite());
                prop_assert!(solution.rocket().total_delta_v().as_mps().is_finite());
            }
        }
    }

    /// Every public constructor either builds something physical or says
    /// why not. None of them panic, whatever numbers they are given.
    #[test]
    fn constructors_never_panic(
        a in prop::num::f64::ANY,
        b in prop::num::f64::ANY,
        c in prop::num::f64::ANY,
        d in prop::num::f64::ANY,
        e in prop::num::f64::ANY,
        count in 0u32..40,
    ) {
        let engine = Engine::new(
            "Fuzz",
            Force::newtons(a),
            Force::newtons(b),
            Isp::seconds(c),
            Isp::seconds(d),
            Mass::kg(e),
            Propellant::LoxCh4,
        );
        if let Ok(engine) = &engine {
            prop_assert!(engine.isp_sl().as_seconds() <= engine.isp_vac().as_seconds());
            prop_assert!(engine.dry_mass().as_kg() > 0.0);
        }
        let engine = engine.unwrap_or_else(|_| self::engine("raptor-2"));

        let stage = Stage::new(engine.clone(), count, Mass::kg(a), Mass::kg(b));
        let _ = Stage::with_structural_ratio(engine.clone(), count, Mass::kg(c), d);
        let rocket = stage.ok().map(|s| Rocket::new(vec![s], Mass::kg(e)));
        if let Some(Ok(rocket)) = rocket {
            prop_assert!(rocket.total_mass().as_kg().is_finite());
            let _ = rocket.total_delta_v();
            let _ = rocket.liftoff_twr();
        }

        let _ = Problem::builder()
            .payload(Mass::kg(a))
            .target(Velocity::mps(b))
            .engine(engine)
            .constraints(
                Constraints::default()
                    .with_min_liftoff_twr(c)
                    .with_structural_ratio(d)
                    .with_margin(e)
                    .with_surface_gravity(a),
            )
            .build();
        let _ = Uncertainty::default().with_isp_percent(a).validate();
    }
}

proptest! {
    // Brute force takes tens of milliseconds per case, so fewer of them.
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// The analytical optimizer is never beaten by exhaustive search, and the
    /// search gets close to it. They share the sizing model but none of the
    /// search logic, so agreement is evidence that both are right.
    #[test]
    fn analytical_matches_brute_force(
        engine_name in prop::sample::select(BOOSTER_ENGINES.to_vec()),
        payload in 1_000.0..30_000.0_f64,
        dv in 7_000.0..10_000.0_f64,
    ) {
        let problem = problem(engine_name, payload, dv, 0.08, 2);
        let Some(analytical) = solve(&problem) else { return Ok(()) };
        let brute = BruteForceOptimizer::default()

            .optimize(&problem)
            .expect("brute force should find what analytical found");
        let (a, b) = (mass(&analytical), mass(&brute));
        prop_assert!(a <= b * 1.01, "analytical {a:.0} kg, brute force {b:.0} kg");
        prop_assert!(b <= a * 1.05, "brute force {b:.0} kg, analytical {a:.0} kg");
    }
}
