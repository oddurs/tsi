//! Validation tests comparing against known real-world rocket data.
//!
//! These tests ensure our physics calculations match reality by comparing
//! against published data from actual launch vehicles.
//!
//! Sources:
//! - NASA historical documents
//! - SpaceX published specifications
//! - Encyclopedia Astronautica

use approx::assert_relative_eq;
use tsi::engine::{Engine, EngineDatabase, Propellant};
use tsi::optimizer::{AnalyticalOptimizer, Constraints, Optimizer, Problem};
use tsi::physics::{burn_time, delta_v, twr, IspModel, G0};
use tsi::stage::{Rocket, Stage};
use tsi::units::{Force, Isp, Mass, Ratio, Velocity};

/// Saturn V first stage (S-IC) - 5x F-1 engines
///
/// The S-IC stage used LOX/RP-1 propellant and produced the highest
/// ideal delta-v of any first stage ever flown.
///
/// Data source: NASA Saturn V Flight Manual
#[test]
fn saturn_v_s1c_ideal_delta_v() {
    // S-IC specifications
    let propellant_mass = Mass::kg(2_149_500.0); // LOX + RP-1
    let dry_mass = Mass::kg(130_000.0); // Stage dry mass
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // Average Isp (weighted between sea level and vacuum)
    // Sea level: 263s, Vacuum: 304s, rough average: 280s
    let isp = Isp::seconds(280.0);

    let dv = delta_v(isp, mass_ratio);

    // Expected ideal delta-v: ~7,900 m/s
    // (Actual achieved was ~3,200 m/s due to gravity/drag losses and
    // not burning to depletion)
    assert!(
        dv.as_mps() > 7500.0,
        "S-IC delta-v too low: {}",
        dv.as_mps()
    );
    assert!(
        dv.as_mps() < 8500.0,
        "S-IC delta-v too high: {}",
        dv.as_mps()
    );
}

/// Saturn V second stage (S-II) - 5x J-2 engines
#[test]
fn saturn_v_s2_ideal_delta_v() {
    // S-II specifications (LOX/LH2)
    let propellant_mass = Mass::kg(443_000.0);
    let dry_mass = Mass::kg(36_000.0);
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // J-2 vacuum Isp: 421s
    let isp = Isp::seconds(421.0);

    let dv = delta_v(isp, mass_ratio);

    // Expected: ~10,200 m/s ideal
    assert!(
        dv.as_mps() > 9500.0,
        "S-II delta-v too low: {}",
        dv.as_mps()
    );
    assert!(
        dv.as_mps() < 11000.0,
        "S-II delta-v too high: {}",
        dv.as_mps()
    );
}

/// Falcon 9 Block 5 first stage - 9x Merlin-1D engines
///
/// The most frequently flown first stage in history.
/// Data source: SpaceX published specs, community analysis
#[test]
fn falcon_9_stage_1_ideal_delta_v() {
    // Falcon 9 v1.2 Block 5 first stage
    let propellant_mass = Mass::kg(411_000.0);
    let dry_mass = Mass::kg(22_200.0);
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // tsi's ascent-averaged model: Merlin-1D (282 s at sea level, 311 s in
    // vacuum) at the mean pressure a booster sees on its way up. The rocket
    // equation weights the light, high end of the burn most, so the result
    // sits nearer the vacuum figure than the sea-level one.
    let merlin = EngineDatabase::default().get("merlin-1d").unwrap().clone();
    let isp = merlin.isp_for(IspModel::AscentAveraged).as_seconds();
    let midpoint = (282.0 + 311.0) / 2.0;
    assert!(
        (midpoint..311.0).contains(&isp),
        "F9 S1 effective Isp: {isp} s"
    );

    // Isolated (no second stage on top): ~8,700 m/s ideal
    let dv = delta_v(Isp::seconds(isp), mass_ratio).as_mps();
    assert!(
        (dv / 8_700.0 - 1.0).abs() < 0.03,
        "F9 S1 isolated delta-v: {dv:.0} m/s"
    );
}

