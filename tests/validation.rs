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
use tsi::engine::EngineDatabase;
use tsi::optimizer::{AnalyticalOptimizer, Constraints, Optimizer, Problem};
use tsi::physics::{burn_time, delta_v, twr, G0};
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

    // Average Isp during ascent (282s SL, 311s vac)
    // Weighted toward sea level for first stage
    let isp = Isp::seconds(297.0);

    let dv = delta_v(isp, mass_ratio);

    // Expected: ~8,700 m/s ideal
    assert!(
        dv.as_mps() > 8000.0,
        "F9 S1 delta-v too low: {}",
        dv.as_mps()
    );
    assert!(
        dv.as_mps() < 9500.0,
        "F9 S1 delta-v too high: {}",
        dv.as_mps()
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

/// Verify Falcon 9 total delta-v capability
///
/// Combined S1 + S2 should achieve LEO with margin.
#[test]
fn falcon_9_total_delta_v_leo_capable() {
    // Stage 1 with expendable profile (more propellant used)
    let s1_prop = Mass::kg(411_000.0);
    let s1_dry = Mass::kg(22_200.0);
    let s1_ratio = (s1_prop + s1_dry) / s1_dry;
    let s1_dv = delta_v(Isp::seconds(297.0), s1_ratio);

    // Stage 2
    let s2_prop = Mass::kg(111_500.0);
    let s2_dry = Mass::kg(4_000.0);
    let s2_ratio = (s2_prop + s2_dry) / s2_dry;
    let s2_dv = delta_v(Isp::seconds(348.0), s2_ratio);

    let total = s1_dv.as_mps() + s2_dv.as_mps();

    // LEO requires ~9,400 m/s. With losses, F9 needs ~10,500 m/s ideal total.
    // F9 achieves this comfortably.
    assert!(total > 18000.0, "F9 total delta-v insufficient: {}", total);
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
// Optimizer validation tests
// ============================================================================

/// Optimizer produces equal delta-v per stage (optimal staging theory)
///
/// For identical engines and structural ratios, the optimal solution
/// splits delta-v equally between stages.
#[test]
fn optimizer_equal_dv_split() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();

    let problem = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(9_000.0),
        vec![raptor.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let optimizer = AnalyticalOptimizer;
    let solution = optimizer.optimize(&problem).unwrap();
    let rocket = &solution.rocket;

    let stage1_dv = rocket.stage_delta_v(0).as_mps();
    let stage2_dv = rocket.stage_delta_v(1).as_mps();

    // Stages should have approximately equal delta-v
    let ratio = stage1_dv / stage2_dv;
    assert!(
        ratio > 0.95 && ratio < 1.05,
        "Stage delta-v not equal: S1={:.0}, S2={:.0}",
        stage1_dv,
        stage2_dv
    );
}

/// Optimizer meets target delta-v with margin
#[test]
fn optimizer_meets_target_with_margin() {
    let db = EngineDatabase::default();
    let raptor = db.get("raptor-2").unwrap();

    let target = 9_400.0;
    let problem = Problem::new(
        Mass::kg(5_000.0),
        Velocity::mps(target),
        vec![raptor.clone()],
        Constraints::default(),
    )
    .with_stage_count(2);

    let optimizer = AnalyticalOptimizer;
    let solution = optimizer.optimize(&problem).unwrap();

    let achieved = solution.rocket.total_delta_v().as_mps();
    let margin = achieved - target;

    // Should exceed target (2% margin built in)
    assert!(
        achieved >= target,
        "Optimizer failed to meet target: {:.0} < {:.0}",
        achieved,
        target
    );
    // Margin should be reasonable (1-5%)
    let margin_percent = margin / target * 100.0;
    assert!(
        margin_percent >= 1.0 && margin_percent <= 5.0,
        "Margin outside expected range: {:.1}%",
        margin_percent
    );
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
        pf >= 1.5 && pf <= 5.0,
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