/// Falcon 9 Block 5 second stage - 1x Merlin Vacuum
#[test]
fn falcon_9_stage_2_ideal_delta_v() {
    let propellant_mass = Mass::kg(111_500.0);
    let dry_mass = Mass::kg(4_000.0);
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // Merlin Vacuum Isp: 348s
    let isp = Isp::seconds(348.0);

    let dv = delta_v(isp, mass_ratio);

    // Expected: ~11,400 m/s ideal
    assert!(
        dv.as_mps() > 10500.0,
        "F9 S2 delta-v too low: {}",
        dv.as_mps()
    );
    assert!(
        dv.as_mps() < 12500.0,
        "F9 S2 delta-v too high: {}",
        dv.as_mps()
    );
}

/// Space Shuttle Solid Rocket Booster (SRB)
///
/// Each shuttle had 2 SRBs providing about 71% of liftoff thrust.
#[test]
fn shuttle_srb_ideal_delta_v() {
    // Per SRB (2 total per launch)
    let propellant_mass = Mass::kg(500_000.0);
    let dry_mass = Mass::kg(87_500.0);
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // Average Isp: 242s (sea level average)
    let isp = Isp::seconds(242.0);

    let dv = delta_v(isp, mass_ratio);

    // Note: This is isolated SRB delta-v, not contribution to stack
    // Expected: ~4,200 m/s
    assert!(dv.as_mps() > 3800.0, "SRB delta-v too low: {}", dv.as_mps());
    assert!(
        dv.as_mps() < 4600.0,
        "SRB delta-v too high: {}",
        dv.as_mps()
    );
}

/// Starship Super Heavy booster - 33x Raptor-2 engines
///
/// The largest and most powerful rocket stage ever built.
/// Data source: SpaceX published specs (subject to updates)
#[test]
fn super_heavy_ideal_delta_v() {
    // Super Heavy approximate specs
    let propellant_mass = Mass::kg(3_400_000.0);
    let dry_mass = Mass::kg(200_000.0);
    let wet_mass = propellant_mass + dry_mass;
    let mass_ratio = wet_mass / dry_mass;

    // Raptor average Isp during ascent: ~330s
    let isp = Isp::seconds(330.0);

    let dv = delta_v(isp, mass_ratio);

    // Expected: ~9,200 m/s ideal (without Starship on top)
    assert!(
        dv.as_mps() > 8500.0,
        "Super Heavy delta-v too low: {}",
        dv.as_mps()
    );
    assert!(
        dv.as_mps() < 10000.0,
        "Super Heavy delta-v too high: {}",
        dv.as_mps()
    );
}

/// Falcon 9 stacked ideal delta-v.
///
/// Stage delta-vs only add up when each stage is computed carrying everything
/// above it: stage 1 lifts stage 2 and the payload, not just itself. Summing
/// the isolated stage values (as tsi's v0.6 test did) gives over 18 km/s,
/// about twice the real figure.
///
/// Source: SpaceX Falcon User's Guide (2021) for payload; stage masses from
/// the SpaceX Falcon 9 page and community compilations.
#[test]
fn falcon_9_stacked_delta_v() {
    let f9 = falcon_9();
    let total = f9.total_delta_v().as_mps();
    // Published estimates of F9's ideal delta-v to LEO cluster around 9.3 km/s.
    assert!(
        (total / 9_300.0 - 1.0).abs() < 0.05,
        "F9 stacked ideal delta-v: {total:.0} m/s"
    );
    // The booster carries a heavy stack, so it delivers far less than its
    // isolated ~8.7 km/s.
    let s1 = f9.stage_delta_v(0).as_mps();
    assert!((3_000.0..4_500.0).contains(&s1), "F9 S1 stacked: {s1:.0}");
}

/// TWR sanity check - F9 liftoff TWR
#[test]
fn falcon_9_liftoff_twr() {
    // 9 Merlin-1D at sea level
    let thrust = Force::newtons(9.0 * 845_000.0); // ~7.6 MN total
    let mass = Mass::kg(549_000.0); // Fully loaded first stage + second stage + payload

    let ratio = twr(thrust, mass, G0);

    // F9 liftoff TWR is approximately 1.4
    assert_relative_eq!(ratio.as_f64(), 1.38, epsilon = 0.1);
}

/// Burn time validation - Merlin-1D
#[test]
fn merlin_burn_time() {
    let propellant = Mass::kg(411_000.0);
    let thrust = Force::newtons(9.0 * 845_000.0); // 9 engines at SL
    let isp = Isp::seconds(282.0); // Sea level

    let time = burn_time(propellant, thrust, isp);

    // F9 first stage burn is approximately 162 seconds at full throttle
    // With these parameters we get ~149s, which is reasonable given
    // real flights throttle and don't burn to depletion
    assert!(
        time.as_seconds() > 140.0,
        "Burn time too short: {}",
        time.as_seconds()
    );
    assert!(
        time.as_seconds() < 180.0,
        "Burn time too long: {}",
        time.as_seconds()
    );
}

/// Theoretical validation: optimal staging with equal Isp
///
/// For stages with identical Isp and structural ratio, optimal mass
/// distribution gives equal delta-v per stage.
#[test]
fn optimal_staging_equal_dv_theory() {
    let target_dv = 9000.0; // m/s total
    let stages = 3;
    let isp = 350.0; // Same for all stages
    let _structural_ratio = 0.1;

    // For optimal staging, each stage contributes equally
    let dv_per_stage = target_dv / stages as f64;

    // Required mass ratio for each stage
    let required_ratio = (dv_per_stage / (isp * G0)).exp();

    // With 10% structural ratio, mass ratio = (1 + prop/struct) / (1)
    // where struct = prop * structural_ratio
    // This is just a sanity check that the math is consistent
    assert_relative_eq!(dv_per_stage, 3000.0, epsilon = 0.1);
    assert!(required_ratio > 1.0);
    assert!(required_ratio < 5.0); // Reasonable for 3000 m/s per stage
}

// ============================================================================
// Real vehicles as tsi models them
// ============================================================================

/// Build a stage from published propellant and dry mass, with the engines'
/// mass taken out of the dry mass to leave the structure.
fn stage(engine: &str, count: u32, propellant_kg: f64, dry_kg: f64) -> Stage {
    let engine = EngineDatabase::default().get(engine).unwrap().clone();
    let structure = dry_kg - engine.dry_mass().as_kg() * count as f64;
    Stage::new(engine, count, Mass::kg(propellant_kg), Mass::kg(structure))
}

/// Falcon 9 Block 5, expendable, with its 22.8 t LEO payload.
fn falcon_9() -> Rocket {
    Rocket::new(
        vec![
            stage("merlin-1d", 9, 411_000.0, 22_200.0),
            stage("merlin-vacuum", 1, 111_500.0, 4_000.0),
        ],
        Mass::kg(22_800.0),
    )
}

/// Saturn V (Apollo 11) with ~45 t of spacecraft to trans-lunar injection.
///
/// Source: NASA SP-4206 *Stages to Saturn*, appendix; Saturn V Flight Manual SA-503.
fn saturn_v() -> Rocket {
    Rocket::new(
        vec![
            stage("f-1", 5, 2_160_000.0, 131_000.0),
            stage("j-2", 5, 443_000.0, 36_000.0),
            stage("j-2", 1, 107_000.0, 13_500.0),
        ],
        Mass::kg(45_000.0),
    )
}

/// Ask the optimizer to design a rocket that does the real vehicle's job,
/// with the real vehicle's engines on each stage.
fn redesign(real: &Rocket, structural_ratio: f64, min_twr: (f64, f64)) -> Rocket {
    let (min_liftoff_twr, min_upper_twr) = min_twr;
    let constraints = Constraints::new(
        Ratio::new(min_liftoff_twr),
        Ratio::new(min_upper_twr),
        real.stage_count() as u32,
        Ratio::new(structural_ratio),
    );
    let mut problem = Problem::new(real.payload(), real.total_delta_v(), vec![], constraints)
        .with_stage_count(real.stage_count() as u32);
    for (i, s) in real.stages().iter().enumerate() {
        problem = problem.with_pinned_engine(i, s.engine().clone());
    }
    AnalyticalOptimizer.optimize(&problem).unwrap().rocket
}

// ============================================================================
// Optimizer validation tests
// ============================================================================

/// Given Falcon 9's engines, payload and delta-v, the optimizer designs
/// something very like Falcon 9.
///
/// F9's structural ratios are about 4.4% (S1) and 3.2% (S2); tsi uses one
/// ratio for every stage, so 4% stands in for both.
#[test]
fn optimizer_reproduces_falcon_9() {
    let real = falcon_9();
    // tsi's default TWR limits (Falcon 9 lifts off at about 1.36)
    let design = redesign(&real, 0.04, (1.2, 0.5));

    // Same engine counts: nine Merlins on the booster, one MVac above.
    assert_eq!(design.stages()[0].engine_count(), 9);
    assert_eq!(design.stages()[1].engine_count(), 1);

    // Liftoff mass within 5% of the real 571.5 t.
    let ratio = design.total_mass().as_kg() / real.total_mass().as_kg();
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "optimized/real mass = {ratio:.3}"
    );

    // Stage split within 1 km/s of the real one. The optimizer leans further
    // onto the vacuum stage than SpaceX did: ideal staging theory ignores
    // gravity losses, which punish a long, low-thrust upper-stage burn.
    for i in 0..2 {
        let (d, r) = (
            design.stage_delta_v(i).as_mps(),
            real.stage_delta_v(i).as_mps(),
        );
        assert!(
            (d - r).abs() < 1_000.0,
            "stage {}: {d:.0} vs {r:.0} m/s",
            i + 1
        );
    }
}

/// Saturn V shows where ideal staging theory stops being enough.
///
/// Asked to do Saturn V's job with its engines, the optimizer designs a
/// rocket about 30% lighter, with the first stage cut back to the
/// 2 km/s minimum. The J-2's 421 s beats the F-1's ~290 s so decisively that
/// ideal theory puts as much delta-v as possible on the hydrogen stages.
///
/// Von Braun's team knew better. A low-thrust hydrogen stage lighting early
/// spends minutes fighting gravity, which ideal delta-v doesn't count, and
/// hydrogen tanks are bulky and heavy (the real S-IVB's structural ratio is
/// 11%, against the S-IC's 4%). Capturing that needs gravity losses in the
/// optimization, which the trajectory work planned after 1.0 will bring.
#[test]
fn optimizer_finds_saturn_v_heavier_than_ideal() {
    let real = saturn_v();
    // Saturn V lifted off at about 1.18; its upper stages lit at 0.8 and 0.6
    let design = redesign(&real, 0.05, (1.15, 0.6));

    let ratio = design.total_mass().as_kg() / real.total_mass().as_kg();
    assert!(
        (0.6..0.85).contains(&ratio),
        "optimized/real mass = {ratio:.3}"
    );
    // The ideal design shrinks the kerosene first stage...
    assert!(design.stage_delta_v(0).as_mps() < real.stage_delta_v(0).as_mps());
    // ...but no further than the floor that keeps upper stages above the air.
    assert!(design.stage_delta_v(0).as_mps() >= 2_000.0 - 1e-6);
}

/// The analytical optimizer's classical core: identical stages split
/// delta-v equally when engine mass is negligible and there's no atmosphere.
#[test]
fn optimizer_equal_dv_split() {
    let feather = Engine::new(
        "Feather",
        Force::kilonewtons(2_000.0),
        Force::kilonewtons(2_000.0),
        Isp::seconds(350.0),
        Isp::seconds(350.0),
        Mass::kg(0.001),
        Propellant::LoxCh4,
    );
    let problem = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(9_000.0),
        vec![feather],
        Constraints::default()
            .with_booster_isp(IspModel::Vacuum)
            .with_max_engines(100),
    )
    .with_stage_count(2);

    let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
    let rocket = &solution.rocket;

    let stage1_dv = rocket.stage_delta_v(0).as_mps();
    let stage2_dv = rocket.stage_delta_v(1).as_mps();
    assert!(
        (stage1_dv - stage2_dv).abs() < 1.0,
        "Stage delta-v not equal: S1={stage1_dv:.1}, S2={stage2_dv:.1}"
    );
}

/// Real engines break the equal split: a booster from sea level has lower
/// effective Isp, and fixed engine mass hurts small stages most.
#[test]
fn optimizer_shifts_delta_v_to_the_upper_stage() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();

    let problem = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(9_400.0),
        vec![raptor.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let rocket = AnalyticalOptimizer.optimize(&problem).unwrap().rocket;
    assert!(rocket.stage_delta_v(1).as_mps() > rocket.stage_delta_v(0).as_mps());
}

/// Optimizer meets the target exactly with no margin, and by the margin
/// asked for when there is one.
#[test]
fn optimizer_margin_is_explicit() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();
    let target = 9_400.0;

    for margin in [0.0, 0.02, 0.05] {
        let problem = Problem::new(
            Mass::kg(5_000.0),
            Velocity::mps(target),
            vec![raptor.clone()],
            Constraints::default().with_margin(Ratio::new(margin)),
        )
        .with_stage_count(2);

        let solution = AnalyticalOptimizer.optimize(&problem).unwrap();
        let achieved = solution.rocket.total_delta_v().as_mps();
        assert!(
            (achieved - target * (1.0 + margin)).abs() < 0.01,
            "margin {margin}: achieved {achieved:.3} m/s"
        );
    }
}

/// Optimizer respects TWR constraints
#[test]
fn optimizer_respects_twr_constraints() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();

    let min_twr = 1.3;
    let constraints = Constraints::new(Ratio::new(min_twr), Ratio::new(0.7), 2, Ratio::new(0.08));

    let problem = Problem::new(
        Mass::kg(10_000.0),
        Velocity::mps(9_400.0),
        vec![raptor.clone()],
        constraints,
    )
    .with_stage_count(2);

    let optimizer = AnalyticalOptimizer;
    let solution = optimizer.optimize(&problem).unwrap();

    // First stage TWR must meet minimum
    let stage1_twr = solution.rocket.stage_twr(0).as_f64();
    assert!(
        stage1_twr >= min_twr,
        "Stage 1 TWR below minimum: {:.2} < {:.2}",
        stage1_twr,
        min_twr
    );
}

/// Optimizer produces reasonable payload fraction
///
/// For LEO missions, payload fraction is typically 2-4% for
/// expendable rockets with good engines.
#[test]
fn optimizer_reasonable_payload_fraction() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();

    let payload = 5_000.0;
    let problem = Problem::new(
        Mass::kg(payload),
        Velocity::mps(9_400.0), // LEO delta-v
        vec![raptor.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let optimizer = AnalyticalOptimizer;
    let solution = optimizer.optimize(&problem).unwrap();

    let pf = solution.rocket.payload_fraction().as_f64() * 100.0;

    // Payload fraction should be in realistic range for LEO
    assert!(
        (1.5..=5.0).contains(&pf),
        "Payload fraction unrealistic: {:.2}%",
        pf
    );
}

/// Different engines produce different optimal configurations
#[test]
fn optimizer_engine_comparison() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();
    let merlin = db.get("merlin-1d").unwrap();

    let problem_raptor = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(9_000.0),
        vec![raptor.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let problem_merlin = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(9_000.0),
        vec![merlin.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let optimizer = AnalyticalOptimizer;
    let raptor_solution = optimizer.optimize(&problem_raptor).unwrap();
    let merlin_solution = optimizer.optimize(&problem_merlin).unwrap();

    // Raptor has higher Isp (350s vs 311s), so should have better payload fraction
    let raptor_pf = raptor_solution.rocket.payload_fraction().as_f64();
    let merlin_pf = merlin_solution.rocket.payload_fraction().as_f64();

    assert!(
        raptor_pf > merlin_pf,
        "Higher Isp engine should have better payload fraction: Raptor={:.3}, Merlin={:.3}",
        raptor_pf,
        merlin_pf
    );
}
